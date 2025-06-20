use std::rc::Rc;

use crate::{
    errors::{Error, Errors, Location},
    incremental::{
        self, get_exported_definitions, parse, parse_cloned, CompilerHandle, Definitions, ExportedDefinitions, GetImports, VisibleDefinitions
    },
    parser::ast::TopLevelStatement,
};

/// Collect all definitions which should be visible to expressions within this file.
/// This includes all top-level definitions within this file, as well as any imported ones.
pub fn visible_definitions_impl(context: &VisibleDefinitions, db: &mut CompilerHandle) -> (Definitions, Errors) {
    incremental::enter_query();
    incremental::println(format!("Collecting visible definitions in {}", context.file_name));

    let (mut definitions, mut errors) = get_exported_definitions(context.file_name.clone(), db).clone();

    // This should always be cached. Ignoring errors here since they should already be
    // included in get_exported_definitions' errors
    let ast = parse(context.file_name.clone(), db).0.clone();

    for item in ast.statements.iter() {
        if let TopLevelStatement::Import { file_name, id: import_id } = item {
            let (exports, more_errors) = get_exported_definitions(file_name.name.clone(), db).clone();
            errors.extend(more_errors);

            for (exported_name, exported_id) in exports {
                if let Some(existing) = definitions.get(&exported_name) {
                    // This reports the location the item was defined in, not the location it was imported at.
                    // I could improve this but instead I'll leave it as an exercise for the reader!
                    let first_location = existing.location(db);
                    let second_location = import_id.location(db);
                    let name = exported_name;
                    errors.push(Error::ImportedNameAlreadyInScope { name, first_location, second_location });
                } else {
                    definitions.insert(exported_name, exported_id);
                }
            }
        }
    }

    incremental::exit_query();
    (definitions, errors)
}

/// Collect only the exported definitions within a file.
/// For this small example language, this is all top-level definitions in a file, except for imported ones.
pub fn exported_definitions_impl(context: &ExportedDefinitions, db: &mut CompilerHandle) -> (Definitions, Errors) {
    incremental::enter_query();
    incremental::println(format!("Collecting exported definitions in {}", context.file_name));

    let (ast, mut errors) = parse_cloned(context.file_name.clone(), db);
    let mut definitions = Definitions::default();

    // Collect each definition, issuing an error if there is a duplicate name (imports are not counted)
    for item in ast.statements.iter() {
        if let TopLevelStatement::Definition(definition) = item {
            if let Some(existing) = definitions.get(&definition.name.name) {
                let first_location = existing.location(db);
                let second_location = definition.name.id.location(&definition.id, db);
                let name = definition.name.name.clone();
                errors.push(Error::NameAlreadyInScope { name, first_location, second_location });
            } else {
                definitions.insert(definition.name.name.clone(), definition.id.clone());
            }
        }
    }

    incremental::exit_query();
    (definitions, errors)
}

/// Collects the file names of all imports within this file.
pub fn get_imports_impl(context: &GetImports, db: &mut CompilerHandle) -> Vec<(Rc<String>, Location)> {
    incremental::enter_query();
    incremental::println(format!("Collecting imports of {}", context.file_name));

    // Ignore parse errors for now, we can report them later
    let ast = parse(context.file_name.clone(), db).0.clone();
    let mut imports = Vec::new();

    // Collect each definition, issuing an error if there is a duplicate name (imports are not counted)
    for item in ast.statements.iter() {
        if let TopLevelStatement::Import { file_name, id } = item {
            // We don't care about duplicate imports.
            // This method is only used for finding input files and the top-level
            // will filter out any repeats.
            let location = id.location(db);
            imports.push((file_name.name.clone(), location));
        }
    }

    incremental::exit_query();
    imports
}
