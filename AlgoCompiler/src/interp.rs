//! Interpréteur : exécute un [`Program`] sans passer par Python.
//!
//! Sert au playground (« Exécuter » dans le navigateur, via WASM) et pourra
//! servir à la CLI. La sémantique suit le Python généré (`/` réel, tableaux
//! indicés dès 0, `et` / `ou` paresseux).
//!
//! Limites volontaires (garde-fous, surtout dans le navigateur) :
//! - 5 000 000 d'instructions max (suspicion de boucle infinie) ;
//! - profondeur d'appels max 1 000 (récursion) ;
//! - pas de vérification de types (comme le générateur).

use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::parser::{BinOp, Expr, Mode, Param, Program, Statement, Type, UnOp};

/// Nombre d'instructions exécutées avant de suspecter une boucle infinie.
const DEFAULT_MAX_STEPS: u64 = 5_000_000;
/// Profondeur d'appels maximale (récursion).
const MAX_DEPTH: usize = 1_000;
/// Taille maximale d'une chaîne construite par répétition (`"ab" * n`).
const MAX_REPEAT_CHARS: usize = 1_000_000;

/// Valeur manipulée à l'exécution.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Entier(i64),
    Reel(f64),
    Chaine(String),
    Caractere(char),
    Booleen(bool),
    Tableau(Vec<Value>),
}

impl Value {
    /// Nom du type pour les messages d'erreur (`entier`, `reel`, …).
    fn type_name(&self) -> &'static str {
        match self {
            Value::Entier(_) => "entier",
            Value::Reel(_) => "reel",
            Value::Chaine(_) => "string",
            Value::Caractere(_) => "caractere",
            Value::Booleen(_) => "booleen",
            Value::Tableau(_) => "tableau",
        }
    }

    /// Conversion numérique (entiers et réels mélangés).
    fn as_number(&self) -> Option<f64> {
        match self {
            Value::Entier(n) => Some(*n as f64),
            Value::Reel(f) => Some(*f),
            _ => None,
        }
    }

    /// Texte affiché par `afficher` (`vrai` / `faux`, `5.0`, brut…).
    fn display(&self) -> String {
        match self {
            Value::Entier(n) => n.to_string(),
            Value::Reel(f) => format_reel(*f),
            Value::Chaine(s) => s.clone(),
            Value::Caractere(c) => c.to_string(),
            Value::Booleen(true) => "vrai".to_string(),
            Value::Booleen(false) => "faux".to_string(),
            Value::Tableau(items) => {
                let inner = items
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
        }
    }
}

/// Réel avec au moins un chiffre après la virgule (`5.0`), comme le générateur.
fn format_reel(value: f64) -> String {
    let text = value.to_string();
    if text.contains('.') || text.contains('e') || text.contains("inf") || text.contains("nan") {
        text
    } else {
        format!("{text}.0")
    }
}

/// Erreur d'exécution (boucle infinie suspectée, variable inconnue, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub message: String,
}

impl RuntimeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

type RuntimeResult<T> = Result<T, RuntimeError>;

/// Variable : son type déclaré, sa valeur (absente = `out` pas encore rempli),
/// et son caractère constant.
struct Slot {
    ty: Type,
    value: Option<Value>,
    constant: bool,
}

/// Routine définie par `fonction` / `procedure`.
struct Routine {
    params: Vec<Param>,
    body: Vec<Statement>,
    is_func: bool,
}

/// Déroulement d'un bloc : suite normale ou interruption à propager.
enum Flow {
    Next,
    Break,
    Continue,
    Return(Value),
}

/// Valeur par défaut d'un type (`declarer x : entier;` sans initialisation).
fn default_value(ty: &Type) -> Value {
    match ty {
        Type::Entier | Type::EntierNaturel => Value::Entier(0),
        Type::Reel => Value::Reel(0.0),
        Type::Booleen => Value::Booleen(false),
        Type::Caractere => Value::Caractere(' '),
        Type::Chaine => Value::Chaine(String::new()),
        Type::Tableau { size, element_type } => {
            // Un paramètre sans taille (`tableau_de entier`) n'a pas de valeur
            // par défaut dimensionnée : tableau vide (ne devrait pas servir à
            // l'exécution, les paramètres étant liés à l'appel).
            let n = size.unwrap_or(0).max(0) as usize;
            let item = default_value(element_type);
            Value::Tableau(vec![item; n])
        }
        Type::Constante(inner) => default_value(inner),
    }
}

/// Exécute un programme : `inputs` fournit une ligne par `saisir`.
/// Renvoie le texte affiché (lignes séparées par `\n`).
pub fn run(program: &Program, inputs: &[String]) -> RuntimeResult<String> {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64 | 1)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    run_with_seed(program, inputs, seed)
}

/// Comme [`run`], avec graine fixée (déterminisme).
///
/// C'est aussi l'entrée à privilégier en WASM : selon l'environnement,
/// `SystemTime` peut ne pas être disponible (`run` y piégerait).
pub fn run_with_seed(program: &Program, inputs: &[String], seed: u64) -> RuntimeResult<String> {
    run_with_limits(program, inputs, seed, DEFAULT_MAX_STEPS)
}

/// Comme [`run`], avec graine et limite paramétrables (tests, déterminisme).
pub fn run_with_limits(
    program: &Program,
    inputs: &[String],
    seed: u64,
    max_steps: u64,
) -> RuntimeResult<String> {
    let mut interp = Interp {
        scopes: vec![HashMap::new()],
        routines: HashMap::new(),
        inputs,
        input_pos: 0,
        output: String::new(),
        steps: 0,
        max_steps,
        depth: 0,
        rng_state: seed | 1,
    };
    match interp.exec_block(&program.statements)? {
        Flow::Next => Ok(interp.output),
        Flow::Break => Err(RuntimeError::new("`sortie` hors d'une boucle")),
        Flow::Continue => Err(RuntimeError::new("`continue` hors d'une boucle")),
        Flow::Return(_) => Err(RuntimeError::new("`renvoie` hors d'une fonction")),
    }
}

struct Interp<'a> {
    scopes: Vec<HashMap<String, Slot>>,
    routines: HashMap<String, Routine>,
    inputs: &'a [String],
    input_pos: usize,
    output: String,
    steps: u64,
    max_steps: u64,
    depth: usize,
    rng_state: u64,
}

impl<'a> Interp<'a> {
    fn key(name: &str) -> String {
        name.to_lowercase()
    }

    /// Un pas d'exécution ; au-delà de la limite : boucle infinie suspectée.
    fn step(&mut self) -> RuntimeResult<()> {
        self.steps += 1;
        if self.steps > self.max_steps {
            return Err(RuntimeError::new(format!(
                "limite de {} instructions dépassée — boucle infinie ?",
                self.max_steps
            )));
        }
        Ok(())
    }

    /// Tirage pseudo-aléatoire (xorshift64*, sans dépendance externe).
    fn rng_next(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn lookup(&self, name: &str) -> RuntimeResult<&Slot> {
        let key = Self::key(name);
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&key))
            .ok_or_else(|| {
                RuntimeError::new(format!(
                    "variable inconnue «{name}» — pensez à `declarer {name} : …;`"
                ))
            })
    }

    fn lookup_mut(&mut self, name: &str) -> RuntimeResult<&mut Slot> {
        let key = Self::key(name);
        self.scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(&key))
            .ok_or_else(|| {
                RuntimeError::new(format!(
                    "variable inconnue «{name}» — pensez à `declarer {name} : …;`"
                ))
            })
    }

    /// Valeur d'une variable (erreur si `out` jamais rempli).
    fn read_var(&self, name: &str) -> RuntimeResult<Value> {
        let slot = self.lookup(name)?;
        slot.value
            .clone()
            .ok_or_else(|| RuntimeError::new(format!("«{name}» n'a pas encore reçu de valeur")))
    }

    fn exec_block(&mut self, statements: &[Statement]) -> RuntimeResult<Flow> {
        for stmt in statements {
            self.step()?;
            match self.exec_stmt(stmt)? {
                Flow::Next => {}
                flow => return Ok(flow),
            }
        }
        Ok(Flow::Next)
    }

    #[allow(clippy::too_many_lines)]
    fn exec_stmt(&mut self, stmt: &Statement) -> RuntimeResult<Flow> {
        match stmt {
            Statement::Afficher(expr) => {
                let value = self.eval_expr(expr)?;
                self.output.push_str(&value.display());
                self.output.push('\n');
                Ok(Flow::Next)
            }
            Statement::Declarer { name, ty, init } => {
                let value = match init {
                    Some(expr) => self.eval_expr(expr)?,
                    None => default_value(ty),
                };
                let key = Self::key(name);
                let scope = self.scopes.last_mut().expect("au moins un scope");
                if scope.contains_key(&key) {
                    return Err(RuntimeError::new(format!(
                        "«{name}» est déjà déclarée dans ce bloc"
                    )));
                }
                scope.insert(
                    key,
                    Slot {
                        ty: ty.clone(),
                        value: Some(value),
                        constant: false,
                    },
                );
                Ok(Flow::Next)
            }
            Statement::Constante { name, ty, value } => {
                let value = self.eval_expr(value)?;
                let key = Self::key(name);
                let scope = self.scopes.last_mut().expect("au moins un scope");
                if scope.contains_key(&key) {
                    return Err(RuntimeError::new(format!(
                        "«{name}» est déjà déclarée dans ce bloc"
                    )));
                }
                scope.insert(
                    key,
                    Slot {
                        ty: ty.clone(),
                        value: Some(value),
                        constant: true,
                    },
                );
                Ok(Flow::Next)
            }
            Statement::Affecter { name, value } => {
                let val = self.eval_expr(value)?;
                let slot = self.lookup_mut(name)?;
                if slot.constant {
                    return Err(RuntimeError::new(format!(
                        "on ne modifie pas la constante «{name}»"
                    )));
                }
                slot.value = Some(val);
                Ok(Flow::Next)
            }
            Statement::AffecterIndex { base, index, value } => {
                let idx = self.eval_expr(index)?;
                let val = self.eval_expr(value)?;
                let slot = self.lookup_mut(base)?;
                if slot.constant {
                    return Err(RuntimeError::new(format!(
                        "on ne modifie pas la constante «{base}»"
                    )));
                }
                let items = match slot.value.as_mut() {
                    Some(Value::Tableau(items)) => items,
                    Some(other) => {
                        return Err(RuntimeError::new(format!(
                            "«{base}» est {}, pas un tableau",
                            other.type_name()
                        )));
                    }
                    None => {
                        return Err(RuntimeError::new(format!(
                            "«{base}» n'a pas encore reçu de valeur"
                        )));
                    }
                };
                let i = Self::check_index(&idx, items.len())?;
                items[i] = val;
                Ok(Flow::Next)
            }
            Statement::Si {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.eval_bool(condition)? {
                    self.exec_block(then_branch)
                } else if let Some(else_branch) = else_branch {
                    self.exec_block(else_branch)
                } else {
                    Ok(Flow::Next)
                }
            }
            Statement::ChoixSur {
                expr,
                cases,
                default,
            } => {
                let scrutinee = self.eval_expr(expr)?;
                for (value, body) in cases {
                    let case = self.eval_expr(value)?;
                    if values_equal(&scrutinee, &case) {
                        return self.exec_block(body);
                    }
                }
                if let Some(default) = default {
                    return self.exec_block(default);
                }
                Ok(Flow::Next)
            }
            Statement::Boucle(body) => loop {
                self.step()?;
                match self.exec_block(body)? {
                    Flow::Next | Flow::Continue => {}
                    Flow::Break => break Ok(Flow::Next),
                    Flow::Return(v) => break Ok(Flow::Return(v)),
                }
            },
            Statement::Repeter { body, condition } => loop {
                self.step()?;
                match self.exec_block(body)? {
                    Flow::Next | Flow::Continue => {}
                    Flow::Break => break Ok(Flow::Next),
                    Flow::Return(v) => break Ok(Flow::Return(v)),
                }
                if self.eval_bool(condition)? {
                    break Ok(Flow::Next);
                }
            },
            Statement::Jusqua { condition, body } => {
                while !self.eval_bool(condition)? {
                    self.step()?;
                    match self.exec_block(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::Next)
            }
            Statement::TantQue { condition, body } => {
                while self.eval_bool(condition)? {
                    self.step()?;
                    match self.exec_block(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::Next)
            }
            Statement::Pour {
                var,
                start,
                end,
                descending,
                body,
            } => {
                let first = self.eval_expr(start)?;
                let last = self.eval_expr(end)?;
                let (Value::Entier(first), Value::Entier(last)) = (first, last) else {
                    return Err(RuntimeError::new(
                        "les bornes d'un `pour` doivent être entières",
                    ));
                };
                // Le compteur est créé au besoin (papier : on le déclare
                // rarement avant la boucle).
                let key = Self::key(var);
                if self.lookup(var).is_err() {
                    self.scopes.last_mut().expect("au moins un scope").insert(
                        key,
                        Slot {
                            ty: Type::Entier,
                            value: Some(Value::Entier(first)),
                            constant: false,
                        },
                    );
                }
                if *descending {
                    let mut i = first;
                    while i >= last {
                        self.assign_counter(var, i)?;
                        self.step()?;
                        match self.exec_block(body)? {
                            Flow::Next | Flow::Continue => {}
                            Flow::Break => break,
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                        }
                        i -= 1;
                    }
                } else {
                    let mut i = first;
                    while i <= last {
                        self.assign_counter(var, i)?;
                        self.step()?;
                        match self.exec_block(body)? {
                            Flow::Next | Flow::Continue => {}
                            Flow::Break => break,
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                        }
                        i += 1;
                    }
                }
                Ok(Flow::Next)
            }
            Statement::Sortie => Ok(Flow::Break),
            Statement::Continue => Ok(Flow::Continue),
            Statement::Fonction {
                name, params, body, ..
            } => {
                self.define_routine(name, params, body.clone(), true)?;
                Ok(Flow::Next)
            }
            Statement::Procedure { name, params, body } => {
                self.define_routine(name, params, body.clone(), false)?;
                Ok(Flow::Next)
            }
            Statement::Renvoie(expr) => {
                let value = self.eval_expr(expr)?;
                Ok(Flow::Return(value))
            }
            Statement::Saisir(vars) => {
                for var in vars {
                    self.exec_saisir(var)?;
                }
                Ok(Flow::Next)
            }
            Statement::LigneSuivante => {
                self.output.push('\n');
                Ok(Flow::Next)
            }
            Statement::Appel { name, args } => {
                self.call_statement(name, args)?;
                Ok(Flow::Next)
            }
            Statement::Algorithme { body, .. } => self.exec_block(body),
        }
    }

    /// Assigne le compteur d'un `pour` (variable existante, non constante).
    fn assign_counter(&mut self, var: &str, i: i64) -> RuntimeResult<()> {
        let slot = self.lookup_mut(var)?;
        if slot.constant {
            return Err(RuntimeError::new(format!(
                "on ne modifie pas la constante «{var}»"
            )));
        }
        slot.value = Some(Value::Entier(i));
        Ok(())
    }

    /// Enregistre une routine (erreur en cas de redéfinition).
    fn define_routine(
        &mut self,
        name: &str,
        params: &[Param],
        body: Vec<Statement>,
        is_func: bool,
    ) -> RuntimeResult<()> {
        let key = Self::key(name);
        if self.routines.contains_key(&key) {
            return Err(RuntimeError::new(format!("«{name}» est déjà définie")));
        }
        self.routines.insert(
            key,
            Routine {
                params: params.to_vec(),
                body,
                is_func,
            },
        );
        Ok(())
    }

    /// Indice 0-based vérifié (négatif ou hors tableau → erreur).
    fn check_index(index: &Value, len: usize) -> RuntimeResult<usize> {
        match index {
            Value::Entier(i) if *i >= 0 && (*i as usize) < len => Ok(*i as usize),
            Value::Entier(i) => Err(RuntimeError::new(format!(
                "indice {i} hors bornes (0 à {})",
                len.saturating_sub(1)
            ))),
            other => Err(RuntimeError::new(format!(
                "un indice doit être entier, pas {}",
                other.type_name()
            ))),
        }
    }

    /// Condition de `si` / `tant_que`… : un booléen strict.
    fn eval_bool(&mut self, expr: &Expr) -> RuntimeResult<bool> {
        match self.eval_expr(expr)? {
            Value::Booleen(b) => Ok(b),
            other => Err(RuntimeError::new(format!(
                "une condition doit être un booleen, pas {}",
                other.type_name()
            ))),
        }
    }

    /// `saisir (var);` : consomme une ligne d'entrée, convertie selon le type.
    fn exec_saisir(&mut self, var: &str) -> RuntimeResult<()> {
        let raw = self
            .inputs
            .get(self.input_pos)
            .ok_or_else(|| {
                RuntimeError::new(format!(
                    "saisir : aucune entrée fournie pour «{var}» — ajoutez une ligne dans les entrées"
                ))
            })
            .cloned()?;
        self.input_pos += 1;
        let slot = self.lookup_mut(var)?;
        if slot.constant {
            return Err(RuntimeError::new(format!(
                "on ne modifie pas la constante «{var}»"
            )));
        }
        let ty = match &slot.ty {
            Type::Constante(inner) => inner.as_ref().clone(),
            ty => ty.clone(),
        };
        let value = match ty {
            Type::Entier | Type::EntierNaturel => {
                raw.trim().parse::<i64>().map(Value::Entier).map_err(|_| {
                    RuntimeError::new(format!("«{}» n'est pas un entier", raw.trim()))
                })?
            }
            Type::Reel => raw
                .trim()
                .parse::<f64>()
                .map(Value::Reel)
                .map_err(|_| RuntimeError::new(format!("«{}» n'est pas un reel", raw.trim())))?,
            Type::Booleen => match raw.trim().to_lowercase().as_str() {
                "vrai" => Value::Booleen(true),
                "faux" => Value::Booleen(false),
                _ => {
                    return Err(RuntimeError::new(format!(
                        "«{}» n'est ni `vrai` ni `faux`",
                        raw.trim()
                    )));
                }
            },
            Type::Caractere => {
                let mut chars = raw.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => Value::Caractere(c),
                    _ => {
                        return Err(RuntimeError::new(format!(
                            "«{raw}» n'est pas un seul caractère"
                        )));
                    }
                }
            }
            Type::Chaine => Value::Chaine(raw),
            Type::Tableau { .. } => {
                return Err(RuntimeError::new(
                    "on ne peut pas saisir un tableau entier — saisissez ses cases une par une (`saisir (t[i]);` est interdit : utilisez une variable)",
                ));
            }
            Type::Constante(_) => unreachable!("résolu ci-dessus"),
        };
        slot.value = Some(value);
        Ok(())
    }

    /// `nom(args);` en instruction : appel de procédure.
    fn call_statement(&mut self, name: &str, args: &[Expr]) -> RuntimeResult<()> {
        let is_func = self
            .routines
            .get(&Self::key(name))
            .ok_or_else(|| RuntimeError::new(format!("procedure inconnue «{name}»")))?
            .is_func;
        if is_func {
            return Err(RuntimeError::new(format!(
                "«{name}» est une fonction : récupérez sa valeur (`x <- {name}(…);`)"
            )));
        }
        match self.call_routine(name, args)? {
            None => Ok(()),
            Some(_) => Err(RuntimeError::new(format!(
                "`renvoie` interdit dans une procedure («{name}»)"
            ))),
        }
    }

    /// Exécute une routine : `Ok(None)` = fin normale, `Ok(Some(v))` = `renvoie`.
    /// Les paramètres `out` / `in_out` sont réécrits chez l'appelant.
    fn call_routine(&mut self, name: &str, args: &[Expr]) -> RuntimeResult<Option<Value>> {
        // Clonée d'emblée : on ne peut pas garder d'emprunt sur `routines`
        // pendant l'évaluation des arguments et du corps.
        let (params, body) = match self.routines.get(&Self::key(name)) {
            Some(routine) => (routine.params.clone(), routine.body.clone()),
            None => return Err(RuntimeError::new(format!("routine inconnue «{name}»"))),
        };
        if args.len() != params.len() {
            return Err(RuntimeError::new(format!(
                "«{name}» attend {} argument(s), reçu {}",
                params.len(),
                args.len()
            )));
        }
        if self.depth >= MAX_DEPTH {
            return Err(RuntimeError::new(format!(
                "récursion trop profonde dans «{name}» (limite {MAX_DEPTH})"
            )));
        }
        // Évalue les arguments avant d'empiler le scope.
        enum Bound {
            Value(Value),
            Empty,
        }
        let mut bound: Vec<(String, Type, Bound)> = Vec::with_capacity(args.len());
        // (variable appelante, nom du paramètre, mode) pour réécriture.
        let mut links: Vec<(String, String, Mode)> = Vec::new();
        for (param, arg) in params.iter().zip(args.iter()) {
            match param.mode {
                Mode::In => bound.push((
                    param.name.clone(),
                    param.ty.clone(),
                    Bound::Value(self.eval_expr(arg)?),
                )),
                Mode::Out | Mode::InOut => {
                    let var = match arg {
                        Expr::Ident(var) => var.clone(),
                        _ => {
                            return Err(RuntimeError::new(format!(
                                "le paramètre «{}» ({}) attend une variable",
                                param.name,
                                if param.mode == Mode::Out {
                                    "out"
                                } else {
                                    "in_out"
                                }
                            )));
                        }
                    };
                    // La variable doit exister et être modifiable.
                    let slot = self.lookup(&var)?;
                    if slot.constant {
                        return Err(RuntimeError::new(format!(
                            "on ne modifie pas la constante «{var}»"
                        )));
                    }
                    let initial = match param.mode {
                        Mode::InOut => Bound::Value(self.read_var(&var)?),
                        _ => Bound::Empty,
                    };
                    bound.push((param.name.clone(), param.ty.clone(), initial));
                    links.push((var, param.name.clone(), param.mode));
                }
            }
        }
        self.depth += 1;
        self.scopes.push(HashMap::new());
        for (pname, ty, b) in bound {
            let value = match b {
                Bound::Value(v) => Some(v),
                Bound::Empty => None,
            };
            self.scopes.last_mut().expect("scope empilé").insert(
                Self::key(&pname),
                Slot {
                    ty,
                    value,
                    constant: false,
                },
            );
        }
        let flow = self.exec_block(&body);
        // Relit les valeurs finales AVANT de dépiler le scope de la routine.
        // (En cas d'erreur on abandonne le run : pas besoin de dépiler.)
        let flow = flow?;
        let mut finals: Vec<(String, Value)> = Vec::with_capacity(links.len());
        for (caller, pname, mode) in &links {
            let slot = self
                .scopes
                .last()
                .expect("scope routine")
                .get(&Self::key(pname));
            match slot.and_then(|s| s.value.clone()) {
                Some(v) => finals.push((caller.clone(), v)),
                None => {
                    debug_assert_eq!(*mode, Mode::Out);
                    return Err(RuntimeError::new(format!(
                        "le paramètre «{pname}» (out) n'a jamais reçu de valeur dans «{name}»"
                    )));
                }
            }
        }
        self.scopes.pop();
        self.depth -= 1;
        // Réécrit les `out` / `in_out` chez l'appelant.
        for (caller, value) in finals {
            self.lookup_mut(&caller)?.value = Some(value);
        }
        match flow {
            Flow::Next | Flow::Break | Flow::Continue => {
                // `sortie` / `continue` traversent l'appel (remontent à la boucle).
                Ok(None)
            }
            Flow::Return(v) => Ok(Some(v)),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> RuntimeResult<Value> {
        match expr {
            Expr::Entier(n) => Ok(Value::Entier(*n)),
            Expr::Reel(f) => Ok(Value::Reel(*f)),
            Expr::Chaine(s) => Ok(Value::Chaine(s.clone())),
            Expr::Caractere(c) => Ok(Value::Caractere(*c)),
            Expr::Booleen(b) => Ok(Value::Booleen(*b)),
            Expr::Ident(name) => self.read_var(name),
            Expr::Index { base, index } => {
                let container = self.eval_expr(base)?;
                let idx = self.eval_expr(index)?;
                match &container {
                    Value::Tableau(items) => {
                        let i = Self::check_index(&idx, items.len())?;
                        Ok(items[i].clone())
                    }
                    Value::Chaine(s) => {
                        let chars: Vec<char> = s.chars().collect();
                        let i = Self::check_index(&idx, chars.len())?;
                        Ok(Value::Caractere(chars[i]))
                    }
                    other => Err(RuntimeError::new(format!(
                        "on ne peut pas indicer {}",
                        other.type_name()
                    ))),
                }
            }
            Expr::UnOp { op, expr } => {
                let value = self.eval_expr(expr)?;
                match op {
                    UnOp::Neg => match value {
                        Value::Entier(n) => n.checked_neg().map(Value::Entier).ok_or_else(|| {
                            RuntimeError::new("dépassement de capacité des entiers")
                        }),
                        Value::Reel(f) => Ok(Value::Reel(-f)),
                        other => Err(RuntimeError::new(format!(
                            "on ne peut pas nier {}",
                            other.type_name()
                        ))),
                    },
                    UnOp::Not => match value {
                        Value::Booleen(b) => Ok(Value::Booleen(!b)),
                        other => Err(RuntimeError::new(format!(
                            "`non` attend un booleen, pas {}",
                            other.type_name()
                        ))),
                    },
                }
            }
            Expr::BinOp { op, left, right } => self.eval_binop(*op, left, right),
            Expr::Appel { name, args } => self.eval_call(name, args),
        }
    }

    fn eval_binop(&mut self, op: BinOp, left: &Expr, right: &Expr) -> RuntimeResult<Value> {
        // `et` / `ou` paresseux (court-circuit).
        if op == BinOp::And {
            let l = self.eval_bool(left)?;
            if !l {
                return Ok(Value::Booleen(false));
            }
            return Ok(Value::Booleen(self.eval_bool(right)?));
        }
        if op == BinOp::Or {
            let l = self.eval_bool(left)?;
            if l {
                return Ok(Value::Booleen(true));
            }
            return Ok(Value::Booleen(self.eval_bool(right)?));
        }
        let l = self.eval_expr(left)?;
        let r = self.eval_expr(right)?;
        match op {
            BinOp::Add => add_values(&l, &r),
            BinOp::Sub => sub_values(&l, &r),
            BinOp::Mul => mul_values(&l, &r),
            BinOp::Div => div_values(&l, &r),
            BinOp::Eq => Ok(Value::Booleen(values_equal(&l, &r))),
            BinOp::Ne => Ok(Value::Booleen(!values_equal(&l, &r))),
            BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => cmp_values(op, &l, &r),
            BinOp::And | BinOp::Or => unreachable!("traités ci-dessus"),
        }
    }

    /// Appel en position d'expression : builtin ou fonction.
    fn eval_call(&mut self, name: &str, args: &[Expr]) -> RuntimeResult<Value> {
        if name.eq_ignore_ascii_case("taille") {
            if args.len() != 1 {
                return Err(RuntimeError::new("`taille` attend 1 argument"));
            }
            return match self.eval_expr(&args[0])? {
                Value::Tableau(items) => Ok(Value::Entier(items.len() as i64)),
                Value::Chaine(s) => Ok(Value::Entier(s.chars().count() as i64)),
                Value::Caractere(_) => Ok(Value::Entier(1)),
                other => Err(RuntimeError::new(format!(
                    "`taille` ne s'applique pas à {}",
                    other.type_name()
                ))),
            };
        }
        if name.eq_ignore_ascii_case("modulo") {
            if args.len() != 2 {
                return Err(RuntimeError::new("`modulo` attend 2 arguments"));
            }
            let a = self.eval_expr(&args[0])?;
            let b = self.eval_expr(&args[1])?;
            return match (a, b) {
                (Value::Entier(x), Value::Entier(y)) => {
                    if y == 0 {
                        return Err(RuntimeError::new("`modulo` par zéro"));
                    }
                    x.checked_rem(y)
                        .map(Value::Entier)
                        .ok_or_else(|| RuntimeError::new("dépassement de capacité des entiers"))
                }
                (l, r) => Err(RuntimeError::new(format!(
                    "`modulo` attend deux entiers, pas {} et {}",
                    l.type_name(),
                    r.type_name()
                ))),
            };
        }
        if name.eq_ignore_ascii_case("rand") {
            if args.len() != 2 {
                return Err(RuntimeError::new("`rand` attend 2 arguments"));
            }
            let a = self.eval_expr(&args[0])?;
            let b = self.eval_expr(&args[1])?;
            return match (a, b) {
                (Value::Entier(lo), Value::Entier(hi)) => {
                    if lo > hi {
                        return Err(RuntimeError::new(format!(
                            "`rand` : le minimum ({lo}) dépasse le maximum ({hi})"
                        )));
                    }
                    let span = (hi as u64).wrapping_sub(lo as u64).wrapping_add(1);
                    let draw = if span == 0 {
                        self.rng_next()
                    } else {
                        self.rng_next() % span
                    };
                    Ok(Value::Entier((lo as i128 + draw as i128) as i64))
                }
                (l, r) => Err(RuntimeError::new(format!(
                    "`rand` attend deux entiers, pas {} et {}",
                    l.type_name(),
                    r.type_name()
                ))),
            };
        }
        let is_func = self
            .routines
            .get(&Self::key(name))
            .ok_or_else(|| RuntimeError::new(format!("fonction inconnue «{name}»")))?
            .is_func;
        if !is_func {
            return Err(RuntimeError::new(format!(
                "«{name}» est une procedure : appelez-la comme instruction (`{name}(…);`)"
            )));
        }
        match self.call_routine(name, args)? {
            Some(v) => Ok(v),
            None => Err(RuntimeError::new(format!(
                "la fonction «{name}» se termine sans `renvoie`"
            ))),
        }
    }
}

/// Égalité `vaut` : nombres comparés numériquement, sinon égalité stricte
/// (types différents → faux, pas d'erreur).
fn values_equal(l: &Value, r: &Value) -> bool {
    match (l.as_number(), r.as_number()) {
        (Some(a), Some(b)) => a == b,
        _ => l == r,
    }
}

fn add_values(l: &Value, r: &Value) -> RuntimeResult<Value> {
    match (l, r) {
        (Value::Entier(a), Value::Entier(b)) => a
            .checked_add(*b)
            .map(Value::Entier)
            .ok_or_else(|| RuntimeError::new("dépassement de capacité des entiers")),
        _ if l.as_number().is_some() && r.as_number().is_some() => Ok(Value::Reel(
            l.as_number().unwrap_or(0.0) + r.as_number().unwrap_or(0.0),
        )),
        _ if matches!(l, Value::Chaine(_) | Value::Caractere(_))
            && matches!(r, Value::Chaine(_) | Value::Caractere(_)) =>
        {
            Ok(Value::Chaine(l.display() + &r.display()))
        }
        _ => Err(RuntimeError::new(format!(
            "on ne peut pas additionner {} et {}",
            l.type_name(),
            r.type_name()
        ))),
    }
}

fn sub_values(l: &Value, r: &Value) -> RuntimeResult<Value> {
    match (l, r) {
        (Value::Entier(a), Value::Entier(b)) => a
            .checked_sub(*b)
            .map(Value::Entier)
            .ok_or_else(|| RuntimeError::new("dépassement de capacité des entiers")),
        _ if l.as_number().is_some() && r.as_number().is_some() => Ok(Value::Reel(
            l.as_number().unwrap_or(0.0) - r.as_number().unwrap_or(0.0),
        )),
        _ => Err(RuntimeError::new(format!(
            "on ne peut pas soustraire {} et {}",
            l.type_name(),
            r.type_name()
        ))),
    }
}

fn mul_values(l: &Value, r: &Value) -> RuntimeResult<Value> {
    // `"ab" * 3` → `"ababab"` (répétition, comme en Python).
    if let (Value::Chaine(s), Value::Entier(n)) | (Value::Entier(n), Value::Chaine(s)) = (l, r) {
        if *n < 0 {
            return Err(RuntimeError::new(
                "on ne peut pas répéter un nombre négatif de fois",
            ));
        }
        let count = *n as usize;
        if s.chars().count().saturating_mul(count) > MAX_REPEAT_CHARS {
            return Err(RuntimeError::new("répétition trop grande"));
        }
        return Ok(Value::Chaine(s.repeat(count)));
    }
    match (l, r) {
        (Value::Entier(a), Value::Entier(b)) => a
            .checked_mul(*b)
            .map(Value::Entier)
            .ok_or_else(|| RuntimeError::new("dépassement de capacité des entiers")),
        _ if l.as_number().is_some() && r.as_number().is_some() => Ok(Value::Reel(
            l.as_number().unwrap_or(0.0) * r.as_number().unwrap_or(0.0),
        )),
        _ => Err(RuntimeError::new(format!(
            "on ne peut pas multiplier {} et {}",
            l.type_name(),
            r.type_name()
        ))),
    }
}

/// `/` donne toujours un réel (comme le Python généré).
fn div_values(l: &Value, r: &Value) -> RuntimeResult<Value> {
    match (l.as_number(), r.as_number()) {
        (Some(_), Some(0.0)) => Err(RuntimeError::new("division par zéro")),
        (Some(num), Some(den)) => Ok(Value::Reel(num / den)),
        _ => Err(RuntimeError::new(format!(
            "on ne peut pas diviser {} par {}",
            l.type_name(),
            r.type_name()
        ))),
    }
}

fn cmp_values(op: BinOp, l: &Value, r: &Value) -> RuntimeResult<Value> {
    let ordering = match (l, r) {
        _ if l.as_number().is_some() && r.as_number().is_some() => l
            .as_number()
            .unwrap_or(0.0)
            .partial_cmp(&r.as_number().unwrap_or(0.0)),
        (Value::Chaine(_), Value::Chaine(_)) => {
            let (x, y) = (l.display(), r.display());
            Some(x.cmp(&y))
        }
        (Value::Caractere(_), Value::Caractere(_)) => {
            let (x, y) = (l.display(), r.display());
            Some(x.cmp(&y))
        }
        (Value::Chaine(_), Value::Caractere(_)) | (Value::Caractere(_), Value::Chaine(_)) => {
            Some(l.display().cmp(&r.display()))
        }
        _ => None,
    };
    let ordering = ordering.ok_or_else(|| {
        RuntimeError::new(format!(
            "on ne peut pas comparer {} et {}",
            l.type_name(),
            r.type_name()
        ))
    })?;
    let result = match op {
        BinOp::Lt => ordering.is_lt(),
        BinOp::Gt => ordering.is_gt(),
        BinOp::Le => !ordering.is_gt(),
        BinOp::Ge => !ordering.is_lt(),
        _ => unreachable!("comparaison"),
    };
    Ok(Value::Booleen(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    /// Exécute un source avec des entrées (graine fixe, 100 000 pas max).
    fn run_src(source: &str, inputs: &[&str]) -> RuntimeResult<String> {
        let tokens = tokenize(source).expect("lexing valide");
        let program = parse(&tokens).expect("parse valide");
        let owned: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();
        run_with_limits(&program, &owned, 42, 100_000)
    }

    fn run_ok(source: &str, inputs: &[&str]) -> String {
        run_src(source, inputs).expect("exécution valide")
    }

    fn run_err(source: &str, inputs: &[&str]) -> String {
        run_src(source, inputs)
            .expect_err("exécution en erreur")
            .message
    }

    #[test]
    fn afficher_et_affectation() {
        assert_eq!(run_ok("afficher(42);", &[]), "42\n");
        assert_eq!(
            run_ok("declarer x : entier; x <- 1 + 2 * 3; afficher(x);", &[]),
            "7\n"
        );
        assert_eq!(
            run_ok("declarer x : reel; x <- 7 / 2; afficher(x);", &[]),
            "3.5\n"
        );
        assert_eq!(
            run_ok("declarer x : booleen; x <- vrai; afficher(x);", &[]),
            "vrai\n"
        );
        assert_eq!(
            run_ok("declarer x : reel; x <- 5.0; afficher(x);", &[]),
            "5.0\n"
        );
        assert_eq!(
            run_ok("declarer x : string; x <- \"a\" + \"b\"; afficher(x);", &[]),
            "ab\n"
        );
        // Affecter sans déclarer : erreur claire (le `pour` reste tolérant).
        assert!(run_err("x <- 1;", &[]).contains("inconnue"));
    }

    #[test]
    fn exemple_saisir_pour() {
        // Exemple du playground : saisir + `pour … à` accentué.
        let source = "declarer input : string;\nafficher (\"mot ?\");\nsaisir(input);\npour (i variant_de 0 à 2) faire\nafficher (input);\nffaire\n";
        assert_eq!(
            run_ok(source, &["bonjour"]),
            "mot ?\nbonjour\nbonjour\nbonjour\n"
        );
    }

    #[test]
    fn saisir_conversions() {
        assert_eq!(
            run_ok("declarer n : entier; saisir(n); afficher(n);", &["14"]),
            "14\n"
        );
        assert_eq!(
            run_ok("declarer r : reel; saisir(r); afficher(r);", &["2.5"]),
            "2.5\n"
        );
        assert_eq!(
            run_ok("declarer b : booleen; saisir(b); afficher(b);", &["VRAI"]),
            "vrai\n"
        );
        assert!(run_err("declarer n : entier; saisir(n);", &["abc"]).contains("pas un entier"));
        assert!(run_err("declarer n : entier; saisir(n);", &[]).contains("aucune entrée"));
    }

    #[test]
    fn boucles() {
        assert_eq!(
            run_ok("pour (i variant_de 1 a 3) faire afficher(i); ffaire", &[]),
            "1\n2\n3\n"
        );
        assert_eq!(
            run_ok(
                "pour (i variant_de 3 a 1 descendant) faire afficher(i); ffaire",
                &[]
            ),
            "3\n2\n1\n"
        );
        // `tant_que` faux d'emblée : zéro tour.
        assert_eq!(
            run_ok(
                "tant_que (faux) faire afficher(1); ffaire afficher(0);",
                &[]
            ),
            "0\n"
        );
        // `repeter` : au moins un tour.
        assert_eq!(run_ok("repeter afficher(9); jusqua (vrai);", &[]), "9\n");
        // Garde courte `si … sortie;` sans `fsi` (retour à la ligne requis).
        assert_eq!(
            run_ok(
                "declarer k : entier <- 0;\nboucle\nsi (k vaut 2) sortie;\nk <- k + 1;\nfboucle\nafficher(k);",
                &[]
            ),
            "2\n"
        );
        // `continue` saute les impairs.
        assert_eq!(
            run_ok(
                "pour (i variant_de 1 a 4) faire si (modulo (i, 2) vaut 1) continue; fsi afficher(i); ffaire",
                &[]
            ),
            "2\n4\n"
        );
    }

    #[test]
    fn boucle_infinie_detectee() {
        let err = run_err("boucle afficher(1); fboucle", &[]);
        assert!(err.contains("boucle infinie"), "{err}");
    }

    #[test]
    fn fonctions_et_procedures() {
        let source = "fonction double(x : in entier) renvoie entier debut renvoie x * 2; fin\ndeclarer y : entier <- 21; y <- double (y); afficher (y);\n";
        assert_eq!(run_ok(source, &[]), "42\n");
        // `in_out` : la variable appelante est modifiée.
        let source = "procedure incrementer(c : in_out entier) debut c <- c + 1; fin\ndeclarer n : entier <- 5; incrementer (n); afficher (n);\n";
        assert_eq!(run_ok(source, &[]), "6\n");
        // `out` : créé dans la procédure, récupéré à l'appel.
        let source = "procedure lire(n : out entier) debut n <- 7; fin\ndeclarer m : entier; lire (m); afficher (m);\n";
        assert_eq!(run_ok(source, &[]), "7\n");
        // `out` jamais rempli → erreur claire.
        let source = "procedure oublie(n : out entier) debut afficher(1); fin\ndeclarer m : entier; oublie (m);\n";
        assert!(run_err(source, &[]).contains("jamais reçu de valeur"));
        // Mauvais usage : fonction en instruction, procédure en expression.
        assert!(
            run_err(
                "fonction f() renvoie entier debut renvoie 1; fin\nf();",
                &[]
            )
            .contains("est une fonction")
        );
        assert!(
            run_err("procedure p() debut afficher(1); fin\nx <- p();", &[])
                .contains("est une procedure")
        );
        // Arity.
        assert!(
            run_err(
                "fonction f(x : in entier) renvoie entier debut renvoie x; fin\nafficher(f(1, 2));",
                &[]
            )
            .contains("attend 1 argument")
        );
        // Fonction sans renvoie.
        assert!(
            run_err(
                "fonction f() renvoie entier debut afficher(1); fin\nafficher(f());",
                &[]
            )
            .contains("sans `renvoie`")
        );
    }

    #[test]
    fn erreurs_runtime_claires() {
        assert!(run_err("afficher(x);", &[]).contains("inconnue"));
        assert!(run_err("x <- 1 / 0; afficher(x);", &[]).contains("zéro"));
        assert!(run_err("x <- modulo (1, 0);", &[]).contains("zéro"));
        assert!(
            run_err("declarer t : tableau_de 2 entier; afficher(t[5]);", &[])
                .contains("hors bornes")
        );
        assert!(run_err("declarer K : constante entier <- 1; K <- 2;", &[]).contains("constante"));
        assert!(run_err("sortie;", &[]).contains("hors d'une boucle"));
        assert!(run_err("renvoie 1;", &[]).contains("hors d'une fonction"));
        assert!(run_err("si (1) afficher(1); fsi", &[]).contains("booleen"));
    }

    #[test]
    fn choix_sur_tableaux_divers() {
        assert_eq!(
            run_ok(
                "choix_sur 2 entre cas 1 : afficher(1); cas 2 : afficher(2); autre : afficher(0); fchoix",
                &[]
            ),
            "2\n"
        );
        assert_eq!(
            run_ok(
                "declarer t : tableau_de 2 entier; t[0] <- 6; afficher(t[0]); afficher(taille(t));",
                &[]
            ),
            "6\n2\n"
        );
        assert_eq!(run_ok("ligne_suivante; afficher(1);", &[]), "\n1\n");
        assert_eq!(
            run_ok("algorithme demo debut afficher(\"ok\"); fin", &[]),
            "ok\n"
        );
    }

    #[test]
    fn rand_dans_intervalle() {
        let out = run_ok("afficher(rand (1, 6));", &[]);
        let n: i64 = out.trim().parse().expect("un entier");
        assert!((1..=6).contains(&n), "{n}");
        assert!(run_err("afficher(rand (6, 1));", &[]).contains("minimum"));
    }
}
