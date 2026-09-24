mod ast;
pub use ast::{Expr, Program, Statement, Type};

use crate::errors::{CompileError, CompileResult, Span};
use crate::lexer::{SpannedToken, Token};

/// Libellé attendu quand un type primitif est requis.
const TYPE_EXPECTED: &str =
    "un type primitif (`entier`, `entier_naturel`, `reel`, `booleen`, `caractere` ou `string`)";

/// Libellé attendu quand une expression est requise.
const EXPR_EXPECTED: &str = "une expression";
const EXPR_HINT: &str = "exemples d'expressions : `42`, `3.14`, `\"texte\"`, `'c'`, `vrai`, `faux` ou le nom d'une variable";

/// Analyseur récursif descendant sur une suite de tokens localisés.
///
/// Plus aucun `panic!` / `assert!` : chaque anomalie devient une
/// [`CompileError`] localisée (`E101` token inattendu, `E102` fin prématurée).
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

    fn parse_program(mut self) -> CompileResult<Program> {
        let mut statements = Vec::new();
        while self.current().token != Token::Eof {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> CompileResult<Statement> {
        if matches!(self.current().token, Token::Afficher) {
            return self.parse_afficher();
        }
        if matches!(self.current().token, Token::Declarer) {
            return self.parse_declarer();
        }

        // Une instruction peut aussi être une affectation : `nom <- valeur ;`.
        let ident = match &self.current().token {
            Token::Ident(name) => Some(name.clone()),
            _ => None,
        };
        if let Some(name) = ident {
            let span = self.current().span;
            return self.parse_affectation(&name, span);
        }

        let current = self.current();
        Err(CompileError::unexpected_token(
            current.token.describe(),
            "le mot-clé `afficher`, le mot-clé `declarer` ou une affectation (`nom <- valeur ;`)",
            current.span,
        )
        .with_hint(
            "chaque instruction commence par `afficher(...)`, `declarer nom : type ;` ou `nom <- valeur ;`",
        ))
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

    /// `declarer nom : type ;`
    fn parse_declarer(&mut self) -> CompileResult<Statement> {
        self.advance(); // saute `declarer`
        let name = self.expect_ident("le nom d'une variable")?;
        self.expect(Token::Colon, "`:` après le nom de la variable")?;
        let ty = self.parse_type()?;
        self.expect(Token::Semicolon, "`;` en fin de déclaration")?;
        Ok(Statement::Declarer { name, ty })
    }

    /// `nom <- expr ;`
    ///
    /// `name` / `span` désignent l'identifiant en tête d'instruction.
    fn parse_affectation(&mut self, name: &str, span: Span) -> CompileResult<Statement> {
        self.advance(); // saute l'identifiant
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

    fn parse_type(&mut self) -> CompileResult<Type> {
        let current = self.current();
        let ty = match &current.token {
            Token::TyEntier => Type::Entier,
            Token::TyEntierNaturel => Type::EntierNaturel,
            Token::TyReel => Type::Reel,
            Token::TyBooleen => Type::Booleen,
            Token::TyCaractere => Type::Caractere,
            Token::TyChaine => Type::Chaine,
            Token::Eof => {
                return Err(CompileError::unexpected_eof(TYPE_EXPECTED, current.span)
                    .with_hint("déclaration incomplète — il manque le type de la variable"));
            }
            other => {
                return Err(CompileError::unexpected_token(
                    other.describe(),
                    TYPE_EXPECTED,
                    current.span,
                )
                .with_hint("exemple : declarer unEntier : entier ;"));
            }
        };
        self.advance();
        Ok(ty)
    }

    /// Expression : pour l'instant uniquement un terme unaire
    /// (littéral, variable ou négation `-`).
    fn parse_expr(&mut self) -> CompileResult<Expr> {
        if matches!(self.current().token, Token::Minus) {
            self.advance();
            let operand = self.parse_expr()?;
            return Ok(Expr::Neg(Box::new(operand)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> CompileResult<Expr> {
        let current = self.current();
        let expr = match &current.token {
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
        ] {
            let program = parse_source(decl).unwrap();
            assert!(
                matches!(program.statements.as_slice(), [Statement::Declarer { .. }]),
                "{decl}"
            );
        }
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
                value: Expr::Neg(Box::new(Expr::Entier(5))),
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
        assert!(err.kind.message().contains("type primitif"));
    }

    #[test]
    fn test_affectation_sans_valeur() {
        let err = parse_source("x <- ;").unwrap_err();
        assert_eq!(err.code(), "E101");
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
