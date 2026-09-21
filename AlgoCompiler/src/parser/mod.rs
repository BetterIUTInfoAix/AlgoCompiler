mod ast;
pub use ast::{Program, Statement};

use crate::errors::{CompileError, CompileResult, Span};
use crate::lexer::{SpannedToken, Token};

/// Analyse une suite de tokens localisés en [`Program`].
///
/// Plus aucun `panic!` / `assert!` : chaque anomalie devient une
/// [`CompileError`] localisée (`E101` token inattendu, `E102` fin prématurée).
pub fn parse(tokens: &[SpannedToken]) -> CompileResult<Program> {
    let mut statements = Vec::new();
    let mut i = 0;

    // Token Eof synthétique si la liste est vide (ne devrait pas arriver
    // avec le lexer, mais évite un accès hors bornes).
    let eof = SpannedToken::new(Token::Eof, Span::new(1, 1, 1));
    let at = |i: usize| -> &SpannedToken { tokens.get(i).unwrap_or(&eof) };

    while at(i).token != Token::Eof {
        match &at(i).token {
            Token::Afficher => {
                let stmt_span = at(i).span;
                i += 1; // saute `afficher`
                expect(at(i), Token::LParen, "`(` après `afficher`")?;
                i += 1;
                let current = at(i);
                let text = match &current.token {
                    Token::StringLit(s) => s.clone(),
                    Token::Eof => {
                        return Err(CompileError::unexpected_eof(
                            "une chaîne entre guillemets",
                            current.span,
                        )
                        .with_hint("exemple : afficher(\"texte\");"));
                    }
                    other => {
                        return Err(CompileError::unexpected_token(
                            other.describe(),
                            "une chaîne entre guillemets",
                            current.span,
                        )
                        .with_hint("exemple : afficher(\"texte\");"));
                    }
                };
                i += 1;
                expect(at(i), Token::RParen, "`)` pour fermer l'appel")?;
                i += 1;
                expect(at(i), Token::Semicolon, "`;` en fin d'instruction")?;
                i += 1;

                let _ = stmt_span;
                statements.push(Statement::Afficher(text));
            }
            Token::Eof => break,
            other => {
                let span = at(i).span;
                return Err(CompileError::unexpected_token(
                    other.describe(),
                    "le mot-clé `afficher`",
                    span,
                )
                .with_hint("chaque instruction doit commencer par `afficher(\"...\");`"));
            }
        }
    }

    Ok(Program { statements })
}

/// Vérifie que `actual` correspond au discriminant de `expected_kind`.
/// `expected_label` est le texte affiché à l'utilisateur (ex : "`;` en fin…").
fn expect(actual: &SpannedToken, expected_kind: Token, expected_label: &str) -> CompileResult<()> {
    if discriminants_eq(&actual.token, &expected_kind) {
        return Ok(());
    }
    if actual.token == Token::Eof {
        return Err(
            CompileError::unexpected_eof(expected_label, actual.span).with_hint(format!(
                "instruction incomplète — il manque {expected_label}"
            )),
        );
    }
    Err(CompileError::unexpected_token(
        actual.token.describe(),
        expected_label.to_string(),
        actual.span,
    ))
}

/// Compare deux tokens sans tenir compte du contenu de `StringLit`.
fn discriminants_eq(a: &Token, b: &Token) -> bool {
    matches!(
        (a, b),
        (Token::Afficher, Token::Afficher)
            | (Token::LParen, Token::LParen)
            | (Token::RParen, Token::RParen)
            | (Token::Semicolon, Token::Semicolon)
            | (Token::Eof, Token::Eof)
            | (Token::StringLit(_), Token::StringLit(_))
    )
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
    fn test_parse_afficher() {
        let program = parse_source(r#"afficher("salut");"#).unwrap();
        assert_eq!(
            program.statements,
            vec![Statement::Afficher("salut".to_string())]
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
    fn test_chaine_manquante() {
        let err = parse_source("afficher();").unwrap_err();
        assert_eq!(err.code(), "E101");
        assert!(err.kind.message().contains("chaîne"));
    }
}
