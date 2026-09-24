use crate::errors::Span;

/// Tous les tokens reconnus par le lexer.
///
/// Les mots-clés (y compris les types primitifs) sont reconnus sans tenir
/// compte de la casse : `DECLARER`, `Declarer` et `declarer` produisent le
/// même token. Les identifiants conservent eux leur casse d'origine.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // --- Mots-clés ---
    Afficher,        // afficher
    Declarer,        // declarer
    TyEntier,        // entier
    TyEntierNaturel, // entier_naturel
    TyReel,          // reel
    TyBooleen,       // booleen
    TyCaractere,     // caractere
    TyChaine,        // string
    Vrai,            // vrai
    Faux,            // faux

    // --- Identifiants et littéraux ---
    Ident(String),     // nom de variable
    IntLit(i64),       // 42, -5 (le `-` est un token distinct)
    FloatLit(f64),     // 3.14
    StringLit(String), // le texte entre guillemets doubles
    CharLit(char),     // le caractère entre guillemets simples

    // --- Ponctuation ---
    Assign,    // <-
    Colon,     // :
    LParen,    // (
    RParen,    // )
    Minus,     // -
    Semicolon, // ;
    Eof,       // fin du fichier
}

impl Token {
    /// Description courte pour les messages d'erreur (avec guillemets français).
    pub fn describe(&self) -> String {
        match self {
            Token::Afficher => "le mot-clé `afficher`".to_string(),
            Token::Declarer => "le mot-clé `declarer`".to_string(),
            Token::TyEntier => "le type `entier`".to_string(),
            Token::TyEntierNaturel => "le type `entier_naturel`".to_string(),
            Token::TyReel => "le type `reel`".to_string(),
            Token::TyBooleen => "le type `booleen`".to_string(),
            Token::TyCaractere => "le type `caractere`".to_string(),
            Token::TyChaine => "le type `string`".to_string(),
            Token::Vrai => "le littéral `vrai`".to_string(),
            Token::Faux => "le littéral `faux`".to_string(),
            Token::Ident(name) => format!("l'identifiant `{name}`"),
            Token::IntLit(n) => format!("le nombre `{n}`"),
            Token::FloatLit(f) => format!("le nombre `{f}`"),
            Token::StringLit(s) => format!("la chaîne {s:?}"),
            Token::CharLit(c) => format!("le caractère `{c}`"),
            Token::Assign => "`<-`".to_string(),
            Token::Colon => "`:`".to_string(),
            Token::LParen => "`(`".to_string(),
            Token::RParen => "`)`".to_string(),
            Token::Minus => "`-`".to_string(),
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
