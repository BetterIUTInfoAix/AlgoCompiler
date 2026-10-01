use crate::errors::Span;

/// Tous les tokens reconnus par le lexer.
///
/// Les mots-clés (y compris les types primitifs) sont reconnus sans tenir
/// compte de la casse : `DECLARER`, `Declarer` et `declarer` produisent le
/// même token. Les identifiants conservent eux leur casse d'origine.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // --- Mots-clés : structure ---
    Afficher, // afficher
    Declarer, // declarer
    Constant, // constante

    // --- Mots-clés : types primitifs ---
    TyEntier,        // entier
    TyEntierNaturel, // entier_naturel
    TyReel,          // reel
    TyBooleen,       // booleen
    TyCaractere,     // caractere
    TyChaine,        // string
    TyTableau,       // tableau_de

    // --- Mots-clés : booléens ---
    Vrai, // vrai
    Faux, // faux

    // --- Mots-clés : conditions ---
    Si,       // si
    Sinon,    // sinon
    SinonSi,  // sinon_si
    FSi,      // fsi
    ChoixSur, // choix_sur
    Entre,    // entre
    Cas,      // cas
    Autre,    // autre
    FChoix,   // fchoix

    // --- Mots-clés : boucles ---
    Boucle,     // boucle
    FBoucle,    // fboucle
    Repeter,    // repeter
    Jusqua,     // jusqua
    TantQue,    // tant_que
    Pour,       // pour
    VariantDe,  // variant_de
    Descendant, // descendant
    Faire,      // faire
    FFaire,     // ffaire
    Sortie,     // sortie
    Continue,   // continue

    // --- Opérateurs logiques ---
    Ou,      // ou
    OuSinon, // ou_sinon
    Et,      // et
    EtAlors, // et_alors
    Non,     // non

    // --- Opérateurs de comparaison ---
    Vaut,      // vaut
    NeVautPas, // ne_vaut_pas
    Lt,        // <
    Gt,        // >
    Le,        // <=
    Ge,        // >=

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
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Minus,     // -
    Plus,      // +
    Star,      // *
    Slash,     // /
    Semicolon, // ;
    Eof,       // fin du fichier
}

impl Token {
    /// Description courte pour les messages d'erreur (avec guillemets français).
    pub fn describe(&self) -> String {
        match self {
            Token::Afficher => "le mot-clé `afficher`".to_string(),
            Token::Declarer => "le mot-clé `declarer`".to_string(),
            Token::Constant => "le mot-clé `constante`".to_string(),
            Token::TyEntier => "le type `entier`".to_string(),
            Token::TyEntierNaturel => "le type `entier_naturel`".to_string(),
            Token::TyReel => "le type `reel`".to_string(),
            Token::TyBooleen => "le type `booleen`".to_string(),
            Token::TyCaractere => "le type `caractere`".to_string(),
            Token::TyChaine => "le type `string`".to_string(),
            Token::TyTableau => "le type `tableau_de`".to_string(),
            Token::Vrai => "le littéral `vrai`".to_string(),
            Token::Faux => "le littéral `faux`".to_string(),
            Token::Si => "le mot-clé `si`".to_string(),
            Token::Sinon => "le mot-clé `sinon`".to_string(),
            Token::SinonSi => "le mot-clé `sinon_si`".to_string(),
            Token::FSi => "le mot-clé `fsi`".to_string(),
            Token::ChoixSur => "le mot-clé `choix_sur`".to_string(),
            Token::Entre => "le mot-clé `entre`".to_string(),
            Token::Cas => "le mot-clé `cas`".to_string(),
            Token::Autre => "le mot-clé `autre`".to_string(),
            Token::FChoix => "le mot-clé `fchoix`".to_string(),
            Token::Boucle => "le mot-clé `boucle`".to_string(),
            Token::FBoucle => "le mot-clé `fboucle`".to_string(),
            Token::Repeter => "le mot-clé `repeter`".to_string(),
            Token::Jusqua => "le mot-clé `jusqua`".to_string(),
            Token::TantQue => "le mot-clé `tant_que`".to_string(),
            Token::Pour => "le mot-clé `pour`".to_string(),
            Token::VariantDe => "le mot-clé `variant_de`".to_string(),
            Token::Descendant => "le mot-clé `descendant`".to_string(),
            Token::Faire => "le mot-clé `faire`".to_string(),
            Token::FFaire => "le mot-clé `ffaire`".to_string(),
            Token::Sortie => "le mot-clé `sortie`".to_string(),
            Token::Continue => "le mot-clé `continue`".to_string(),
            Token::Ou => "l'opérateur `ou`".to_string(),
            Token::OuSinon => "l'opérateur `ou_sinon`".to_string(),
            Token::Et => "l'opérateur `et`".to_string(),
            Token::EtAlors => "l'opérateur `et_alors`".to_string(),
            Token::Non => "l'opérateur `non`".to_string(),
            Token::Vaut => "l'opérateur `vaut`".to_string(),
            Token::NeVautPas => "l'opérateur `ne_vaut_pas`".to_string(),
            Token::Lt => "l'opérateur `<`".to_string(),
            Token::Gt => "l'opérateur `>`".to_string(),
            Token::Le => "l'opérateur `<=`".to_string(),
            Token::Ge => "l'opérateur `>=`".to_string(),
            Token::Ident(name) => format!("l'identifiant `{name}`"),
            Token::IntLit(n) => format!("le nombre `{n}`"),
            Token::FloatLit(f) => format!("le nombre `{f}`"),
            Token::StringLit(s) => format!("la chaîne {s:?}"),
            Token::CharLit(c) => format!("le caractère `{c}`"),
            Token::Assign => "`<-`".to_string(),
            Token::Colon => "`:`".to_string(),
            Token::LParen => "`(`".to_string(),
            Token::RParen => "`)`".to_string(),
            Token::LBracket => "`[`".to_string(),
            Token::RBracket => "`]`".to_string(),
            Token::Comma => "`,`".to_string(),
            Token::Minus => "`-`".to_string(),
            Token::Plus => "`+`".to_string(),
            Token::Star => "`*`".to_string(),
            Token::Slash => "`/`".to_string(),
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
