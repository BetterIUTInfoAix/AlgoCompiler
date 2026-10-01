mod ast;
pub use ast::{BinOp, Expr, Mode, Param, Program, Statement, Type, UnOp};

use crate::errors::{CompileError, CompileResult, Span};
use crate::lexer::{SpannedToken, Token};

/// Libellé attendu quand un type est requis.
const TYPE_EXPECTED: &str = "un type (`entier`, `entier_naturel`, `reel`, `booleen`, `caractere`, `string`, `tableau_de taille type` ou `constante type`)";

/// Libellé attendu quand une expression est requise.
const EXPR_EXPECTED: &str = "une expression";
const EXPR_HINT: &str = "exemples d'expressions : `42`, `3.14`, `\"texte\"`, `'c'`, `vrai`, `faux`, le nom d'une variable, `a vaut b`, `x et y`…";

/// Analyseur récursif descendant sur une suite de tokens localisés.
///
/// Chaque anomalie devient une [`CompileError`] localisée
/// (`E101` token inattendu, `E102` fin prématurée).
struct Parser<'a> {
    tokens: &'a [SpannedToken],
    pos: usize,
    /// Token `Eof` synthétique si on dépasse la fin de la liste
    /// (ne devrait pas arriver avec le lexer, mais évite un accès hors bornes).
    eof: SpannedToken,
}

/// Analyse une suite de tokens localisés en [`Program`].
pub fn parse(tokens: &[SpannedToken]) -> CompileResult<Program> {
    Parser::new(tokens).parse_program()
}

/// Vrai si le token peut commencer une instruction.
///
/// Sert à produire une erreur `fsi` manquant explicite (plutôt qu'une erreur
/// générique) quand un bloc `si` en forme longue rencontre la fin du bloc
/// englobant (`fboucle`, `ffaire`, …).
fn can_start_statement(token: &Token) -> bool {
    matches!(
        token,
        Token::Afficher
            | Token::Declarer
            | Token::Saisir
            | Token::LigneSuivante
            | Token::Fonction
            | Token::Procedure
            | Token::Algorithme
            | Token::Renvoie
            | Token::Si
            | Token::ChoixSur
            | Token::Boucle
            | Token::Repeter
            | Token::Jusqua
            | Token::TantQue
            | Token::Pour
            | Token::Sortie
            | Token::Continue
            | Token::Ident(_)
    )
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [SpannedToken]) -> Self {
        Self {
            tokens,
            pos: 0,
            eof: SpannedToken::new(Token::Eof, Span::new(1, 1, 1)),
        }
    }

    fn current(&self) -> &SpannedToken {
        self.tokens.get(self.pos).unwrap_or(&self.eof)
    }

    /// Passe au token suivant.
    fn advance(&mut self) {
        self.pos += 1;
    }

    /// Consomme le token courant si son discriminant correspond à `kind`,
    /// sinon retourne `E101` (ou `E102` en cas de fin de fichier).
    ///
    /// `expected_label` est le texte affiché à l'utilisateur (ex : "`;` en fin…").
    fn expect(&mut self, kind: Token, expected_label: &str) -> CompileResult<()> {
        if std::mem::discriminant(&self.current().token) == std::mem::discriminant(&kind) {
            self.advance();
            return Ok(());
        }
        let current = self.current();
        let span = current.span;
        if current.token == Token::Eof {
            return Err(
                CompileError::unexpected_eof(expected_label, span).with_hint(format!(
                    "instruction incomplète — il manque {expected_label}"
                )),
            );
        }
        Err(CompileError::unexpected_token(
            current.token.describe(),
            expected_label,
            span,
        ))
    }

    /// Consomme un identifiant (le nom d'une variable).
    fn expect_ident(&mut self, expected_label: &str) -> CompileResult<String> {
        let current = self.current();
        if let Token::Ident(name) = &current.token {
            let name = name.clone();
            self.advance();
            return Ok(name);
        }
        let span = current.span;
        if current.token == Token::Eof {
            return Err(
                CompileError::unexpected_eof(expected_label, span).with_hint(format!(
                    "instruction incomplète — il manque {expected_label}"
                )),
            );
        }
        Err(CompileError::unexpected_token(
            current.token.describe(),
            expected_label,
            span,
        ))
    }

    /// Consomme le séparateur `a` de la boucle `pour` (`variant_de x a y`).
    ///
    /// `a` n'est pas un mot-clé réservé du lexer (pour pouvoir l'utiliser
    /// comme nom de variable) : on accepte ici l'identifiant `a`
    /// (insensible à la casse), ainsi que `à` accentué (courant sur papier).
    fn expect_a(&mut self) -> CompileResult<()> {
        let is_a = matches!(&self.current().token, Token::Ident(name) if {
            let lower = name.to_lowercase();
            lower == "a" || lower == "à"
        });
        if is_a {
            self.advance();
            return Ok(());
        }
        let current = self.current();
        let span = current.span;
        if current.token == Token::Eof {
            return Err(CompileError::unexpected_eof(
                "`a` entre les bornes (`variant_de debut a fin`)",
                span,
            ));
        }
        Err(CompileError::unexpected_token(
            current.token.describe(),
            "`a` entre les bornes (`variant_de debut a fin`)",
            span,
        )
        .with_hint("exemple : pour (i variant_de 1 a 10) faire … ffaire"))
    }

    fn parse_program(mut self) -> CompileResult<Program> {
        let mut statements = Vec::new();
        while self.current().token != Token::Eof {
            // L'enveloppe `algorithme nom debut … fin` est une instruction
            // comme une autre : son corps rejoint le programme principal.
            if self.current().token == Token::Algorithme {
                let algo = self.parse_algorithme()?;
                statements.push(algo);
                continue;
            }
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    /// Parse un bloc d'instructions jusqu'à l'un des tokens d'arrêt.
    ///
    /// `stops` contient les tokens qui terminent le bloc (non consommés).
    /// `context` décrit le bloc pour les messages d'erreur
    /// (ex : "`fsi` pour fermer le bloc `si`").
    fn parse_block_until(
        &mut self,
        stops: &[Token],
        context: &str,
    ) -> CompileResult<Vec<Statement>> {
        let mut statements = Vec::new();
        loop {
            let cur = &self.current().token;
            if *cur == Token::Eof {
                let span = self.current().span;
                return Err(CompileError::unexpected_eof(context, span)
                    .with_hint(format!("bloc incomplet — il manque {context}")));
            }
            if stops
                .iter()
                .any(|s| std::mem::discriminant(cur) == std::mem::discriminant(s))
            {
                return Ok(statements);
            }
            statements.push(self.parse_statement()?);
        }
    }

    fn parse_statement(&mut self) -> CompileResult<Statement> {
        match &self.current().token {
            Token::Afficher => self.parse_afficher(),
            Token::Declarer => self.parse_declarer(),
            Token::Saisir => self.parse_saisir(),
            Token::LigneSuivante => {
                self.advance();
                self.expect(Token::Semicolon, "`;` après `ligne_suivante`")?;
                Ok(Statement::LigneSuivante)
            }
            Token::Fonction => self.parse_fonction(),
            Token::Procedure => self.parse_procedure(),
            Token::Algorithme => self.parse_algorithme(),
            Token::Renvoie => {
                self.advance(); // saute `renvoie`
                let value = self.parse_expr()?;
                self.expect(Token::Semicolon, "`;` après `renvoie <valeur>`")?;
                Ok(Statement::Renvoie(value))
            }
            Token::Si => self.parse_si(),
            Token::ChoixSur => self.parse_choix_sur(),
            Token::Boucle => self.parse_boucle(),
            Token::Repeter => self.parse_repeter(),
            Token::Jusqua => self.parse_jusqua_form(),
            Token::TantQue => self.parse_tant_que(),
            Token::Pour => self.parse_pour(),
            Token::Sortie => {
                self.advance();
                self.expect(Token::Semicolon, "`;` après `sortie`")?;
                Ok(Statement::Sortie)
            }
            Token::Continue => {
                self.advance();
                self.expect(Token::Semicolon, "`;` après `continue`")?;
                Ok(Statement::Continue)
            }
            Token::Ident(_) => {
                let name = match &self.current().token {
                    Token::Ident(n) => n.clone(),
                    _ => unreachable!(),
                };
                // Appel de procédure en position d'instruction : `nom(args);`.
                // On regarde le token suivant sans le consommer pour le
                // distinguer d'une affectation `nom <- valeur ;`.
                let next_is_lparen = matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.token),
                    Some(Token::LParen)
                );
                if next_is_lparen {
                    // `affichr("x");` est presque sûrement une coquille pour
                    // `afficher(...)` : on garde la suggestion au lieu
                    // d'accepter silencieusement un appel inconnu.
                    if crate::errors::suggest_keyword(&name) == Some("afficher") {
                        let span = self.current().span;
                        return Err(CompileError::unexpected_token(
                            format!("l'identifiant `{name}`"),
                            "le symbole `<-` après un identifiant",
                            span,
                        )
                        .with_hint("vouliez-vous dire `afficher` ?"));
                    }
                    return self.parse_appel_statement(&name);
                }
                let span = self.current().span;
                self.parse_affectation(&name, span)
            }
            _ => {
                let current = self.current();
                Err(CompileError::unexpected_token(
                    current.token.describe(),
                    "une instruction (`afficher`, `saisir`, `declarer`, `si`, `boucle`, `tant_que`, `pour`, `fonction`, `procedure`… ou une affectation `nom <- valeur ;`)",
                    current.span,
                )
                .with_hint(
                    "chaque instruction commence par un mot-clé ou par `nom <- valeur ;`",
                ))
            }
        }
    }

    /// `afficher ( expr ) ;`
    fn parse_afficher(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `afficher`
        self.expect(Token::LParen, "`(` après `afficher`")?;
        let value = self.parse_expr()?;
        self.expect(Token::RParen, "`)` pour fermer l'appel")?;
        self.expect(Token::Semicolon, "`;` en fin d'instruction")?;
        Ok(Statement::Afficher(value))
    }

    /// `declarer nom : type [<- expr] ;`
    ///
    /// Avec `constante` (`declarer K : constante reel <- 3.14 ;`), l'initialisation
    /// est obligatoire et produit un [`Statement::Constante`].
    fn parse_declarer(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `declarer`
        let name = self.expect_ident("le nom d'une variable")?;
        self.expect(Token::Colon, "`:` après le nom de la variable")?;
        let ty = self.parse_type()?;
        let init = if matches!(self.current().token, Token::Assign) {
            self.advance(); // saute `<-`
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(Token::Semicolon, "`;` en fin de déclaration")?;
        match ty {
            Type::Constante(inner) => match init {
                Some(value) => Ok(Statement::Constante {
                    name,
                    ty: (*inner).clone(),
                    value,
                }),
                None => {
                    // On n'a plus le span exact du type, mais l'erreur reste
                    // compréhensible sans localisation fine : on utilise E101 sans span ?
                    // Non : toute erreur doit être localisée. On signale sur `;`
                    // avec un message explicite.
                    Err(CompileError::unexpected_token(
                        "`;`",
                        "`<- valeur` après une `constante`",
                        self.tokens
                            .get(self.pos.saturating_sub(1))
                            .map(|t| t.span)
                            .unwrap_or(Span::new(1, 1, 1)),
                    )
                    .with_hint("exemple : declarer Kpi : constante reel <- 3.14 ;"))
                }
            },
            _ => Ok(Statement::Declarer { name, ty, init }),
        }
    }

    /// `nom <- expr ;` ou `nom[expr] <- expr ;`
    ///
    /// `name` / `span` désignent l'identifiant en tête d'instruction.
    fn parse_affectation(&mut self, name: &str, span: Span) -> CompileResult<Statement> {
        self.advance(); // saute l'identifiant
        // Affectation indicée : `tableau[indice] <- valeur ;`
        if matches!(self.current().token, Token::LBracket) {
            self.advance(); // saute `[`
            let index = self.parse_expr()?;
            self.expect(Token::RBracket, "`]` pour fermer l'indice")?;
            self.expect(Token::Assign, "`<-` après `tableau[indice]`")?;
            let value = self.parse_expr()?;
            self.expect(Token::Semicolon, "`;` en fin d'instruction")?;
            return Ok(Statement::AffecterIndex {
                base: name.to_string(),
                index,
                value,
            });
        }
        if !matches!(self.current().token, Token::Assign) {
            // L'identifiant est en début d'instruction mais n'est pas suivi de
            // `<-` : soit il manque l'affectation, soit un mot-clé a été
            // mal orthographié (`affichr(...)`).
            let err = CompileError::unexpected_token(
                format!("l'identifiant `{name}`"),
                "le symbole `<-` après un identifiant",
                span,
            );
            return Err(match crate::errors::suggest_keyword(name) {
                Some(keyword) => err.with_hint(format!("vouliez-vous dire `{keyword}` ?")),
                None => err.with_hint("une affectation a la forme `nom <- valeur ;`"),
            });
        }
        self.advance(); // saute `<-`
        let value = self.parse_expr()?;
        self.expect(Token::Semicolon, "`;` en fin d'instruction")?;
        Ok(Statement::Affecter {
            name: name.to_string(),
            value,
        })
    }

    /// `saisir (var [, var …]);` (lecture au clavier).
    fn parse_saisir(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `saisir`
        self.expect(Token::LParen, "`(` après `saisir`")?;
        let mut vars = vec![self.expect_ident("le nom d'une variable à lire")?];
        while matches!(self.current().token, Token::Comma) {
            self.advance(); // saute `,`
            vars.push(self.expect_ident("le nom d'une variable à lire")?);
        }
        self.expect(Token::RParen, "`)` pour fermer l'appel")?;
        self.expect(Token::Semicolon, "`;` en fin d'instruction")?;
        Ok(Statement::Saisir(vars))
    }

    /// `nom(args);` en position d'instruction (appel de procédure).
    ///
    /// `name` est l'identifiant en tête d'instruction (déjà lu, non consommé
    /// au-delà : le curseur est encore dessus à l'appel).
    fn parse_appel_statement(&mut self, name: &str) -> CompileResult<Statement> {
        let name = name.to_string();
        self.advance(); // saute le nom
        self.advance(); // saute `(`
        let mut args = Vec::new();
        if !matches!(self.current().token, Token::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if matches!(self.current().token, Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen, "`)` pour fermer l'appel")?;
        self.expect(Token::Semicolon, "`;` en fin d'instruction")?;
        Ok(Statement::Appel { name, args })
    }

    /// `fonction nom(params) renvoie type debut … fin`
    fn parse_fonction(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `fonction`
        let name = self.expect_ident("le nom de la fonction")?;
        self.expect(Token::LParen, "`(` après le nom de la fonction")?;
        let params = self.parse_params()?;
        self.expect(Token::RParen, "`)` pour fermer les paramètres")?;
        self.expect(
            Token::Renvoie,
            "`renvoie <type>` après les paramètres (`fonction f(x : in entier) renvoie entier`)",
        )?;
        let ret = self.parse_type()?;
        self.expect(Token::Debut, "`debut` après l'en-tête de la fonction")?;
        let body = self.parse_routine_body("`fin` pour fermer la fonction")?;
        Ok(Statement::Fonction {
            name,
            params,
            ret,
            body,
        })
    }

    /// `procedure nom(params) debut … fin`
    fn parse_procedure(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `procedure`
        let name = self.expect_ident("le nom de la procédure")?;
        self.expect(Token::LParen, "`(` après le nom de la procédure")?;
        let params = self.parse_params()?;
        self.expect(Token::RParen, "`)` pour fermer les paramètres")?;
        self.expect(Token::Debut, "`debut` après l'en-tête de la procédure")?;
        let body = self.parse_routine_body("`fin` pour fermer la procédure")?;
        Ok(Statement::Procedure { name, params, body })
    }

    /// `algorithme nom debut … fin` (moule du programme principal).
    fn parse_algorithme(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `algorithme`
        let name = self.expect_ident("le nom de l'algorithme")?;
        self.expect(Token::Debut, "`debut` après `algorithme <nom>`")?;
        let body = self.parse_routine_body("`fin` pour fermer l'algorithme")?;
        Ok(Statement::Algorithme { name, body })
    }

    /// Corps d'une routine (`fonction`, `procedure`, `algorithme`) jusqu'à `fin`.
    fn parse_routine_body(&mut self, context: &str) -> CompileResult<Vec<Statement>> {
        let mut body = Vec::new();
        loop {
            if self.current().token == Token::Eof {
                let span = self.current().span;
                return Err(CompileError::unexpected_eof(context, span)
                    .with_hint(format!("bloc incomplet — il manque {context}")));
            }
            if self.current().token == Token::Fin {
                self.advance(); // saute `fin`
                return Ok(body);
            }
            body.push(self.parse_statement()?);
        }
    }

    /// Liste de paramètres formels (éventuellement vide) : `nom : mode type, …`.
    fn parse_params(&mut self) -> CompileResult<Vec<Param>> {
        let mut params = Vec::new();
        if matches!(self.current().token, Token::RParen) {
            return Ok(params);
        }
        loop {
            let name = self.expect_ident("le nom d'un paramètre")?;
            self.expect(Token::Colon, "`:` après le nom du paramètre")?;
            let mode = self.parse_param_mode()?;
            let ty = self.parse_type()?;
            params.push(Param { name, mode, ty });
            if matches!(self.current().token, Token::Comma) {
                self.advance(); // saute `,`
            } else {
                return Ok(params);
            }
        }
    }

    /// Marqueur de paramètre : `in`, `out` ou `in_out`.
    ///
    /// Lus comme de simples identifiants (comme le `a` de `variant_de … a …`)
    /// pour ne pas réserver ces mots dans tout le langage.
    fn parse_param_mode(&mut self) -> CompileResult<Mode> {
        let (name, span) = match &self.current().token {
            Token::Ident(name) => (name.clone(), self.current().span),
            Token::Eof => {
                return Err(CompileError::unexpected_eof(
                    "`in`, `out` ou `in_out` avant le type du paramètre",
                    self.current().span,
                ));
            }
            other => {
                return Err(CompileError::unexpected_token(
                    other.describe(),
                    "`in`, `out` ou `in_out` avant le type du paramètre",
                    self.current().span,
                )
                .with_hint("exemple : `x : in entier`, `c : in_out entier`"));
            }
        };
        let mode = if name.eq_ignore_ascii_case("in") {
            Mode::In
        } else if name.eq_ignore_ascii_case("out") {
            Mode::Out
        } else if name.eq_ignore_ascii_case("in_out") {
            Mode::InOut
        } else {
            return Err(CompileError::unexpected_token(
                format!("l'identifiant `{name}`"),
                "`in`, `out` ou `in_out` avant le type du paramètre",
                span,
            )
            .with_hint("exemple : `x : in entier`, `c : in_out entier`"));
        };
        self.advance();
        Ok(mode)
    }

    /// `si (cond) ... [sinon_si (cond) ...]* [sinon ...] fsi`
    ///
    /// Les `sinon_si` sont désucrés en `Si` imbriqués dans la branche `sinon`.
    ///
    /// Forme courte (sans `fsi`) : quand une seule instruction suit la
    /// condition **sur la même ligne**, elle forme à elle seule le `si` :
    /// ```text
    /// boucle
    /// si (k vaut 2) sortie;
    /// afficher(k);
    /// fboucle
    /// ```
    /// Un `si` écrit sur plusieurs lignes exige toujours son `fsi`
    /// (chaque `si` veut son `fsi`), tout comme les formes avec `sinon_si` /
    /// `sinon` ou plusieurs instructions sur la même ligne.
    fn parse_si(&mut self) -> CompileResult<Statement> {
        let si_line = self.current().span.line;
        self.advance(); // saute `si`
        self.expect(Token::LParen, "`(` après `si`")?;
        let condition = self.parse_expr()?;
        self.expect(Token::RParen, "`)` pour fermer la condition")?;
        // Branche vide : `si (cond) fsi` / `si (cond) sinon …` (forme longue).
        if matches!(
            self.current().token,
            Token::SinonSi | Token::Sinon | Token::FSi
        ) {
            return self.finish_si_long(condition, Vec::new());
        }
        let first_start_line = self.current().span.line;
        let first = self.parse_statement()?;
        // Ligne du dernier token consommé (le `;` d'une instruction simple,
        // le mot de fin d'un bloc) : distingue la forme courte sur une ligne
        // de la forme longue.
        let first_end_line = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.line)
            .unwrap_or(first_start_line);
        // Suite de la forme longue : `sinon_si` / `sinon` / `fsi` juste après.
        if matches!(
            self.current().token,
            Token::SinonSi | Token::Sinon | Token::FSi
        ) {
            return self.finish_si_long(condition, vec![first]);
        }
        // Forme courte : une seule instruction, sans `fsi`. Deux cas :
        // - le `si` et son instruction sont sur la même ligne et la suite
        //   est sur une autre ligne (garde `si (k vaut 2) sortie;` dans
        //   une boucle, suivie d'autres instructions) ;
        // - la suite ne peut pas commencer une instruction (`fboucle`,
        //   `ffaire`, `fin`, fin de fichier…) : le `si` se termine ici.
        if !can_start_statement(&self.current().token)
            || (first_start_line == si_line && self.current().span.line > first_end_line)
        {
            return Ok(Statement::Si {
                condition,
                then_branch: vec![first],
                else_branch: None,
            });
        }
        // Sinon : forme longue (`si (a) s1; s2; fsi`, `si (a)\n s1; … fsi`).
        let mut then_branch = vec![first];
        self.parse_si_rest(&mut then_branch)?;
        self.finish_si_long(condition, then_branch)
    }

    /// Termine un bloc `si` en forme longue : chaîne de `sinon_si`,
    /// `sinon` optionnel, puis `fsi` obligatoire.
    fn finish_si_long(
        &mut self,
        condition: Expr,
        then_branch: Vec<Statement>,
    ) -> CompileResult<Statement> {
        // Chaîne éventuelle de `sinon_si`.
        let mut else_branch: Option<Vec<Statement>> = None;
        // On collecte les `sinon_si` puis on les replie de droite à gauche.
        let mut sinon_si_chain: Vec<(Expr, Vec<Statement>)> = Vec::new();
        while matches!(self.current().token, Token::SinonSi) {
            self.advance(); // saute `sinon_si`
            self.expect(Token::LParen, "`(` après `sinon_si`")?;
            let cond = self.parse_expr()?;
            self.expect(Token::RParen, "`)` pour fermer la condition")?;
            let mut body = Vec::new();
            self.parse_si_rest(&mut body)?;
            sinon_si_chain.push((cond, body));
        }
        if matches!(self.current().token, Token::Sinon) {
            self.advance(); // saute `sinon`
            let body =
                self.parse_block_until(&[Token::FSi], "`fsi` pour fermer le bloc `sinon`")?;
            else_branch = Some(body);
        }
        // Repli des `sinon_si` : le dernier devient le plus imbriqué.
        for (cond, body) in sinon_si_chain.into_iter().rev() {
            let nested = Statement::Si {
                condition: cond,
                then_branch: body,
                else_branch,
            };
            else_branch = Some(vec![nested]);
        }
        self.expect(Token::FSi, "`fsi` pour fermer le bloc `si`")?;
        Ok(Statement::Si {
            condition,
            then_branch,
            else_branch,
        })
    }

    /// Parse les instructions d'un bloc `si` / `sinon_si` (forme longue)
    /// jusqu'à `sinon_si` / `sinon` / `fsi` (non consommés).
    ///
    /// Contrairement à [`parse_block_until`], un terminateur de bloc
    /// englobant (`fboucle`, `ffaire`, …) produit une erreur `fsi` manquant
    /// explicite plutôt qu'une erreur générique sur le terminateur.
    fn parse_si_rest(&mut self, then_branch: &mut Vec<Statement>) -> CompileResult<()> {
        loop {
            let cur = &self.current().token;
            if *cur == Token::Eof {
                let span = self.current().span;
                return Err(
                    CompileError::unexpected_eof("`fsi` pour fermer le bloc `si`", span)
                        .with_hint("bloc incomplet — il manque `fsi`"),
                );
            }
            if matches!(cur, Token::SinonSi | Token::Sinon | Token::FSi) {
                return Ok(());
            }
            if !can_start_statement(cur) {
                let span = self.current().span;
                return Err(CompileError::unexpected_token(
                    cur.describe(),
                    "`fsi` pour fermer le bloc `si`",
                    span,
                )
                .with_hint("chaque `si` veut son `fsi` — ajoutez `fsi` avant la fin du bloc"));
            }
            then_branch.push(self.parse_statement()?);
        }
    }

    /// `choix_sur expr entre (cas expr : ...)* [autre : ...] fchoix`
    fn parse_choix_sur(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `choix_sur`
        let expr = self.parse_expr()?;
        self.expect(Token::Entre, "`entre` après `choix_sur <variable>`")?;
        let mut cases: Vec<(Expr, Vec<Statement>)> = Vec::new();
        let mut default: Option<Vec<Statement>> = None;
        loop {
            match &self.current().token {
                Token::Cas => {
                    self.advance(); // saute `cas`
                    let value = self.parse_expr()?;
                    self.expect(Token::Colon, "`:` après la valeur du `cas`")?;
                    let body = self.parse_block_until(
                        &[Token::Cas, Token::Autre, Token::FChoix],
                        "`fchoix` pour fermer le bloc `choix_sur`",
                    )?;
                    cases.push((value, body));
                }
                Token::Autre => {
                    self.advance(); // saute `autre`
                    self.expect(Token::Colon, "`:` après `autre`")?;
                    let body = self.parse_block_until(
                        &[Token::FChoix],
                        "`fchoix` pour fermer le bloc `choix_sur`",
                    )?;
                    default = Some(body);
                    // `autre` est le dernier bloc possible.
                    break;
                }
                Token::FChoix => break,
                Token::Eof => {
                    let span = self.current().span;
                    return Err(
                        CompileError::unexpected_eof("`cas`, `autre` ou `fchoix`", span)
                            .with_hint("bloc `choix_sur` incomplet — il manque `fchoix`"),
                    );
                }
                _ => {
                    let current = self.current();
                    return Err(CompileError::unexpected_token(
                        current.token.describe(),
                        "`cas`, `autre` ou `fchoix`",
                        current.span,
                    )
                    .with_hint("exemple : cas 1 : afficher(1); … autre : afficher(0); fchoix"));
                }
            }
        }
        self.expect(Token::FChoix, "`fchoix` pour fermer le bloc `choix_sur`")?;
        Ok(Statement::ChoixSur {
            expr,
            cases,
            default,
        })
    }

    /// `boucle ... fboucle` (boucle infinie, sortie via `sortie ;`).
    fn parse_boucle(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `boucle`
        let body = self.parse_block_until(&[Token::FBoucle], "`fboucle` pour fermer la boucle")?;
        self.expect(Token::FBoucle, "`fboucle` pour fermer la boucle")?;
        Ok(Statement::Boucle(body))
    }

    /// `repeter ... jusqua (cond) ;` (exécute au moins une fois).
    fn parse_repeter(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `repeter`
        let body = self.parse_block_until(
            &[Token::Jusqua],
            "`jusqua (condition) ;` pour fermer le bloc `repeter`",
        )?;
        self.expect(Token::Jusqua, "`jusqua` après le bloc `repeter`")?;
        self.expect(Token::LParen, "`(` après `jusqua`")?;
        let condition = self.parse_expr()?;
        self.expect(Token::RParen, "`)` pour fermer la condition")?;
        self.expect(Token::Semicolon, "`;` après `jusqua (condition)`")?;
        Ok(Statement::Repeter { body, condition })
    }

    /// `jusqua (cond) faire ... ffaire` (teste avant d'exécuter).
    fn parse_jusqua_form(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `jusqua`
        self.expect(Token::LParen, "`(` après `jusqua`")?;
        let condition = self.parse_expr()?;
        self.expect(Token::RParen, "`)` pour fermer la condition")?;
        self.expect(Token::Faire, "`faire` après `jusqua (condition)`")?;
        let body = self.parse_block_until(&[Token::FFaire], "`ffaire` pour fermer la boucle")?;
        self.expect(Token::FFaire, "`ffaire` pour fermer la boucle")?;
        Ok(Statement::Jusqua { condition, body })
    }

    /// `tant_que (cond) faire ... ffaire`.
    fn parse_tant_que(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `tant_que`
        self.expect(Token::LParen, "`(` après `tant_que`")?;
        let condition = self.parse_expr()?;
        self.expect(Token::RParen, "`)` pour fermer la condition")?;
        self.expect(Token::Faire, "`faire` après `tant_que (condition)`")?;
        let body = self.parse_block_until(&[Token::FFaire], "`ffaire` pour fermer la boucle")?;
        self.expect(Token::FFaire, "`ffaire` pour fermer la boucle")?;
        Ok(Statement::TantQue { condition, body })
    }

    /// `pour (i variant_de debut a fin [descendant]) faire ... ffaire`.
    fn parse_pour(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `pour`
        self.expect(Token::LParen, "`(` après `pour`")?;
        let var = self.expect_ident("le nom du compteur (`i` par convention)")?;
        self.expect(
            Token::VariantDe,
            "`variant_de` après le compteur (`pour (i variant_de …)`)",
        )?;
        let start = self.parse_expr()?;
        self.expect_a()?;
        let end = self.parse_expr()?;
        let descending = matches!(self.current().token, Token::Descendant);
        if descending {
            self.advance();
        }
        self.expect(Token::RParen, "`)` pour fermer l'en-tête `pour`")?;
        self.expect(Token::Faire, "`faire` après `pour (...)`")?;
        let body = self.parse_block_until(&[Token::FFaire], "`ffaire` pour fermer la boucle")?;
        self.expect(Token::FFaire, "`ffaire` pour fermer la boucle")?;
        Ok(Statement::Pour {
            var,
            start,
            end,
            descending,
            body,
        })
    }

    fn parse_type(&mut self) -> CompileResult<Type> {
        let current = self.current();
        match &current.token {
            Token::TyEntier => {
                self.advance();
                Ok(Type::Entier)
            }
            Token::TyEntierNaturel => {
                self.advance();
                Ok(Type::EntierNaturel)
            }
            Token::TyReel => {
                self.advance();
                Ok(Type::Reel)
            }
            Token::TyBooleen => {
                self.advance();
                Ok(Type::Booleen)
            }
            Token::TyCaractere => {
                self.advance();
                Ok(Type::Caractere)
            }
            Token::TyChaine => {
                self.advance();
                Ok(Type::Chaine)
            }
            Token::TyTableau => {
                self.advance(); // saute `tableau_de`
                // Taille : entier littéral (`tableau_de 3 entier`).
                let size = match &self.current().token {
                    Token::IntLit(n) => {
                        let n = *n;
                        self.advance();
                        n
                    }
                    Token::Eof => {
                        let span = self.current().span;
                        return Err(CompileError::unexpected_eof(
                            "la taille du tableau (`tableau_de 3 entier`)",
                            span,
                        ));
                    }
                    _ => {
                        let cur = self.current();
                        return Err(CompileError::unexpected_token(
                            cur.token.describe(),
                            "la taille du tableau (`tableau_de 3 entier`)",
                            cur.span,
                        )
                        .with_hint("exemple : declarer t : tableau_de 3 entier ;"));
                    }
                };
                let element = self.parse_scalar_type()?;
                Ok(Type::Tableau {
                    size,
                    element_type: Box::new(element),
                })
            }
            Token::Constant => {
                self.advance(); // saute `constante`
                let inner = self.parse_scalar_type()?;
                Ok(Type::Constante(Box::new(inner)))
            }
            Token::Eof => Err(CompileError::unexpected_eof(TYPE_EXPECTED, current.span)
                .with_hint("déclaration incomplète — il manque le type de la variable")),
            other => {
                Err(
                    CompileError::unexpected_token(other.describe(), TYPE_EXPECTED, current.span)
                        .with_hint("exemple : declarer unEntier : entier ;"),
                )
            }
        }
    }

    /// Parse un type scalaire (sans `tableau_de` ni `constante` imbriqués).
    fn parse_scalar_type(&mut self) -> CompileResult<Type> {
        let current = self.current();
        let ty = match &current.token {
            Token::TyEntier => Type::Entier,
            Token::TyEntierNaturel => Type::EntierNaturel,
            Token::TyReel => Type::Reel,
            Token::TyBooleen => Type::Booleen,
            Token::TyCaractere => Type::Caractere,
            Token::TyChaine => Type::Chaine,
            Token::Eof => {
                return Err(CompileError::unexpected_eof(TYPE_EXPECTED, current.span));
            }
            other => {
                return Err(CompileError::unexpected_token(
                    other.describe(),
                    TYPE_EXPECTED,
                    current.span,
                ));
            }
        };
        self.advance();
        Ok(ty)
    }

    // --- Expressions (précédence : ou < et < comparaison < unaire < primaire) ---

    fn parse_expr(&mut self) -> CompileResult<Expr> {
        self.parse_or()
    }

    /// `ou` / `ou_sinon` (priorité la plus basse).
    fn parse_or(&mut self) -> CompileResult<Expr> {
        let mut left = self.parse_and()?;
        while matches!(self.current().token, Token::Ou | Token::OuSinon) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// `et` / `et_alors`.
    fn parse_and(&mut self) -> CompileResult<Expr> {
        let mut left = self.parse_comparison()?;
        while matches!(self.current().token, Token::Et | Token::EtAlors) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// Comparaisons : `vaut`, `ne_vaut_pas`, `<`, `>`, `<=`, `>=`.
    fn parse_comparison(&mut self) -> CompileResult<Expr> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match &self.current().token {
                Token::Vaut => BinOp::Eq,
                Token::NeVautPas => BinOp::Ne,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Le => BinOp::Le,
                Token::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// Additions et soustractions : `+`, `-` (binaire).
    fn parse_additive(&mut self) -> CompileResult<Expr> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match &self.current().token {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// Multiplications et divisions : `*`, `/`.
    fn parse_multiplicative(&mut self) -> CompileResult<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match &self.current().token {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// Unaires : `-expr` (négation) et `non expr` (négation logique).
    fn parse_unary(&mut self) -> CompileResult<Expr> {
        if matches!(self.current().token, Token::Minus) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnOp {
                op: UnOp::Neg,
                expr: Box::new(operand),
            });
        }
        if matches!(self.current().token, Token::Non) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnOp {
                op: UnOp::Not,
                expr: Box::new(operand),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> CompileResult<Expr> {
        // Parenthèse : `( expr )`.
        if matches!(self.current().token, Token::LParen) {
            self.advance();
            let expr = self.parse_expr()?;
            self.expect(Token::RParen, "`)` pour fermer la parenthèse")?;
            return Ok(expr);
        }
        let current = self.current();
        // Appel de fonction : `nom(args)` — on regarde le token suivant
        // sans le consommer pour distinguer d'une simple variable.
        if let Token::Ident(name) = &current.token {
            let name = name.clone();
            let next_is_lparen = matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.token),
                Some(Token::LParen)
            );
            if next_is_lparen {
                self.advance(); // saute le nom
                self.advance(); // saute `(`
                let mut args = Vec::new();
                if !matches!(self.current().token, Token::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if matches!(self.current().token, Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RParen, "`)` pour fermer l'appel")?;
                return Ok(Expr::Appel { name, args });
            }
        }
        let mut expr = match &current.token {
            Token::IntLit(n) => Expr::Entier(*n),
            Token::FloatLit(f) => Expr::Reel(*f),
            Token::StringLit(s) => Expr::Chaine(s.clone()),
            Token::CharLit(c) => Expr::Caractere(*c),
            Token::Vrai => Expr::Booleen(true),
            Token::Faux => Expr::Booleen(false),
            Token::Ident(name) => Expr::Ident(name.clone()),
            Token::Eof => {
                return Err(CompileError::unexpected_eof(EXPR_EXPECTED, current.span)
                    .with_hint(format!("expression incomplète — il manque {EXPR_EXPECTED}")));
            }
            other => {
                return Err(CompileError::unexpected_token(
                    other.describe(),
                    EXPR_EXPECTED,
                    current.span,
                )
                .with_hint(EXPR_HINT));
            }
        };
        self.advance();
        // Accès indicé chaînés : `t[i]`, `t[i][j]`.
        while matches!(self.current().token, Token::LBracket) {
            self.advance(); // saute `[`
            let index = self.parse_expr()?;
            self.expect(Token::RBracket, "`]` pour fermer l'indice")?;
            expr = Expr::Index {
                base: Box::new(expr),
                index: Box::new(index),
            };
        }
        Ok(expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse_source(source: &str) -> CompileResult<Program> {
        let tokens = tokenize(source)?;
        parse(&tokens)
    }

    #[test]
    fn test_parse_afficher_chaine() {
        let program = parse_source(r#"afficher("salut");"#).unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Afficher(Expr::Chaine("salut".to_string()))]
        );
    }

    #[test]
    fn test_parse_afficher_variable() {
        let program = parse_source("afficher(unEntier);").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Afficher(Expr::Ident("unEntier".to_string()))]
        );
    }

    #[test]
    fn test_parse_declarer() {
        let program = parse_source("declarer unEntier : entier;").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Declarer {
                name: "unEntier".to_string(),
                ty: Type::Entier,
                init: None,
            }]
        );
    }

    #[test]
    fn test_parse_declarer_avec_init() {
        let program = parse_source("declarer x : entier <- 42;").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Declarer {
                name: "x".to_string(),
                ty: Type::Entier,
                init: Some(Expr::Entier(42)),
            }]
        );
    }

    #[test]
    fn test_parse_declarer_tous_les_types() {
        for decl in [
            "declarer a : entier;",
            "declarer b : entier_naturel;",
            "declarer c : reel;",
            "declarer d : booleen;",
            "declarer e : caractere;",
            "declarer f : string;",
            "declarer t : tableau_de 3 entier;",
            "declarer Kpi : constante reel <- 3.14;",
        ] {
            let program = parse_source(decl).unwrap();
            assert_eq!(program.statements.len(), 1, "{decl}");
        }
    }

    #[test]
    fn test_parse_constante_sans_init_erreur() {
        let err = parse_source("declarer Kpi : constante reel;").unwrap_err();
        assert_eq!(err.code(), "E101");
    }

    #[test]
    fn test_parse_affectation() {
        let program = parse_source("x <- 42;").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Affecter {
                name: "x".to_string(),
                value: Expr::Entier(42),
            }]
        );
    }

    #[test]
    fn test_parse_affectation_negative() {
        let program = parse_source("x <- -5;").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Affecter {
                name: "x".to_string(),
                value: Expr::UnOp {
                    op: UnOp::Neg,
                    expr: Box::new(Expr::Entier(5)),
                },
            }]
        );
    }

    #[test]
    fn test_parse_affectation_index() {
        let program = parse_source("t[0] <- 6;").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::AffecterIndex {
                base: "t".to_string(),
                index: Expr::Entier(0),
                value: Expr::Entier(6),
            }]
        );
    }

    #[test]
    fn test_parse_comparaisons() {
        let program = parse_source("si (a vaut b) afficher(a); fsi").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Si { .. }]
        ));
        for cond in ["vaut", "ne_vaut_pas", "<", ">", "<=", ">="] {
            assert!(
                parse_source(&format!("si (a {cond} b) afficher(a); fsi")).is_ok(),
                "{cond}"
            );
        }
    }

    #[test]
    fn test_parse_operateurs_logiques_et_priorite() {
        // `et` prioritaire sur `ou` : `a ou b et c` == `a ou (b et c)`.
        let program = parse_source("si (a ou b et c) afficher(a); fsi").unwrap();
        match &program.statements[0] {
            Statement::Si { condition, .. } => assert_eq!(
                *condition,
                Expr::BinOp {
                    op: BinOp::Or,
                    left: Box::new(Expr::Ident("a".to_string())),
                    right: Box::new(Expr::BinOp {
                        op: BinOp::And,
                        left: Box::new(Expr::Ident("b".to_string())),
                        right: Box::new(Expr::Ident("c".to_string())),
                    }),
                }
            ),
            other => panic!("attendu Si, trouvé {other:?}"),
        }
    }

    #[test]
    fn test_parse_non_et_parentheses() {
        let program = parse_source("si (non (a vaut b)) afficher(a); fsi").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Si { .. }]
        ));
    }

    #[test]
    fn test_parse_si_sinon() {
        let program = parse_source("si (a vaut b) afficher(a); sinon afficher(b); fsi").unwrap();
        match &program.statements[0] {
            Statement::Si { else_branch, .. } => assert!(else_branch.is_some()),
            other => panic!("attendu Si, trouvé {other:?}"),
        }
    }

    #[test]
    fn test_parse_si_sinon_si() {
        let program = parse_source(
            "si (a vaut 1) afficher(a); sinon_si (a vaut 2) afficher(b); sinon afficher(c); fsi",
        )
        .unwrap();
        match &program.statements[0] {
            Statement::Si { else_branch, .. } => {
                // `sinon_si` désucré en `Si` imbriqué.
                assert!(matches!(
                    else_branch.as_deref(),
                    Some([Statement::Si { .. }])
                ));
            }
            other => panic!("attendu Si, trouvé {other:?}"),
        }
    }

    #[test]
    fn test_parse_choix_sur() {
        let program = parse_source(
            "choix_sur x entre cas 1 : afficher(1); cas 2 : afficher(2); autre : afficher(0); fchoix",
        )
        .unwrap();
        match &program.statements[0] {
            Statement::ChoixSur { cases, default, .. } => {
                assert_eq!(cases.len(), 2);
                assert!(default.is_some());
            }
            other => panic!("attendu ChoixSur, trouvé {other:?}"),
        }
    }

    #[test]
    fn test_parse_boucle() {
        let program = parse_source("boucle afficher(1); fboucle").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Boucle(_)]
        ));
    }

    #[test]
    fn test_parse_repeter_jusqua() {
        let program = parse_source("repeter afficher(1); jusqua (x vaut 5);").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Repeter { .. }]
        ));
    }

    #[test]
    fn test_parse_jusqua_faire() {
        let program = parse_source("jusqua (x vaut 5) faire afficher(1); ffaire").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Jusqua { .. }]
        ));
    }

    #[test]
    fn test_parse_tant_que() {
        let program = parse_source("tant_que (x vaut 5) faire afficher(1); ffaire").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::TantQue { .. }]
        ));
    }

    #[test]
    fn test_parse_pour() {
        let program = parse_source("pour (i variant_de 1 a 10) faire afficher(i); ffaire").unwrap();
        match &program.statements[0] {
            Statement::Pour {
                var, descending, ..
            } => {
                assert_eq!(var, "i");
                assert!(!descending);
            }
            other => panic!("attendu Pour, trouvé {other:?}"),
        }
        let program =
            parse_source("pour (i variant_de 10 a 1 descendant) faire afficher(i); ffaire")
                .unwrap();
        match &program.statements[0] {
            Statement::Pour { descending, .. } => assert!(descending),
            other => panic!("attendu Pour, trouvé {other:?}"),
        }
    }

    #[test]
    fn test_parse_pour_accent() {
        // `à` accentué accepté comme séparateur (`variant_de 0 à 10`).
        let program = parse_source("pour (i variant_de 0 à 3) faire afficher(i); ffaire").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Pour { .. }]
        ));
    }

    #[test]
    fn test_parse_sortie_continue() {
        let program = parse_source("boucle sortie; fboucle").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Boucle(body)] if matches!(body.as_slice(), [Statement::Sortie])
        ));
        let program = parse_source("boucle continue; fboucle").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Boucle(body)] if matches!(body.as_slice(), [Statement::Continue])
        ));
    }

    #[test]
    fn test_parse_arithmetique_priorite() {
        // `*` prioritaire sur `+` : `a + b * 2` == `a + (b * 2)`.
        let program = parse_source("x <- a + b * 2;").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Affecter {
                name: "x".to_string(),
                value: Expr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(Expr::Ident("a".to_string())),
                    right: Box::new(Expr::BinOp {
                        op: BinOp::Mul,
                        left: Box::new(Expr::Ident("b".to_string())),
                        right: Box::new(Expr::Entier(2)),
                    }),
                },
            }]
        );
        // Les parenthèses changent l'ordre : `(a + b) * 2`.
        let program = parse_source("x <- (a + b) * 2;").unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Affecter {
                value: Expr::BinOp { op: BinOp::Mul, .. },
                ..
            }
        ));
    }

    #[test]
    fn test_parse_appel_fonction() {
        let program = parse_source("x <- taille(t);").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Affecter {
                name: "x".to_string(),
                value: Expr::Appel {
                    name: "taille".to_string(),
                    args: vec![Expr::Ident("t".to_string())],
                },
            }]
        );
    }

    #[test]
    fn test_programme_complet() {
        let program = parse_source("declarer x : reel;\nx <- 3.14;\nafficher(x);\n").unwrap();
        assert_eq!(program.statements.len(), 3);
        assert_eq!(
            program.statements[2],
            Statement::Afficher(Expr::Ident("x".to_string()))
        );
    }

    #[test]
    fn test_point_virgule_manquant() {
        let err = parse_source("afficher(\"salut\")").unwrap_err();
        assert_eq!(err.code(), "E102"); // Eof atteint alors que `;` était attendu
        assert!(err.render("afficher(\"salut\")", None).contains(";"));
    }

    #[test]
    fn test_parenthese_manquante() {
        let err = parse_source("afficher\"salut\");").unwrap_err();
        assert_eq!(err.code(), "E101");
    }

    #[test]
    fn test_token_inattendu() {
        let tokens = tokenize("afficher(\"a\");").unwrap();
        // Injecte un `;` surnuméraire en tête pour simuler un token inattendu.
        let mut bad = vec![tokens[4].clone()];
        bad.extend_from_slice(&tokens);
        let err = parse(&bad).unwrap_err();
        assert_eq!(err.code(), "E101");
        assert!(err.hint.is_some());
    }

    #[test]
    fn test_expression_manquante() {
        let err = parse_source("afficher();").unwrap_err();
        assert_eq!(err.code(), "E101");
        assert!(err.kind.message().contains("expression"));
    }

    #[test]
    fn test_type_manquant() {
        let err = parse_source("declarer x : ;").unwrap_err();
        assert_eq!(err.code(), "E101");
        assert!(err.kind.message().contains("type"));
    }

    #[test]
    fn test_affectation_sans_valeur() {
        let err = parse_source("x <- ;").unwrap_err();
        assert_eq!(err.code(), "E101");
    }

    #[test]
    fn test_fsi_manquant() {
        // Forme longue sur plusieurs lignes sans `fsi` : erreur.
        let err = parse_source("si (a vaut b)\nafficher(a);\nafficher(b);\n").unwrap_err();
        assert_eq!(err.code(), "E102");
    }

    #[test]
    fn test_si_court_sans_fsi() {
        // Forme courte : `si` et son unique instruction sur la même ligne,
        // sans `fsi` (cas de l'issue : `si (condition) sortie;`).
        let program = parse_source("boucle si (k vaut 2) sortie; fboucle").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Boucle(body)]
                if matches!(body.as_slice(), [Statement::Si { else_branch: None, .. }])
        ));
        // En fin de fichier aussi.
        let program = parse_source("si (a vaut b) afficher(a);").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Si {
                else_branch: None,
                ..
            }]
        ));
    }

    #[test]
    fn test_si_court_garde_en_boucle() {
        // La garde `si … sortie;` ne mange pas l'instruction suivante :
        // `afficher(k)` reste dans la boucle, hors du `si`.
        let program = parse_source("boucle\nsi (k vaut 2) sortie;\nafficher(k);\nfboucle").unwrap();
        match program.statements.as_slice() {
            [Statement::Boucle(body)] => {
                assert_eq!(body.len(), 2);
                assert!(matches!(body[0], Statement::Si { .. }));
                assert!(matches!(body[1], Statement::Afficher(_)));
            }
            other => panic!("attendu Boucle, trouvé {other:?}"),
        }
    }

    #[test]
    fn test_si_long_une_ligne_plusieurs_instructions() {
        // Même ligne + `fsi` final : forme longue à deux instructions.
        let program = parse_source("si (a vaut b) afficher(a); afficher(b); fsi").unwrap();
        match program.statements.as_slice() {
            [Statement::Si { then_branch, .. }] => assert_eq!(then_branch.len(), 2),
            other => panic!("attendu Si, trouvé {other:?}"),
        }
        // Même ligne sans `fsi` : il manque le `fsi` de la forme longue.
        let err = parse_source("si (a vaut b) afficher(a); afficher(b);").unwrap_err();
        assert_eq!(err.code(), "E102");
    }

    #[test]
    fn test_si_court_avant_fboucle() {
        // `si` à une instruction juste avant `fboucle` : forme courte valide.
        let program = parse_source("boucle\nsi (k vaut 2)\nsortie;\nfboucle").unwrap();
        assert!(matches!(
            program.statements.as_slice(),
            [Statement::Boucle(body)]
                if matches!(body.as_slice(), [Statement::Si { else_branch: None, .. }])
        ));
    }

    #[test]
    fn test_si_sans_fsi_avant_fboucle_erreur_claire() {
        // Forme longue multi-lignes oubliée dans une boucle : l'erreur parle
        // du `fsi` manquant (pas du `fboucle` qui suit).
        let source = "boucle\nsi (k vaut 2)\nsortie;\nafficher(k);\nfboucle";
        let err = parse_source(source).unwrap_err();
        let rendered = err.render(source, None);
        assert!(rendered.contains("fsi"), "{rendered}");
    }

    #[test]
    fn test_ffaire_manquant() {
        let err = parse_source("tant_que (a vaut b) faire afficher(a);").unwrap_err();
        assert_eq!(err.code(), "E102");
    }

    #[test]
    fn test_mot_cle_inconnu_suggestion() {
        // `affichr` est un identifiant valide, mais suivi de `(` il est
        // presque sûr que l'utilisateur voulait écrire `afficher`.
        let err = parse_source(r#"affichr("x");"#).unwrap_err();
        assert_eq!(err.code(), "E101");
        assert!(
            err.hint
                .as_ref()
                .is_some_and(|h| h.contains("vouliez-vous dire `afficher`")),
            "{:?}",
            err.hint
        );
    }

    #[test]
    fn test_parse_fonction() {
        let program =
            parse_source("fonction double(x : in entier) renvoie entier debut renvoie x * 2; fin")
                .unwrap();
        match program.statements.as_slice() {
            [
                Statement::Fonction {
                    name,
                    params,
                    ret,
                    body,
                },
            ] => {
                assert_eq!(name, "double");
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "x");
                assert_eq!(params[0].mode, Mode::In);
                assert_eq!(params[0].ty, Type::Entier);
                assert_eq!(*ret, Type::Entier);
                assert_eq!(body.len(), 1);
                assert!(matches!(body[0], Statement::Renvoie(_)));
            }
            other => panic!("attendu Fonction, trouvé {other:?}"),
        }
    }

    #[test]
    fn test_parse_procedure_in_out() {
        let program =
            parse_source("procedure incrementer(c : in_out entier) debut c <- c + 1; fin").unwrap();
        match program.statements.as_slice() {
            [Statement::Procedure { name, params, body }] => {
                assert_eq!(name, "incrementer");
                assert_eq!(params[0].mode, Mode::InOut);
                assert_eq!(body.len(), 1);
            }
            other => panic!("attendu Procedure, trouvé {other:?}"),
        }
    }

    #[test]
    fn test_parse_fonction_plusieurs_params() {
        let program = parse_source(
            "fonction est_pair(x : in entier, k : in entier) renvoie booleen debut renvoie x vaut k; fin",
        )
        .unwrap();
        match program.statements.as_slice() {
            [Statement::Fonction { params, .. }] => assert_eq!(params.len(), 2),
            other => panic!("attendu Fonction, trouvé {other:?}"),
        }
    }

    #[test]
    fn test_parse_marqueur_invalide() {
        let err = parse_source("fonction f(x : vite entier) renvoie entier debut renvoie x; fin")
            .unwrap_err();
        assert_eq!(err.code(), "E101");
    }

    #[test]
    fn test_parse_fin_manquante() {
        let err = parse_source("procedure p() debut afficher(1);").unwrap_err();
        assert_eq!(err.code(), "E102");
    }

    #[test]
    fn test_parse_saisir() {
        let program = parse_source("saisir (n);").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Saisir(vec!["n".to_string()])]
        );
        let program = parse_source("saisir (a, b);").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Saisir(vec!["a".to_string(), "b".to_string()])]
        );
    }

    #[test]
    fn test_parse_appel_procedure() {
        let program = parse_source("incrementer (compteur);").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Appel {
                name: "incrementer".to_string(),
                args: vec![Expr::Ident("compteur".to_string())],
            }]
        );
    }

    #[test]
    fn test_parse_algorithme() {
        let program =
            parse_source("algorithme mon_premier debut afficher(\"Bonjour\"); fin").unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Algorithme {
                name: "mon_premier".to_string(),
                body: vec![Statement::Afficher(Expr::Chaine("Bonjour".to_string()))],
            }]
        );
    }

    #[test]
    fn test_parse_ligne_suivante() {
        let program = parse_source("ligne_suivante;").unwrap();
        assert_eq!(program.statements, vec![Statement::LigneSuivante]);
    }
}
