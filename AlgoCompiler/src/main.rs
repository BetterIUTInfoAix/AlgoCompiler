use std::env;
use std::fs;
use std::process;

use betteralgo::codegen::generate_python;
use betteralgo::errors::CompileError;
use betteralgo::lexer::tokenize;
use betteralgo::parser::parse;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage : {} <fichier.algo>", args[0]);
        process::exit(2);
    }

    let path = &args[1];

    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            let error = CompileError::io(format!("impossible de lire {path:?} : {err}"));
            // Pas de contexte source disponible : on affiche la version courte.
            eprintln!("Erreur {}", error.short(Some(path)));
            process::exit(1);
        }
    };

    let tokens = match tokenize(&source) {
        Ok(tokens) => tokens,
        Err(err) => {
            err.report(&source, Some(path));
            process::exit(1);
        }
    };

    let program = match parse(&tokens) {
        Ok(program) => program,
        Err(err) => {
            err.report(&source, Some(path));
            process::exit(1);
        }
    };

    let python_code = generate_python(&program);
    println!("{python_code}");
}
