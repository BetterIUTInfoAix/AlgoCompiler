//! Système centralisé de gestion des erreurs du compilateur.
//!
//! Objectifs :
//! - ne plus utiliser `panic!` / `assert!` dans le lexer et le parser ;
//! - localiser chaque erreur (ligne / colonne) ;
//! - afficher un extrait de code avec un curseur `^^^` et une suggestion ;
//! - exposer un type unique [`CompileError`] utilisable avec `Result`.

use std::fmt;

/// Position d'un token ou d'une erreur dans le source.
///
/// `line` et `column` sont indexés à partir de 1 (convention éditeurs).
/// `length` est exprimée en caractères (pas en octets) pour l'affichage `^^^`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

impl Span {
    pub fn new(line: usize, column: usize, length: usize) -> Self {
        Self {
            line: line.max(1),
            column: column.max(1),
            length: length.max(1),
        }
    }

    /// Position de fin de fichier (utilisée pour `Eof`).
    pub fn eof(source: &str) -> Self {
        let line = source.lines().count().max(1);
        let column = source
            .lines()
            .last()
            .map(|l| l.chars().count() + 1)
            .unwrap_or(1);
        Self::new(line, column, 1)
    }
}

/// Catégorie d'erreur. Chaque variante possède un code stable `E0xx`/`E1xx`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// Caractère non reconnu par le lexer. Ex : `@`, `#`, `&`.
    UnknownCharacter(char),
    /// Mot non reconnu (ni mot-clé, ni identifiant valide ici). Ex : `affichr`.
    UnknownWord(String),
    /// Chaîne ouverte avec `"` mais jamais fermée.
    UnterminatedString,
    /// Token inattendu côté parser.
    UnexpectedToken { found: String, expected: String },
    /// Fin de fichier prématurée (il manquait quelque chose).
    UnexpectedEof { expected: String },
    /// Échec d'entrée/sortie (lecture du fichier source).
    Io(String),
}

impl ErrorKind {
    /// Code stable pour la documentation et les tests.
    pub fn code(&self) -> &'static str {
        match self {
            ErrorKind::UnknownCharacter(_) => "E001",
            ErrorKind::UnknownWord(_) => "E002",
            ErrorKind::UnterminatedString => "E003",
            ErrorKind::UnexpectedToken { .. } => "E101",
            ErrorKind::UnexpectedEof { .. } => "E102",
            ErrorKind::Io(_) => "E201",
        }
    }

    /// Phase de compilation concernée.
    pub fn phase(&self) -> &'static str {
        match self {
            ErrorKind::UnknownCharacter(_)
            | ErrorKind::UnknownWord(_)
            | ErrorKind::UnterminatedString => "lexique",
            ErrorKind::UnexpectedToken { .. } | ErrorKind::UnexpectedEof { .. } => "syntaxe",
            ErrorKind::Io(_) => "entrée/sortie",
        }
    }

    /// Message principal (sans localisation ni aide).
    pub fn message(&self) -> String {
        match self {
            ErrorKind::UnknownCharacter(c) => format!("caractère inconnu {c:?}"),
            ErrorKind::UnknownWord(w) => format!("mot inconnu {w:?}"),
            ErrorKind::UnterminatedString => {
                "chaîne de caractères non fermée (guillemet `\"` manquant)".to_string()
            }
            ErrorKind::UnexpectedToken { found, expected } => {
                format!("token inattendu {found}, attendu {expected}")
            }
            ErrorKind::UnexpectedEof { expected } => {
                format!("fin de fichier inattendue, attendu {expected}")
            }
            ErrorKind::Io(msg) => msg.to_string(),
        }
    }
}

/// Erreur de compilation complète : nature + localisation + aide optionnelle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub kind: ErrorKind,
    pub span: Option<Span>,
    pub hint: Option<String>,
}

impl CompileError {
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            span: None,
            hint: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        let h = hint.into();
        if !h.is_empty() {
            self.hint = Some(h);
        }
        self
    }

    // --- Constructeurs pratiques ---

    pub fn unknown_character(c: char, span: Span) -> Self {
        Self::new(ErrorKind::UnknownCharacter(c))
            .with_span(span)
            .with_hint(
                "caractères attendus ici : `(`, `)`, `;`, `\"...\"` ou le mot-clé `afficher`",
            )
    }

    pub fn unknown_word(word: impl Into<String>, span: Span) -> Self {
        let word = word.into();
        let mut err = Self::new(ErrorKind::UnknownWord(word.clone())).with_span(span);
        if let Some(suggestion) = suggest_keyword(&word) {
            err = err.with_hint(format!("vouliez-vous dire `{suggestion}` ?"));
        } else {
            err = err.with_hint("seul le mot-clé `afficher` est supporté pour le moment");
        }
        err
    }

    pub fn unterminated_string(span: Span) -> Self {
        Self::new(ErrorKind::UnterminatedString)
            .with_span(span)
            .with_hint("fermez la chaîne avec un guillemet `\"` sur la même ligne")
    }

    pub fn unexpected_token(
        found: impl Into<String>,
        expected: impl Into<String>,
        span: Span,
    ) -> Self {
        Self::new(ErrorKind::UnexpectedToken {
            found: found.into(),
            expected: expected.into(),
        })
        .with_span(span)
    }

    pub fn unexpected_eof(expected: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::UnexpectedEof {
            expected: expected.into(),
        })
        .with_span(span)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Io(message.into()))
    }

    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// Localisation courte `ligne:colonne`, ou `None` si non localisée.
    pub fn location(&self) -> Option<String> {
        self.span.map(|s| format!("{}:{}", s.line, s.column))
    }

    /// Rendu une ligne, sans extrait de code (utile pour `Display` / logs).
    pub fn short(&self, filename: Option<&str>) -> String {
        let file = filename.unwrap_or("<source>");
        match self.span {
            Some(s) => format!(
                "[{}] erreur de {} à {}:{}:{} : {}{}",
                self.code(),
                self.kind.phase(),
                file,
                s.line,
                s.column,
                self.kind.message(),
                self.hint
                    .as_ref()
                    .map(|h| format!(" — aide : {h}"))
                    .unwrap_or_default(),
            ),
            None => format!(
                "[{}] erreur de {} : {}{}",
                self.code(),
                self.kind.phase(),
                self.kind.message(),
                self.hint
                    .as_ref()
                    .map(|h| format!(" — aide : {h}"))
                    .unwrap_or_default(),
            ),
        }
    }

    /// Rendu complet avec extrait de la ligne fautive et curseur `^^^`.
    ///
    /// Exemple :
    /// ```text
    /// Erreur E002 à hello.algo:1:1 : mot inconnu "affichr"
    ///   |
    /// 1 | affichr("salut");
    ///   | ^^^^^^^ vouliez-vous dire `afficher` ?
    /// ```
    pub fn render(&self, source: &str, filename: Option<&str>) -> String {
        let mut out = String::from("Erreur ");
        out.push_str(&self.short(filename));

        let Some(span) = self.span else {
            return out;
        };
        let Some(line_text) = source.lines().nth(span.line.saturating_sub(1)) else {
            return out;
        };

        // Largeur du numéro de ligne pour aligner les `|`.
        let gutter = span.line.to_string().len().max(1);
        out.push('\n');
        out.push_str(&format!("{:>width$} |\n", "", width = gutter));
        out.push_str(&format!(
            "{:>width$} | {line_text}\n",
            span.line,
            width = gutter
        ));

        // Le décalage du curseur est calculé en caractères pour supporter l'UTF-8.
        let prefix_chars = line_text.chars().count();
        let start = (span.column.saturating_sub(1)).min(prefix_chars);
        let carets = "^".repeat(span.length.max(1));
        let hint = self
            .hint
            .as_ref()
            .map(|h| format!(" {h}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "{:>width$} | {:<pad$}{carets}{hint}",
            "",
            "",
            width = gutter,
            pad = start,
        ));
        out
    }

    /// Affiche l'erreur sur `stderr` avec le contexte source.
    pub fn report(&self, source: &str, filename: Option<&str>) {
        eprintln!("{}", self.render(source, filename));
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short(None))
    }
}

impl std::error::Error for CompileError {}

/// Alias pratique pour les fonctions du compilateur.
pub type CompileResult<T> = Result<T, CompileError>;

// --- Suggestions de mots-clés (sans dépendance externe) ---

const KEYWORDS: &[&str] = &["afficher"];

/// Retourne le mot-clé le plus proche si la distance d'édition est petite.
pub fn suggest_keyword(word: &str) -> Option<&'static str> {
    let mut best: Option<(&'static str, usize)> = None;
    for &kw in KEYWORDS {
        let d = levenshtein(word, kw);
        // Seuil : 2 erreurs max, ou 1/3 du mot pour les mots un peu plus longs.
        let threshold = 2.max(kw.len() / 3);
        if d <= threshold && best.map(|(_, b)| d < b).unwrap_or(true) {
            best = Some((kw, d));
        }
    }
    best.map(|(kw, _)| kw)
}

/// Distance de Levenshtein simple sur les `char` (suffit pour des mots-clés courts).
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur.push((prev[j] + cost).min((cur[j] + 1).min(prev[j + 1] + 1)));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_contient_code_et_position() {
        let err = CompileError::unknown_character('@', Span::new(2, 5, 1));
        let s = err.short(Some("test.algo"));
        assert!(s.contains("E001"), "{s}");
        assert!(s.contains("2:5"), "{s}");
    }

    #[test]
    fn render_affiche_extrait_et_curseur() {
        let source = "afficher(\"ok\");\naffichr(\"ko\");\n";
        let err = CompileError::unknown_word("affichr", Span::new(2, 1, 7));
        let rendered = err.render(source, Some("test.algo"));
        assert!(rendered.contains("2 | affichr"), "{rendered}");
        assert!(rendered.contains("^^^^^^^"), "{rendered}");
        assert!(rendered.contains("afficher"), "{rendered}");
    }

    #[test]
    fn render_sans_span_ne_panique_pas() {
        let err = CompileError::io("fichier introuvable");
        assert!(err.render("vide", None).contains("E201"));
    }

    #[test]
    fn suggestion_proche() {
        assert_eq!(suggest_keyword("affichr"), Some("afficher"));
        assert_eq!(suggest_keyword("xyz"), None);
    }

    #[test]
    fn eof_span_est_coherent() {
        let span = Span::eof("a\nb");
        assert_eq!((span.line, span.column), (2, 2));
    }
}
