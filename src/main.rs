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
use incremental::{set_source_file, CompileFile, Compiler, GetImports};
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Write},
    sync::Arc,
};

use crate::errors::{Error, Errors};

// All the compiler passes:
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

    let file_name = Arc::new(INPUT_FILE.to_string());
    set_source_file(file_name.clone(), source, &mut compiler);

    println!("Passes Run:");

    // First, run through our input file and any imports recursively to find any
    // files which have changed. These are the imports to our incremental compilation
    // so we can't dynamically update our inputs within another query. Instead, we
    // can query to collect them all and update them here at top-level.
    let (files, mut errors) = collect_all_changed_files(file_name, &mut compiler);
    errors.extend(compile_all(files, &mut compiler));

    println!("Compiler finished.\n");

    for error in errors {
        println!("  {}", error.message());
    }

    if let Err(error) = write_metadata(compiler) {
        println!("\n{error}");
    }
}

/// One limitation of query systems is that you cannot change inputs during an incremental
/// computation. For a compiler, this means dynamically discovering new files (inputs) to parse is
/// a bit more difficult. Instead of doing it within an incremental computation like the parser or
/// during name resolution, we have a separate step here to collect all the files that are used and
/// check if any have changed. Some frameworks may let you iterate through known inputs where you
/// may be able to test if a file has changed there, but inc-complete doesn't support this (yet) as
/// of version 0.5.0.
///
/// In `collect_all_changed_files`, we start by parsing the first INPUT_FILE which imports all
/// other files. We collect each import and for each import we read the new file, set the source
/// file input (which requires an exclusive &mut reference), and spawn a thread to parse that file
/// and collect the imports. Spawning multiple threads here is advantageous when a file imports
/// many source files - we can distribute work to parse many of them at once. The implementation
/// for this could be more efficient though. For example, the parser could accept the shared `queue`
/// of files to parse as an argument, and push to this queue immediately when it finds an import.
fn collect_all_changed_files(start_file: Arc<String>, compiler: &mut Compiler) -> (HashSet<Arc<String>>, Errors) {
    // We expect `compiler.update_input` to already be called for start_file.
    // Reason being is that we can't start with `start_file` in our queue because
    // it is the only file without a source location for the import, because there was no import.
    let imports = compiler.get(GetImports { file_name: start_file.clone() });
    let queue = imports.iter().cloned().collect::<scc::Queue<_>>();

    // let thread_pool = rayon::ThreadPoolBuilder::new().build().unwrap();
        let mut finished = HashSet::new();
        finished.insert(start_file);
        let mut errors = Vec::new();

        while let Some(file_and_location) = queue.pop() {
            let file = file_and_location.0.clone();
            let location = file_and_location.1.clone();

            if finished.contains(&file) {
                continue;
            }
            finished.insert(file.clone());

            let text = read_file(&file).unwrap_or_else(|_| {
                errors.push(Error::UnknownImportFile { file_name: file.clone(), location });

                // Treat file as an empty string. This will probably just lead to more errors but does
                // let us continue to collect name/type errors for other files
                String::new()
            });
            set_source_file(file.clone(), text, compiler);

            // Parse and collect imports of the file in a separate thread. This can be helpful
            // when files contain many imports, so we can parse many of them simultaneously.
            // scope.spawn(|_| {
                for import in compiler.get(GetImports { file_name: file }) {
                    queue.push(import);
                }
            // });
        }
        (finished, errors)
}

fn compile_all(files: HashSet<Arc<String>>, compiler: &mut Compiler) -> Errors {
    for file in files {
        let output_file = file.replace(".ex", ".py");
        let text = CompileFile { file_name: file }.get(compiler);
        if let Err(msg) = write_file(&output_file, &text) {
            println!("! {msg}");
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
    let serialized = ron::to_string(&compiler).map_err(|error| format!("Failed to serialize database:\n{error}"))?;
    write_file(METADATA_FILE, &serialized)
}

fn read_file(file_name: &str) -> Result<String, String> {
    let mut file = File::open(file_name).map_err(|error| format!("Failed to open `{INPUT_FILE}`:\n{error}"))?;

    let mut text = String::new();
    file.read_to_string(&mut text).map_err(|error| format!("Failed to read from file `{INPUT_FILE}`:\n{error}"))?;

    Ok(text)
}
