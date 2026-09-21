use crate::errors::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Afficher,          // le mot-clé "afficher"
    StringLit(String), // le texte entre guillemets
    LParen,            // (
    RParen,            // )
    Semicolon,         // ;
    Eof,               // fin du fichier
}

impl Token {
    /// Description courte pour les messages d'erreur (avec guillemets français).
    pub fn describe(&self) -> String {
        match self {
            Token::Afficher => "le mot-clé `afficher`".to_string(),
            Token::StringLit(s) => format!("la chaîne {s:?}"),
            Token::LParen => "`(`".to_string(),
            Token::RParen => "`)`".to_string(),
            Token::Semicolon => "`;`".to_string(),
            Token::Eof => "la fin du fichier".to_string(),
        }
    }
}

/// Token localisé : le token brut plus sa position dans le source.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

impl SpannedToken {
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}
