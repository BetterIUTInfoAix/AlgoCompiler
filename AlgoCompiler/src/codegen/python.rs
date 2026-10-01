use crate::parser::{BinOp, Expr, Program, Statement, Type, UnOp};

/// Niveau d'indentation Python (4 espaces par niveau).
const INDENT: &str = "    ";

/// Génère le code Python équivalent à partir de l'AST.
///
/// Les déclarations (`declarer`) sont traduites en annotations Python. Les
/// annotations informent les lecteurs et les vérificateurs de types, mais
/// Python ne les contrôle pas à l'exécution.
pub fn generate_python(program: &Program) -> String {
    let mut output = String::new();
    for stmt in &program.statements {
        gen_statement(stmt, 0, &mut output);
    }
    output
}

fn indent(level: usize, output: &mut String) {
    for _ in 0..level {
        output.push_str(INDENT);
    }
}

fn gen_block(statements: &[Statement], level: usize, output: &mut String) {
    if statements.is_empty() {
        indent(level, output);
        output.push_str("pass\n");
        return;
    }
    for stmt in statements {
        gen_statement(stmt, level, output);
    }
}

#[allow(clippy::too_many_lines)]
fn gen_statement(stmt: &Statement, level: usize, output: &mut String) {
    match stmt {
        Statement::Afficher(value) => {
            indent(level, output);
            output.push_str(&format!("print({})\n", python_expr(value)));
        }
        Statement::Declarer { name, ty, init } => {
            indent(level, output);
            match ty {
                Type::Tableau { size, element_type } => {
                    let default = tableau_default(element_type);
                    output.push_str(&format!(
                        "{name}: list[{}] = [{default}] * {size}\n",
                        python_type(element_type)
                    ));
                }
                Type::Constante(inner) => {
                    // Ne devrait pas arriver : le parser produit `Constante`.
                    output.push_str(&format!("{name}: {} = None\n", python_type(inner)));
                }
                _ => {
                    if let Some(value) = init {
                        output.push_str(&format!(
                            "{name}: {} = {}\n",
                            python_type(ty),
                            python_expr(value)
                        ));
                    } else {
                        output.push_str(&format!("{name}: {}\n", python_type(ty)));
                    }
                }
            }
        }
        Statement::Constante { name, ty, value } => {
            indent(level, output);
            output.push_str(&format!(
                "{name}: {} = {}\n",
                python_type(ty),
                python_expr(value)
            ));
        }
        Statement::Affecter { name, value } => {
            indent(level, output);
            output.push_str(&format!("{name} = {}\n", python_expr(value)));
        }
        Statement::AffecterIndex { base, index, value } => {
            indent(level, output);
            output.push_str(&format!(
                "{base}[{}] = {}\n",
                python_expr(index),
                python_expr(value)
            ));
        }
        Statement::Si {
            condition,
            then_branch,
            else_branch,
        } => {
            indent(level, output);
            output.push_str(&format!("if {}:\n", python_expr(condition)));
            gen_block(then_branch, level + 1, output);
            if let Some(else_branch) = else_branch {
                // `sinon_si` désucré : un seul `Si` imbriqué → `elif`.
                if let [
                    Statement::Si {
                        condition: elif_cond,
                        then_branch: elif_then,
                        else_branch: elif_else,
                    },
                ] = else_branch.as_slice()
                {
                    gen_elif(elif_cond, elif_then, elif_else, level, output);
                } else {
                    indent(level, output);
                    output.push_str("else:\n");
                    gen_block(else_branch, level + 1, output);
                }
            }
        }
        Statement::ChoixSur {
            expr,
            cases,
            default,
        } => {
            let scrutinee = python_expr(expr);
            for (i, (value, body)) in cases.iter().enumerate() {
                indent(level, output);
                if i == 0 {
                    output.push_str(&format!("if {scrutinee} == {}:\n", python_expr(value)));
                } else {
                    output.push_str(&format!("elif {scrutinee} == {}:\n", python_expr(value)));
                }
                gen_block(body, level + 1, output);
            }
            if let Some(default) = default {
                if cases.is_empty() {
                    // `choix_sur` avec seulement `autre` : exécution inconditionnelle.
                    gen_block(default, level, output);
                } else {
                    indent(level, output);
                    output.push_str("else:\n");
                    gen_block(default, level + 1, output);
                }
            } else if cases.is_empty() {
                indent(level, output);
                output.push_str("pass  # choix_sur sans cas\n");
            }
        }
        Statement::Boucle(body) => {
            indent(level, output);
            output.push_str("while True:\n");
            gen_block(body, level + 1, output);
        }
        Statement::Repeter { body, condition } => {
            // `repeter … jusqua (cond)` : exécute au moins une fois.
            indent(level, output);
            output.push_str("while True:\n");
            gen_block(body, level + 1, output);
            indent(level + 1, output);
            output.push_str(&format!("if {}:\n", python_expr(condition)));
            indent(level + 2, output);
            output.push_str("break\n");
        }
        Statement::Jusqua { condition, body } => {
            // `jusqua (cond) faire …` : boucle tant que la condition est fausse.
            indent(level, output);
            output.push_str(&format!("while not ({}):\n", python_expr(condition)));
            gen_block(body, level + 1, output);
        }
        Statement::TantQue { condition, body } => {
            indent(level, output);
            output.push_str(&format!("while {}:\n", python_expr(condition)));
            gen_block(body, level + 1, output);
        }
        Statement::Pour {
            var,
            start,
            end,
            descending,
            body,
        } => {
            indent(level, output);
            let start = python_expr(start);
            let end = python_expr(end);
            if *descending {
                // Bornes incluses : `range(debut, fin - 1, -1)`.
                output.push_str(&format!("for {var} in range({start}, ({end}) - 1, -1):\n"));
            } else {
                // Bornes incluses : `range(debut, fin + 1)`.
                output.push_str(&format!("for {var} in range({start}, ({end}) + 1):\n"));
            }
            gen_block(body, level + 1, output);
        }
        Statement::Sortie => {
            indent(level, output);
            output.push_str("break\n");
        }
        Statement::Continue => {
            indent(level, output);
            output.push_str("continue\n");
        }
    }
}

/// Émet une chaîne `elif` / `else` pour un `sinon_si` désucré.
fn gen_elif(
    condition: &Expr,
    then_branch: &[Statement],
    else_branch: &Option<Vec<Statement>>,
    level: usize,
    output: &mut String,
) {
    indent(level, output);
    output.push_str(&format!("elif {}:\n", python_expr(condition)));
    gen_block(then_branch, level + 1, output);
    if let Some(else_branch) = else_branch {
        if let [
            Statement::Si {
                condition: elif_cond,
                then_branch: elif_then,
                else_branch: elif_else,
            },
        ] = else_branch.as_slice()
        {
            gen_elif(elif_cond, elif_then, elif_else, level, output);
        } else {
            indent(level, output);
            output.push_str("else:\n");
            gen_block(else_branch, level + 1, output);
        }
    }
}

/// Traduit un type algorithmique en annotation Python.
fn python_type(ty: &Type) -> String {
    match ty {
        Type::Entier | Type::EntierNaturel => "int".to_string(),
        Type::Reel => "float".to_string(),
        Type::Booleen => "bool".to_string(),
        Type::Caractere | Type::Chaine => "str".to_string(),
        Type::Tableau { element_type, .. } => format!("list[{}]", python_type(element_type)),
        Type::Constante(inner) => python_type(inner),
    }
}

/// Valeur par défaut d'un élément de tableau selon son type.
fn tableau_default(ty: &Type) -> &'static str {
    match ty {
        Type::Reel => "0.0",
        Type::Booleen => "False",
        Type::Caractere | Type::Chaine => "\"\"",
        _ => "0",
    }
}

/// Traduit une expression algorithmique en expression Python.
fn python_expr(expr: &Expr) -> String {
    python_expr_min(expr, 0)
}

/// Traduit une expression en ne parenthésant que si sa précédence est
/// strictement inférieure à `min_prec`.
///
/// Niveaux : `ou` = 1, `et` = 2, comparaisons = 3, `+`/`-` = 4, `*`/`/` = 5.
/// L'associativité gauche autorise `a + b + c` sans parenthèses, tandis que
/// l'opérande droit impose `min_prec + 1` (`a - (b - c)` garde ses parenthèses).
fn python_expr_min(expr: &Expr, min_prec: u8) -> String {
    match expr {
        Expr::Entier(n) => n.to_string(),
        Expr::Reel(f) => format_reel(*f),
        Expr::Chaine(s) => python_string(s),
        Expr::Caractere(c) => python_string(&c.to_string()),
        Expr::Booleen(true) => "True".to_string(),
        Expr::Booleen(false) => "False".to_string(),
        Expr::Ident(name) => name.clone(),
        Expr::Index { base, index } => {
            format!("{}[{}]", python_expr(base), python_expr(index))
        }
        Expr::UnOp { op, expr } => match op {
            UnOp::Neg => format!("-{}", python_atom(expr)),
            UnOp::Not => format!("not {}", python_atom(expr)),
        },
        Expr::Appel { name, args } => {
            let args = args.iter().map(python_expr).collect::<Vec<_>>().join(", ");
            // `taille(t)` (longueur d'un tableau / string) → `len(t)`.
            if name.eq_ignore_ascii_case("taille") {
                format!("len({args})")
            } else {
                format!("{name}({args})")
            }
        }
        Expr::BinOp { op, left, right } => {
            let prec = python_prec(op);
            let text = format!(
                "{} {} {}",
                python_expr_min(left, prec),
                python_binop(op),
                python_expr_min(right, prec + 1)
            );
            if prec < min_prec {
                format!("({text})")
            } else {
                text
            }
        }
    }
}

/// Précédence d'un opérateur binaire (plus le nombre est haut, plus il lie).
fn python_prec(op: &BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => 3,
        BinOp::Add | BinOp::Sub => 4,
        BinOp::Mul | BinOp::Div => 5,
    }
}

/// Parenthèse un atome si nécessaire pour les opérateurs unaires.
fn python_atom(expr: &Expr) -> String {
    match expr {
        Expr::BinOp { .. } | Expr::UnOp { .. } => format!("({})", python_expr(expr)),
        _ => python_expr(expr),
    }
}

/// Traduit un opérateur binaire algorithmique en opérateur Python.
fn python_binop(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
        BinOp::And => "and",
        BinOp::Or => "or",
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
    fn declarer_genere_annotation() {
        assert_eq!(python("declarer x : entier;"), "x: int\n");
    }

    #[test]
    fn declarer_genere_les_annotations_python() {
        for (source_type, python_type) in [
            ("entier", "int"),
            ("entier_naturel", "int"),
            ("reel", "float"),
            ("booleen", "bool"),
            ("caractere", "str"),
            ("string", "str"),
        ] {
            assert_eq!(
                python(&format!("declarer x : {source_type};")),
                format!("x: {python_type}\n")
            );
        }
    }

    #[test]
    fn declarer_tableau_genere_liste() {
        assert_eq!(
            python("declarer t : tableau_de 3 entier;"),
            "t: list[int] = [0] * 3\n"
        );
    }

    #[test]
    fn declarer_avec_init() {
        assert_eq!(python("declarer x : entier <- 42;"), "x: int = 42\n");
    }

    #[test]
    fn constante_genere_affectation() {
        assert_eq!(
            python("declarer Kpi : constante reel <- 3.14;"),
            "Kpi: float = 3.14\n"
        );
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
    fn affectation_index() {
        assert_eq!(python("t[0] <- 6;"), "t[0] = 6\n");
    }

    #[test]
    fn arithmetique() {
        assert_eq!(python("x <- a + b * 2;"), "x = a + b * 2\n");
        assert_eq!(python("x <- (a + b) / 2;"), "x = (a + b) / 2\n");
        assert_eq!(python("x <- a - -b;"), "x = a - -b\n");
    }

    #[test]
    fn appel_taille_devient_len() {
        assert_eq!(python("x <- taille(t);"), "x = len(t)\n");
    }

    #[test]
    fn operateurs_comparaison() {
        for (algo, py) in [
            ("vaut", "=="),
            ("ne_vaut_pas", "!="),
            ("<", "<"),
            (">", ">"),
            ("<=", "<="),
            (">=", ">="),
        ] {
            assert_eq!(
                python(&format!("si (a {algo} b) afficher(a); fsi")),
                format!("if a {py} b:\n    print(a)\n"),
                "{algo}"
            );
        }
    }

    #[test]
    fn operateurs_logiques() {
        assert_eq!(
            python("si (a et b) afficher(a); fsi"),
            "if a and b:\n    print(a)\n"
        );
        assert_eq!(
            python("si (a et_alors b) afficher(a); fsi"),
            "if a and b:\n    print(a)\n"
        );
        assert_eq!(
            python("si (a ou b) afficher(a); fsi"),
            "if a or b:\n    print(a)\n"
        );
        assert_eq!(
            python("si (a ou_sinon b) afficher(a); fsi"),
            "if a or b:\n    print(a)\n"
        );
        assert_eq!(
            python("si (non a) afficher(a); fsi"),
            "if not a:\n    print(a)\n"
        );
    }

    #[test]
    fn si_sinon() {
        assert_eq!(
            python("si (a vaut b) afficher(a); sinon afficher(b); fsi"),
            "if a == b:\n    print(a)\nelse:\n    print(b)\n"
        );
    }

    #[test]
    fn si_sinon_si() {
        assert_eq!(
            python("si (a vaut 1) afficher(a); sinon_si (a vaut 2) afficher(b); fsi"),
            "if a == 1:\n    print(a)\nelif a == 2:\n    print(b)\n"
        );
    }

    #[test]
    fn choix_sur() {
        assert_eq!(
            python("choix_sur x entre cas 1 : afficher(1); autre : afficher(0); fchoix"),
            "if x == 1:\n    print(1)\nelse:\n    print(0)\n"
        );
    }

    #[test]
    fn boucle_infinie() {
        assert_eq!(
            python("boucle afficher(1); fboucle"),
            "while True:\n    print(1)\n"
        );
    }

    #[test]
    fn boucle_sortie() {
        assert_eq!(
            python("boucle si (x vaut 1) sortie; fsi fboucle"),
            "while True:\n    if x == 1:\n        break\n"
        );
    }

    #[test]
    fn repeter_jusqua() {
        assert_eq!(
            python("repeter afficher(1); jusqua (x vaut 5);"),
            "while True:\n    print(1)\n    if x == 5:\n        break\n"
        );
    }

    #[test]
    fn jusqua_faire() {
        assert_eq!(
            python("jusqua (x vaut 5) faire afficher(1); ffaire"),
            "while not (x == 5):\n    print(1)\n"
        );
    }

    #[test]
    fn tant_que() {
        assert_eq!(
            python("tant_que (x vaut 5) faire afficher(1); ffaire"),
            "while x == 5:\n    print(1)\n"
        );
    }

    #[test]
    fn pour_croissant() {
        assert_eq!(
            python("pour (i variant_de 1 a 3) faire afficher(i); ffaire"),
            "for i in range(1, (3) + 1):\n    print(i)\n"
        );
    }

    #[test]
    fn pour_descendant() {
        assert_eq!(
            python("pour (i variant_de 3 a 1 descendant) faire afficher(i); ffaire"),
            "for i in range(3, (1) - 1, -1):\n    print(i)\n"
        );
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
        assert_eq!(
            python(source),
            "x: int\nx = 42\nprint(\"valeur :\")\nprint(x)\n"
        );
    }

    #[test]
    fn programme_boucle_complete() {
        let source = "declarer i : entier;\npour (i variant_de 1 a 3) faire afficher(i); ffaire\n";
        assert_eq!(
            python(source),
            "i: int\nfor i in range(1, (3) + 1):\n    print(i)\n"
        );
    }
}
