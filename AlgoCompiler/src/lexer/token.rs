#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Afficher,           // le mot-clé "afficher"
    StringLit(String),  // le texte entre guillemets
    LParen,             // (
    RParen,             // )
    Semicolon,          // ;
    Eof,                // fin du fichier
}