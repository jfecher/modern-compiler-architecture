use lexer::lex_file;


mod lexer;
mod parser;
mod name_resolution;
mod type_inference;

fn main() {
    let tokens = lex_file("def add2: Int -> Int -> Int = fn x y -> x + y");
    println!("{:?}", tokens);
}
