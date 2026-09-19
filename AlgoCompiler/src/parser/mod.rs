mod ast;
pub use ast::{Program, Statement};

use crate::lexer::Token;

pub fn parse(tokens: &[Token]) -> Program {
    let mut statements = Vec::new();
    let mut i = 0;

    while i < tokens.len() && tokens[i] != Token::Eof {
        match &tokens[i] {
            Token::Afficher => {
                i += 1; // saute Afficher
                assert_eq!(tokens[i], Token::LParen);
                i += 1;
                let text = match &tokens[i] {
                    Token::StringLit(s) => s.clone(),
                    _ => panic!("attendu une chaîne de caractères"),
                };
                i += 1;
                assert_eq!(tokens[i], Token::RParen);
                i += 1;
                assert_eq!(tokens[i], Token::Semicolon);
                i += 1;

                statements.push(Statement::Afficher(text));
            }
            _ => panic!("token inattendu: {:?}", tokens[i]),
        }
    }

    Program { statements }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_parse_afficher() {
        let tokens = tokenize(r#"afficher("salut");"#);
        let program = parse(&tokens);
        assert_eq!(program.statements, vec![Statement::Afficher("salut".to_string())]);
    }
}