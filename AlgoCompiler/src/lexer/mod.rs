mod token;
pub use token::{SpannedToken, Token};

use crate::errors::{CompileError, CompileResult, Span};

/// Découpe le source en tokens localisés.
///
/// Retourne `Err(CompileError)` au lieu de paniquer :
/// - `E001` caractère inconnu,
/// - `E003` chaîne (guillemets doubles) non fermée,
/// - `E004` caractère (guillemets simples) non fermé,
/// - `E005` contenu entre guillemets simples qui n'est pas un seul caractère,
/// - `E006` nombre entier trop grand.
///
/// Les positions (`line`, `column`) sont comptées en caractères (1-based).
pub fn tokenize(source: &str) -> CompileResult<Vec<SpannedToken>> {
    Lexer::new(source).run()
}

/// Analyseur lexic : état mutable de la progression dans le source.
struct Lexer<'a> {
    /// Source complet, conservé pour calculer le span de fin de fichier.
    source: &'a str,
    chars: Vec<char>,
    /// Index du prochain caractère à lire (en caractères, pas en octets).
    pos: usize,
    line: usize,
    column: usize,
    tokens: Vec<SpannedToken>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Caractère à `offset` positions après le courant (pour les token à 2 caractères).
    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    /// Consomme le caractère courant en maintenant ligne / colonne.
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.pos += 1;
        Some(c)
    }

    /// Pousse un token d'un seul caractère (`(`, `;`, `:` …).
    fn push_simple(&mut self, token: Token) {
        let span = Span::new(self.line, self.column, 1);
        self.bump();
        self.tokens.push(SpannedToken::new(token, span));
    }

    fn run(mut self) -> CompileResult<Vec<SpannedToken>> {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
                continue;
            }
            if c == '/' && self.peek_at(1) == Some('/') {
                self.skip_comment();
                continue;
            }

            match c {
                '(' => self.push_simple(Token::LParen),
                ')' => self.push_simple(Token::RParen),
                ';' => self.push_simple(Token::Semicolon),
                ':' => self.push_simple(Token::Colon),
                '-' => self.push_simple(Token::Minus),
                '<' => self.scan_assign()?,
                '"' => self.scan_string()?,
                '\'' => self.scan_char()?,
                c if c.is_ascii_digit() => self.scan_number()?,
                c if c.is_alphabetic() => self.scan_word(),
                c => {
                    return Err(CompileError::unknown_character(
                        c,
                        Span::new(self.line, self.column, 1),
                    ));
                }
            }
        }

        let eof_span = Span::eof(self.source);
        self.tokens.push(SpannedToken::new(Token::Eof, eof_span));
        Ok(self.tokens)
    }

    /// Ignore une fin de ligne après `//` (le retour à la ligne lui-même est
    /// laissé à la gestion des espaces, pour ne pas fausser le comptage).
    fn skip_comment(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.bump();
        }
    }

    /// `<-` : affectation. Un `<` suivi d'autre chose est une erreur.
    fn scan_assign(&mut self) -> CompileResult<()> {
        let (line, column) = (self.line, self.column);
        if self.peek_at(1) != Some('-') {
            return Err(CompileError::unknown_character(
                '<',
                Span::new(line, column, 1),
            ));
        }
        self.bump();
        self.bump();
        self.tokens
            .push(SpannedToken::new(Token::Assign, Span::new(line, column, 2)));
        Ok(())
    }

    /// Chaîne entre guillemets doubles, fermée sur la même ligne.
    fn scan_string(&mut self) -> CompileResult<()> {
        let (start_line, start_col) = (self.line, self.column);
        self.bump(); // saute `"` ouvrant
        let mut content = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => {
                    let length = self.column.saturating_sub(start_col) + 1;
                    return Err(CompileError::unterminated_string(Span::new(
                        start_line,
                        start_col,
                        length.max(1),
                    )));
                }
                Some('"') => {
                    self.bump(); // saute `"` fermant
                    let length = content.chars().count() + 2; // guillemets inclus
                    let span = Span::new(start_line, start_col, length);
                    self.tokens
                        .push(SpannedToken::new(Token::StringLit(content), span));
                    return Ok(());
                }
                Some(c) => {
                    content.push(c);
                    self.bump();
                }
            }
        }
    }

    /// Caractère entre guillemets simples, fermé sur la même ligne.
    ///
    /// Les séquences d'échappement de base sont décodées : `\n`, `\t`, `\r`,
    /// `\\`, `\'`, `\"`.
    fn scan_char(&mut self) -> CompileResult<()> {
        let (start_line, start_col) = (self.line, self.column);
        self.bump(); // saute `'` ouvrant
        let mut raw = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => {
                    let length = self.column.saturating_sub(start_col) + 1;
                    return Err(CompileError::unterminated_char(Span::new(
                        start_line,
                        start_col,
                        length.max(1),
                    )));
                }
                Some('\'') => break,
                Some(c) => {
                    raw.push(c);
                    self.bump();
                }
            }
        }
        self.bump(); // saute `'` fermant
        let length = raw.chars().count() + 2;
        let span = Span::new(start_line, start_col, length);

        let decoded = decode_escapes(&raw);
        let mut chars = decoded.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => {
                self.tokens.push(SpannedToken::new(Token::CharLit(c), span));
                Ok(())
            }
            _ => Err(CompileError::invalid_char(raw, span)),
        }
    }

    /// Nombre entier ou réel (au moins un chiffre après le `.`).
    fn scan_number(&mut self) -> CompileResult<()> {
        let (start_line, start_col) = (self.line, self.column);
        let mut text = String::new();
        self.eat_digits(&mut text);

        let is_float =
            self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit());
        if is_float {
            text.push('.');
            self.bump();
            self.eat_digits(&mut text);
        }

        let span = Span::new(start_line, start_col, text.chars().count());
        let token = if is_float {
            let value = text
                .parse::<f64>()
                .map_err(|_| CompileError::number_too_large(text.clone(), span))?;
            Token::FloatLit(value)
        } else {
            match text.parse::<i64>() {
                Ok(value) => Token::IntLit(value),
                Err(_) => return Err(CompileError::number_too_large(text, span)),
            }
        };

        self.tokens.push(SpannedToken::new(token, span));
        Ok(())
    }

    fn eat_digits(&mut self, text: &mut String) {
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            text.push(c);
            self.bump();
        }
    }

    /// Mot-clé ou identifiant (insensible à la casse pour les mots-clés).
    ///
    /// À tenir synchronisé avec `KEYWORDS` dans `crate::errors` (suggestions).
    fn scan_word(&mut self) {
        let (start_line, start_col) = (self.line, self.column);
        let mut word = String::new();
        while let Some(c) = self.peek() {
            if !(c.is_alphanumeric() || c == '_') {
                break;
            }
            word.push(c);
            self.bump();
        }
        let span = Span::new(start_line, start_col, word.chars().count());
        let token = match word.to_lowercase().as_str() {
            "afficher" => Token::Afficher,
            "declarer" => Token::Declarer,
            "entier" => Token::TyEntier,
            "entier_naturel" => Token::TyEntierNaturel,
            "reel" => Token::TyReel,
            "booleen" => Token::TyBooleen,
            "caractere" => Token::TyCaractere,
            "string" => Token::TyChaine,
            "vrai" => Token::Vrai,
            "faux" => Token::Faux,
            _ => Token::Ident(word),
        };
        self.tokens.push(SpannedToken::new(token, span));
    }
}

/// Décode les séquences d'échappement simples d'un caractère.
///
/// Une échappement inconnu est conservé tel quel (il sera rejeté par la
/// validation « un seul caractère » du lexer).
fn decode_escapes(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some('\'') => out.push('\''),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
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
    fn test_declarer_et_affectation() {
        let tokens = tokenize("declarer x : entier; x <- -5;").unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                Token::Declarer,
                Token::Ident("x".to_string()),
                Token::Colon,
                Token::TyEntier,
                Token::Semicolon,
                Token::Ident("x".to_string()),
                Token::Assign,
                Token::Minus,
                Token::IntLit(5),
                Token::Semicolon,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_mots_cles_insensibles_a_la_casse() {
        let tokens = tokenize("DECLARER x : Entier_Naturel;").unwrap();
        assert_eq!(kinds(&tokens)[0], Token::Declarer);
        assert_eq!(kinds(&tokens)[3], Token::TyEntierNaturel);
        // L'identifiant conserve sa casse d'origine.
        assert_eq!(kinds(&tokens)[1], Token::Ident("x".to_string()));
    }

    #[test]
    fn test_litteraux_scalaires() {
        let tokens = tokenize(r#"b <- vrai; f <- faux; r <- 5.61; c <- 'e';"#).unwrap();
        assert!(kinds(&tokens).contains(&Token::Vrai));
        assert!(kinds(&tokens).contains(&Token::Faux));
        assert!(kinds(&tokens).contains(&Token::FloatLit(5.61)));
        assert!(kinds(&tokens).contains(&Token::CharLit('e')));
    }

    #[test]
    fn test_commentaire_ignore() {
        // Deux tokens : l'`afficher` puis son span commence en ligne 2.
        let tokens = tokenize("// un commentaire\nafficher(\"a\");").unwrap();
        assert_eq!(tokens[0].token, Token::Afficher);
        assert_eq!((tokens[0].span.line, tokens[0].span.column), (2, 1));
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

    #[test]
    fn test_guillemet_simple_non_ferme() {
        let err = tokenize("x <- 'a;").unwrap_err();
        assert_eq!(err.code(), "E004");
    }

    #[test]
    fn test_plusieurs_caracteres_entre_guillemets() {
        let err = tokenize("x <- 'ab';").unwrap_err();
        assert_eq!(err.code(), "E005");
    }

    #[test]
    fn test_caractere_vide() {
        let err = tokenize("x <- '';").unwrap_err();
        assert_eq!(err.code(), "E005");
    }

    #[test]
    fn test_caractere_avec_echappement() {
        let tokens = tokenize(r#"c <- '\n';"#).unwrap();
        assert!(kinds(&tokens).contains(&Token::CharLit('\n')));
    }

    #[test]
    fn test_entier_trop_grand() {
        let err = tokenize("x <- 99999999999999999999;").unwrap_err();
        assert_eq!(err.code(), "E006");
    }
}
