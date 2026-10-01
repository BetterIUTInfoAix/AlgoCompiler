mod ast;
pub use ast::{BinOp, Expr, Program, Statement, Type, UnOp};

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
    /// (insensible à la casse).
    fn expect_a(&mut self) -> CompileResult<()> {
        let is_a =
            matches!(&self.current().token, Token::Ident(name) if name.eq_ignore_ascii_case("a"));
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
                let span = self.current().span;
                self.parse_affectation(&name, span)
            }
            _ => {
                let current = self.current();
                Err(CompileError::unexpected_token(
                    current.token.describe(),
                    "une instruction (`afficher`, `declarer`, `si`, `boucle`, `tant_que`, `pour`… ou une affectation `nom <- valeur ;`)",
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

    /// `si (cond) ... [sinon_si (cond) ...]* [sinon ...] fsi`
    ///
    /// Les `sinon_si` sont désucrés en `Si` imbriqués dans la branche `sinon`.
    fn parse_si(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `si`
        self.expect(Token::LParen, "`(` après `si`")?;
        let condition = self.parse_expr()?;
        self.expect(Token::RParen, "`)` pour fermer la condition")?;
        let then_branch = self.parse_block_until(
            &[Token::SinonSi, Token::Sinon, Token::FSi],
            "`fsi` pour fermer le bloc `si`",
        )?;
        // Chaîne éventuelle de `sinon_si`.
        let mut else_branch: Option<Vec<Statement>> = None;
        // On collecte les `sinon_si` puis on les replie de droite à gauche.
        let mut sinon_si_chain: Vec<(Expr, Vec<Statement>)> = Vec::new();
        while matches!(self.current().token, Token::SinonSi) {
            self.advance(); // saute `sinon_si`
            self.expect(Token::LParen, "`(` après `sinon_si`")?;
            let cond = self.parse_expr()?;
            self.expect(Token::RParen, "`)` pour fermer la condition")?;
            let body = self.parse_block_until(
                &[Token::SinonSi, Token::Sinon, Token::FSi],
                "`fsi` pour fermer le bloc `sinon_si`",
            )?;
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
        let err = parse_source("si (a vaut b) afficher(a);").unwrap_err();
        assert_eq!(err.code(), "E102");
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
}
