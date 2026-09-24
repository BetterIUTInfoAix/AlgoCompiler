use crate::parser::{Expr, Program, Statement};

/// Génère le code Python équivalent à partir de l'AST.
///
/// Les déclarations (`declarer`) ne produisent rien : Python crée la variable
/// à la première affectation. (Aucune vérification de type n'est faite pour
/// l'instant.)
pub fn generate_python(program: &Program) -> String {
    let mut output = String::new();

    for stmt in &program.statements {
        match stmt {
            Statement::Afficher(value) => {
                output.push_str(&format!("print({})\n", python_expr(value)));
            }
            Statement::Declarer { .. } => {}
            Statement::Affecter { name, value } => {
                output.push_str(&format!("{name} = {}\n", python_expr(value)));
            }
        }
    }

    output
}

/// Traduit une expression algorithmique en expression Python.
fn python_expr(expr: &Expr) -> String {
    match expr {
        Expr::Entier(n) => n.to_string(),
        Expr::Reel(f) => format_reel(*f),
        Expr::Chaine(s) => python_string(s),
        Expr::Caractere(c) => python_string(&c.to_string()),
        Expr::Booleen(true) => "True".to_string(),
        Expr::Booleen(false) => "False".to_string(),
        Expr::Ident(name) => name.clone(),
        Expr::Neg(inner) => format!("-{}", python_expr(inner)),
    }
}

/// Affiche un réel avec au moins un chiffre après la virgule (`5.0` au lieu
/// de `5`), pour préserver le type `float` en Python.
fn format_reel(value: f64) -> String {
    let text = value.to_string();
    if text.contains('.') || text.contains('e') || text.contains("inf") || text.contains("nan") {
        text
    } else {
        format!("{text}.0")
    }
}

/// Produit un littéral Python entre guillemets doubles, avec échappements.
fn python_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn python(source: &str) -> String {
        let tokens = tokenize(source).expect("lexing valide");
        let program = parse(&tokens).expect("parse valide");
        generate_python(&program)
    }

    #[test]
    fn afficher_une_chaine() {
        assert_eq!(python(r#"afficher("salut");"#), "print(\"salut\")\n");
    }

    #[test]
    fn afficher_une_variable() {
        assert_eq!(python("afficher(unEntier);"), "print(unEntier)\n");
    }

    #[test]
    fn declarer_ne_genere_rien() {
        assert_eq!(python("declarer x : entier;"), "");
    }

    #[test]
    fn affectation_entier() {
        assert_eq!(python("x <- 42;"), "x = 42\n");
    }

    #[test]
    fn affectation_negative() {
        assert_eq!(python("x <- -5;"), "x = -5\n");
    }

    #[test]
    fn affectation_booleen() {
        assert_eq!(python("x <- faux;"), "x = False\n");
        assert_eq!(python("y <- vrai;"), "y = True\n");
    }

    #[test]
    fn affectation_reel_reste_un_float() {
        assert_eq!(python("x <- 5.61;"), "x = 5.61\n");
        assert_eq!(python("x <- 5.0;"), "x = 5.0\n");
    }

    #[test]
    fn affectation_caractere_devient_une_chaine() {
        assert_eq!(python(r#"c <- 'e';"#), "c = \"e\"\n");
        assert_eq!(python(r#"c <- '\"';"#), "c = \"\\\"\"\n");
    }

    #[test]
    fn format_reel_toujours_avec_virgule() {
        assert_eq!(format_reel(5.0), "5.0");
        assert_eq!(format_reel(5.61), "5.61");
        assert_eq!(format_reel(-5.61), "-5.61");
    }

    #[test]
    fn python_string_echappe() {
        assert_eq!(python_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(python_string("a\\b"), "\"a\\\\b\"");
        assert_eq!(python_string("a\nb"), "\"a\\nb\"");
    }

    #[test]
    fn programme_complet() {
        let source = "declarer x : entier;\nx <- 42;\nafficher(\"valeur :\");\nafficher(x);\n";
        assert_eq!(python(source), "x = 42\nprint(\"valeur :\")\nprint(x)\n");
    }
}
