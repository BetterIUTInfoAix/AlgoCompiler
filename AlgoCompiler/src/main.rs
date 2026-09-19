use betteralgo::lexer::tokenize;
use betteralgo::parser::parse;
use betteralgo::codegen::generate_python;

fn main() {
    let source = r#"afficher ("Casali le goat !");"#;

    let tokens = tokenize(source);
    let program = parse(&tokens);
    let python_code = generate_python(&program);

    println!("--- Code Algo ---\n{}", source);
    println!("--- Python généré ---\n{}", python_code);
}