mod token;
pub use token::Token;

pub fn tokenize(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c == '(' {
            tokens.push(Token::LParen);
            i += 1;
        } else if c == ')' {
            tokens.push(Token::RParen);
            i += 1;
        } else if c == ';' {
            tokens.push(Token::Semicolon);
            i += 1;
        } else if c == '"' {
            // on lit jusqu'au prochain guillemet
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            tokens.push(Token::StringLit(s));
            i += 1; // on saute le guillemet fermant
        } else if c.is_alphabetic() {
            // on lit un mot entier (identifiant ou mot-clé)
            let start = i;
            while i < chars.len() && chars[i].is_alphanumeric() {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            match word.as_str() {
                "afficher" => tokens.push(Token::Afficher),
                _ => panic!("mot inconnu: {}", word), // on améliorera ça avec errors.rs plus tard
            }
        } else {
            panic!("caractère inconnu: {}", c);
        }
    }

    tokens.push(Token::Eof);
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_afficher_simple() {
        let tokens = tokenize(r#"afficher("salut");"#);
        assert_eq!(
            tokens,
            vec![
                Token::Afficher,
                Token::LParen,
                Token::StringLit("salut".to_string()),
                Token::RParen,
                Token::Semicolon,
                Token::Eof,
            ]
        );
    }
}