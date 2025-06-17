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
use std::{fs::File, io::{Read, Write}, rc::Rc};
use incremental::{parse, set_source_file, Compiler};

// All the compiler passes in order:
mod lexer;
mod parser;
mod name_resolution;
mod type_inference;

// Util modules:
mod errors;
mod incremental;

const INPUT_FILE: &str = "readme_example.ex";
const METADATA_FILE: &str = "incremental_metadata.json";

macro_rules! try_or_return {
    ($expr:expr, $err:ident -> $( $message:tt )+) => {
        match $expr {
            Ok(x) => x,
            Err($err) => {
                println!($( $message )+);
                return;
            }
        }
    };
}

// Deserialize the compiler from our metadata file.
// If we fail, just default to a fresh compiler with no cached compilations.
fn make_compiler() -> Compiler {
    let Ok(file) = File::open(METADATA_FILE) else {
        return Compiler::default();
    };

    serde_json::from_reader(&file).unwrap_or_default()
}

fn main() {
    let mut compiler = make_compiler();

    let mut source = String::new();
    let mut file = try_or_return!(File::open(INPUT_FILE), error ->
        "Failed to open `{INPUT_FILE}`:\n{error}");

    try_or_return!(file.read_to_string(&mut source), error ->
        "Failed to read from file `{INPUT_FILE}`:\n{error}");

    let file_name = Rc::new(INPUT_FILE.to_string());

    set_source_file(&file_name, source, &mut compiler);

    println!("Passes Run:");
    let (ast, errors) = parse(file_name, &mut compiler).clone();
    println!("Compiler finished.\n");

    println!("{ast}\n\nerrors:");
    for error in errors {
        println!("  {}", error.message());
    }

    if let Err(error) = write_metadata(compiler) {
        println!("\n{error}");
    }
}

/// This could be changed so that we only write if the metadata actually
/// changed but to simplify things we just always write.
fn write_metadata(compiler: Compiler) -> Result<(), String> {
    let serialized = serde_json::to_string(&compiler).map_err(|error| {
        format!("Failed to serialize database:\n{error}")
    })?;

    let serialized = serialized.into_bytes();

    let mut metadata_file = File::create(METADATA_FILE).map_err(|error| {
        format!("Failed to create file `{METADATA_FILE}`:\n{error}")
    })?;

    metadata_file.write_all(&serialized).map_err(|error| {
        format!("Failed to write to file `{METADATA_FILE}`:\n{error}")
    })
}
