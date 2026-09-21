use std::env;
use std::fs;
use std::process;

use betteralgo::lexer::tokenize;
use betteralgo::parser::parse;
use betteralgo::codegen::generate_python;

fn main() {
    let args : Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprint!("Usage : {} <fichier.algo>", args[0]);
        process::exit(1);
    }

    let path = &args[1];

    let source = fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("Erreur lors de la lecture de {}: {}", path, err);
        process::exit(1);
    });

    let tokens = tokenize(&source);
    let program = parse(&tokens);
    let python_code = generate_python(&program);

    println!("{}", python_code);
}