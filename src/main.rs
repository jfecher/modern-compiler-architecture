//! Welcome to this repository! You're in the entry point to the program where we handle
//! command-line arguments and invoke the rest of the compiler.
//!
//! Compared to a traditional pipeline-style compiler, the main difference in architecture
//! of this compiler comes from it being pull-based rather than push-based. So instead of
//! starting by lexing everything, then parsing, name resolution, type inference, etc.,
//! we start by saying "I want a compiled program!" Then the function to get us a compiled
//! program says "well, I need a type-checked Ast for that." Then our type inference pass
//! says "I need a name-resolved ast," and so on. So this compiler still has the same
//! passes you know and love (and listed further down), they're just composed together a
//! bit differently.
//!
//! List of compiler passes and the source file to find more about them in:
//! - Lexing `src/lexer/mod.rs`:
//! - Parsing `src/parser/mod.rs`:
//! - Name Resolution `src/name_resolution/mod.rs`:
//! - Type Inference `src/type_inference/mod.rs`:
//!
//! Non-passes:
//! - `src/errors.rs`: Defines each error used in the program as well as the `Location` struct
//! - `src/incremental.rs`: Some plumbing for the inc-complete library which also defines
//!   which functions we're caching the result of.
use std::{fs::File, io::Read, rc::Rc};

use incremental::{parse_db, Compiler, SourceFile};

// All the compiler passes in order:
mod lexer;
mod parser;
mod name_resolution;
mod type_inference;

// Util modules:
mod errors;
mod incremental;

fn main() {
    let mut source = String::new();
    let mut file = File::open("readme_example.ex").unwrap();
    file.read_to_string(&mut source).unwrap();
    let file_name = Rc::new("readme_example.ex".to_string());

    let mut db = Compiler::new();
    db.update_input(SourceFile(file_name.clone()), source);

    let (ast, errors) = parse_db(file_name, &mut db);

    println!("{ast}\n\nerrors:");
    for error in errors {
        println!("  {}", error.message());
    }

    let serialized = serde_json::to_string_pretty(&db);
}
