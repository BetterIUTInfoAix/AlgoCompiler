/// Type primitif d'une variable.
///
/// Seuls les types scalaires sont supportés pour l'instant : les tableaux
/// (`tableau_de`) et les constantes (`constante`) restent à implémenter.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Entier,
    EntierNaturel,
    Reel,
    Booleen,
    Caractere,
    Chaine,
}

/// Expression : littéral, variable ou négation unaire.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Entier(i64),
    Reel(f64),
    Chaine(String),
    Caractere(char),
    Booleen(bool),
    Ident(String),
    Neg(Box<Expr>),
}

/// Instruction algorithmique.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// `afficher(expr);`
    Afficher(Expr),
    /// `declarer nom : type;`
    Declarer { name: String, ty: Type },
    /// `nom <- expr;`
    Affecter { name: String, value: Expr },
}

/// Programme complet : la liste de ses instructions.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}
