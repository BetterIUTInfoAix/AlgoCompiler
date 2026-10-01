/// Type d'une variable.
///
/// Inclut les scalaires, les tableaux (`tableau_de taille type`) et les
/// constantes (`constante type`).
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Entier,
    EntierNaturel,
    Reel,
    Booleen,
    Caractere,
    Chaine,
    /// Tableau : `tableau_de taille type`
    Tableau {
        size: i64,
        element_type: Box<Type>,
    },
    /// Constante : `constante type`
    Constante(Box<Type>),
}

/// Mode de passage d'un paramètre de sous-programme.
///
/// - `in` : lecture seule (le sous-programme ne modifie pas la variable) ;
/// - `out` : la variable doit recevoir une valeur dans le sous-programme ;
/// - `in_out` : lecture et modification autorisées.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    In,
    Out,
    InOut,
}

/// Paramètre formel d'une fonction ou procédure : `nom : mode type`.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub mode: Mode,
    pub ty: Type,
}

/// Opérateur binaire pour les expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmétique
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    // Comparaison
    Eq, // vaut
    Ne, // ne_vaut_pas
    Lt, // <
    Gt, // >
    Le, // <=
    Ge, // >=
    // Logique
    And, // et / et_alors
    Or,  // ou / ou_sinon
}

/// Opérateur unaire pour les expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg, // - (négation arithmétique)
    Not, // non (négation logique)
}

/// Expression : littéral, variable, opérateur unaire ou binaire, accès tableau.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Entier(i64),
    Reel(f64),
    Chaine(String),
    Caractere(char),
    Booleen(bool),
    Ident(String),
    /// Appel de fonction : `nom(arg1, arg2, …)` (ex : `taille(t)`).
    Appel {
        name: String,
        args: Vec<Expr>,
    },
    /// Accès à un élément de tableau : `tableau[indice]`
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
    },
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

/// Instruction algorithmique.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// `afficher(expr);`
    Afficher(Expr),
    /// `declarer nom : type [<- expr];`
    Declarer {
        name: String,
        ty: Type,
        init: Option<Expr>,
    },
    /// `constante nom : type <- expr;`
    Constante { name: String, ty: Type, value: Expr },
    /// `nom <- expr;`
    Affecter { name: String, value: Expr },
    /// `tableau[indice] <- expr;`
    AffecterIndex {
        base: String,
        index: Expr,
        value: Expr,
    },
    /// `si (cond) { ... } [sinon { ... }] fsi`
    Si {
        condition: Expr,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    /// `choix_sur var entre cas val: ... autre: ... fchoix`
    ChoixSur {
        expr: Expr,
        cases: Vec<(Expr, Vec<Statement>)>, // (valeur du cas, instructions)
        default: Option<Vec<Statement>>,
    },
    /// `boucle ... fboucle` (boucle infinie)
    Boucle(Vec<Statement>),
    /// `repeter ... jusqua (cond);`
    Repeter {
        body: Vec<Statement>,
        condition: Expr,
    },
    /// `jusqua (cond) faire ... ffaire`
    Jusqua {
        condition: Expr,
        body: Vec<Statement>,
    },
    /// `tant_que (cond) faire ... ffaire`
    TantQue {
        condition: Expr,
        body: Vec<Statement>,
    },
    /// `pour (i variant_de a b [descendant]) faire ... ffaire`
    Pour {
        var: String,
        start: Expr,
        end: Expr,
        descending: bool,
        body: Vec<Statement>,
    },
    /// `sortie;`
    Sortie,
    /// `continue;`
    Continue,
    /// `fonction nom(params) renvoie type debut ... fin`
    Fonction {
        name: String,
        params: Vec<Param>,
        ret: Type,
        body: Vec<Statement>,
    },
    /// `procedure nom(params) debut ... fin`
    Procedure {
        name: String,
        params: Vec<Param>,
        body: Vec<Statement>,
    },
    /// `renvoie expr;` (valeur rendue par une fonction)
    Renvoie(Expr),
    /// `saisir (var);` (lecture au clavier, éventuellement plusieurs variables)
    Saisir(Vec<String>),
    /// `ligne_suivante;` (saute une ligne à l'affichage)
    LigneSuivante,
    /// `nom(args);` (appel de procédure en position d'instruction)
    Appel { name: String, args: Vec<Expr> },
    /// `algorithme nom debut … fin` (moule du programme principal)
    Algorithme { name: String, body: Vec<Statement> },
}

/// Programme complet : la liste de ses instructions.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}
