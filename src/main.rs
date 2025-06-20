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
use incremental::{Compiler, set_source_file};
use std::{
    collections::{HashSet, VecDeque},
    fs::File,
    io::{Read, Write},
    rc::Rc,
};

use crate::{
    errors::{Error, Errors},
    incremental::{Parse, TypeCheck, VisibleDefinitions},
};

// All the compiler passes:
mod definition_collection;
mod lexer;
mod name_resolution;
mod parser;
mod type_inference;

// Util modules:
mod errors;
mod incremental;

const INPUT_FILE: &str = "readme_example.ex";
const METADATA_FILE: &str = "incremental_metadata.ron";

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

    let file_name = Rc::new(INPUT_FILE.to_string());
    set_source_file(&file_name, source, &mut compiler);

    println!("Passes Run:");

    // First, run through our input file and any imports recursively to find any
    // files which have changed. These are the imports to our incremental compilation
    // so we can't dynamically update our inputs within another query. Instead, we
    // can query to collect them all and update them here at top-level.
    let (files, mut errors) = collect_all_changed_files(file_name, &mut compiler);
    errors.extend(compile(files, &mut compiler));

    println!("Compiler finished.\n");

    for error in errors {
        println!("  {}", error.message());
    }

    if let Err(error) = write_metadata(compiler) {
        println!("\n{error}");
    }
}

fn collect_all_changed_files(start_file: Rc<String>, compiler: &mut Compiler) -> (HashSet<Rc<String>>, Errors) {
    // We expect `compiler.update_input` to already be called for start_file.
    // Reason being is that we can't start with `start_file` in our queue because
    // it is the only file without a source location for the import, because there was no import.
    let imports = incremental::get_imports(start_file.clone(), compiler);

    let mut queue = imports.iter().cloned().collect::<VecDeque<_>>();

    let mut finished = HashSet::new();
    finished.insert(start_file);
    let mut errors = Vec::new();

    while let Some((file, location)) = queue.pop_front() {
        if finished.contains(&file) {
            continue;
        }
        finished.insert(file.clone());

        let text = read_file(&file).unwrap_or_else(|_| {
            errors.push(Error::UnknownImportFile { file_name: file.clone(), location });

            // Treat file as an empty string. This will probably just lead to more errors but does let us continue
            // to collect name/type errors for other files
            String::new()
        });
        set_source_file(&file, text, compiler);

        let imports = incremental::get_imports(file, compiler);
        queue.extend(imports.iter().cloned());
    }

    (finished, errors)
}

fn compile(files: HashSet<Rc<String>>, compiler: &mut Compiler) -> Errors {
    let mut errors = Vec::new();
    for file in files {
        let ast = compiler.get(Parse { file_name: file.clone() }).ast.clone();
        // The errors from def collection aren't included in the resolution errors
        errors.extend(compiler.get(VisibleDefinitions { file_name: file }).1.iter().cloned());

        for item in ast.statements.iter() {
            let result = compiler.get(TypeCheck(item.id().clone())).clone();
            errors.extend(result.errors);
            println!("  - {item}  : {}", result.typ);
        }
    }
    errors
}

/// This could be changed so that we only write if the metadata actually
/// changed but to simplify things we just always write.
fn write_metadata(compiler: Compiler) -> Result<(), String> {
    let serialized = ron::to_string(&compiler).map_err(|error| format!("Failed to serialize database:\n{error}"))?;

    let serialized = serialized.into_bytes();

    let mut metadata_file =
        File::create(METADATA_FILE).map_err(|error| format!("Failed to create file `{METADATA_FILE}`:\n{error}"))?;

    metadata_file.write_all(&serialized).map_err(|error| format!("Failed to write to file `{METADATA_FILE}`:\n{error}"))
}

fn read_file(file_name: &str) -> Result<String, String> {
    let mut file = File::open(file_name).map_err(|error| format!("Failed to open `{INPUT_FILE}`:\n{error}"))?;

    let mut text = String::new();
    file.read_to_string(&mut text).map_err(|error| format!("Failed to read from file `{INPUT_FILE}`:\n{error}"))?;

    Ok(text)
}
