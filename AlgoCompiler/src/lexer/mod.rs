mod token;
pub use token::{SpannedToken, Token};

use crate::errors::{CompileError, CompileResult, Span};

/// Découpe le source en tokens localisés.
///
/// Retourne `Err(CompileError)` au lieu de paniquer :
/// - `E001` caractère inconnu,
/// - `E002` mot inconnu (avec suggestion `afficher` si proche),
/// - `E003` chaîne non fermée.
///
/// Les positions (`line`, `column`) sont comptées en caractères (1-based).
pub fn tokenize(source: &str) -> CompileResult<Vec<SpannedToken>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut line = 1usize;
    let mut column = 1usize;

    // Avance de `n` caractères en maintenant ligne / colonne.
    let advance = |n: usize, i: &mut usize, line: &mut usize, column: &mut usize| {
        for _ in 0..n {
            if *i >= chars.len() {
                break;
            }
            if chars[*i] == '\n' {
                *line += 1;
                *column = 1;
            } else {
                *column += 1;
            }
            *i += 1;
        }
    };

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            advance(1, &mut i, &mut line, &mut column);
            continue;
        }

        let start_line = line;
        let start_col = column;

        if c == '(' {
            tokens.push(SpannedToken::new(
                Token::LParen,
                Span::new(start_line, start_col, 1),
            ));
            advance(1, &mut i, &mut line, &mut column);
        } else if c == ')' {
            tokens.push(SpannedToken::new(
                Token::RParen,
                Span::new(start_line, start_col, 1),
            ));
            advance(1, &mut i, &mut line, &mut column);
        } else if c == ';' {
            tokens.push(SpannedToken::new(
                Token::Semicolon,
                Span::new(start_line, start_col, 1),
            ));
            advance(1, &mut i, &mut line, &mut column);
        } else if c == '"' {
            // Chaîne : on cherche le guillemet fermant sur la même ligne.
            advance(1, &mut i, &mut line, &mut column); // saute `"`
            let start = i;
            while i < chars.len() && chars[i] != '"' && chars[i] != '\n' {
                i += 1;
                column += 1;
            }
            if i >= chars.len() || chars[i] != '"' {
                // Fin de ligne / fichier atteinte sans fermeture.
                let length = i.saturating_sub(start) + 1;
                return Err(CompileError::unterminated_string(Span::new(
                    start_line,
                    start_col,
                    length.max(1),
                )));
            }
            let s: String = chars[start..i].iter().collect();
            let length = i.saturating_sub(start) + 2; // guillemets inclus
            tokens.push(SpannedToken::new(
                Token::StringLit(s),
                Span::new(start_line, start_col, length),
            ));
            advance(1, &mut i, &mut line, &mut column); // saute `"` fermant
        } else if c.is_alphabetic() {
            let start = i;
            let start_col_copy = column;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
                column += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let length = i.saturating_sub(start);
            let span = Span::new(start_line, start_col_copy, length);
            match word.as_str() {
                "afficher" => tokens.push(SpannedToken::new(Token::Afficher, span)),
                _ => return Err(CompileError::unknown_word(word, span)),
            }
        } else {
            return Err(CompileError::unknown_character(
                c,
                Span::new(start_line, start_col, 1),
            ));
        }
    }

    let eof_span = Span::eof(source);
    tokens.push(SpannedToken::new(Token::Eof, eof_span));
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(tokens: &[SpannedToken]) -> Vec<Token> {
        tokens.iter().map(|t| t.token.clone()).collect()
    }

    #[test]
    fn test_afficher_simple() {
        let tokens = tokenize(r#"afficher("salut");"#).expect("devrait lexer sans erreur");
        assert_eq!(
            kinds(&tokens),
            vec![
                Token::Afficher,
                Token::LParen,
                Token::StringLit("salut".to_string()),
                Token::RParen,
                Token::Semicolon,
                Token::Eof,
            ]
        );
        // La position du premier token est 1:1.
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.column, 1);
    }

    #[test]
    fn test_positions_multilignes() {
        let tokens = tokenize("afficher(\"a\");\nafficher(\"b\");\n").unwrap();
        // Le 2e `afficher` est ligne 2, colonne 1.
        assert_eq!(tokens[5].token, Token::Afficher);
        assert_eq!((tokens[5].span.line, tokens[5].span.column), (2, 1));
    }

    #[test]
    fn test_mot_inconnu_avec_suggestion() {
        let err = tokenize("affichr(\"x\");").unwrap_err();
        assert_eq!(err.code(), "E002");
        assert!(err.hint.as_ref().unwrap().contains("afficher"));
    }

    #[test]
    fn test_caractere_inconnu() {
        let err = tokenize("afficher(\"x\"); @").unwrap_err();
        assert_eq!(err.code(), "E001");
        assert_eq!(err.span.unwrap().line, 1);
    }

    #[test]
    fn test_chaine_non_fermee() {
        let err = tokenize("afficher(\"oups);").unwrap_err();
        assert_eq!(err.code(), "E003");
    }
}
