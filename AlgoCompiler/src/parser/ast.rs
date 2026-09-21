#[derive(Debug, PartialEq)]
pub enum Statement {
    Afficher(String),
}

#[derive(Debug, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}
