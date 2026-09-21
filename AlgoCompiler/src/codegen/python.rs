use crate::parser::{Program, Statement};

pub fn generate_python(program: &Program) -> String {
    let mut output = String::new();

    for stmt in &program.statements {
        match stmt {
            Statement::Afficher(text) => {
                output.push_str(&format!("print(\"{}\")\n", text));
            }
        }
    }

    output
}
