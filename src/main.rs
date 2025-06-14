use std::{fs::File, io::Read, rc::Rc};

use lexer::lex_file;
use parser::parse;

// All the compiler passes in order:
mod lexer;
mod parser;
mod name_resolution;
mod type_inference;

// Util modules:
mod errors;

fn main() {
    let mut source = String::new();
    let mut file = File::open("readme_example.ex").unwrap();
    file.read_to_string(&mut source).unwrap();
    let file_name = Rc::new("readme_example.ex".to_string());

    let tokens = lex_file(file_name.clone(), &source);
    let (ast, errors) = parse(file_name, tokens);

    println!("ast: {ast:?}\n\nerrors:");
    for error in errors {
        println!("  {}", error.message());
    }
}
