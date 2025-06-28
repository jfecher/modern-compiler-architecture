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
use incremental::{set_source_file, CompileFile, Compiler};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Write},
    sync::Arc,
};

use crate::errors::Errors;

// All the compiler passes:
// (listed out of order because `cargo fmt` alphabetizes them)
mod find_changed_files;
mod definition_collection;
mod lexer;
mod name_resolution;
mod parser;
mod type_inference;
mod backend;

// Util modules:
mod errors;
mod incremental;

const INPUT_FILE: &str = "input.ex";
const METADATA_FILE: &str = "incremental_metadata.json";

// Deserialize the compiler from our metadata file.
// If we fail, just default to a fresh compiler with no cached compilations.
fn make_compiler() -> Compiler {
    let Ok(text) = read_file(METADATA_FILE) else {
        return Compiler::default();
    };

    ron::from_str(&text).unwrap_or_default()
}

fn main() {
    let mut compiler = make_compiler();

    let Ok(source) = read_file(INPUT_FILE) else { return };

    let file_name = Arc::new(INPUT_FILE.to_string());
    set_source_file(file_name.clone(), source, &mut compiler);

    println!("Passes Run:");

    // First, run through our input file and any imports recursively to find any
    // files which have changed. These are the imports to our incremental compilation
    // so we can't dynamically update our inputs within another query. Instead, we
    // can query to collect them all and update them here at top-level.
    let (files, mut errors) = find_changed_files::collect_all_changed_files(file_name, &mut compiler);
    errors.extend(compile_all(files, &mut compiler));

    println!("Compiler finished.\n");

    for error in errors {
        println!("  {}", error.message());
    }

    if let Err(error) = write_metadata(compiler) {
        println!("\n{error}");
    }
}

/// Compile all the files in the set to python files
fn compile_all(files: BTreeSet<Arc<String>>, compiler: &mut Compiler) -> Errors {
    for file in files {
        let output_file = file.replace(".ex", ".py");
        let text = CompileFile { file_name: file }.get(compiler);
        if let Err(msg) = write_file(&output_file, &text) {
            eprintln!("error: {msg}");
        }
    }
    Vec::new()
}

fn write_file(file_name: &str, text: &str) -> Result<(), String> {
    let mut metadata_file =
        File::create(file_name).map_err(|error| format!("Failed to create file `{file_name}`:\n{error}"))?;

    let text = text.as_bytes();
    metadata_file.write_all(text).map_err(|error| format!("Failed to write to file `{file_name}`:\n{error}"))
}

/// This could be changed so that we only write if the metadata actually
/// changed but to simplify things we just always write.
fn write_metadata(compiler: Compiler) -> Result<(), String> {
    // Using `to_writer` here would avoid the intermediate step of creating the string
    let serialized = ron::to_string(&compiler).map_err(|error| format!("Failed to serialize database:\n{error}"))?;
    write_file(METADATA_FILE, &serialized)
}

fn read_file(file_name: &str) -> Result<String, String> {
    let mut file = File::open(file_name).map_err(|error| format!("Failed to open `{INPUT_FILE}`:\n{error}"))?;

    let mut text = String::new();
    file.read_to_string(&mut text).map_err(|error| format!("Failed to read from file `{INPUT_FILE}`:\n{error}"))?;

    Ok(text)
}
