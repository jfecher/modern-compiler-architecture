use std::{collections::BTreeMap, rc::Rc};

use inc_complete::{OutputType, define_input, define_intermediate, impl_storage, storage::HashMapStorage};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{Errors, Location},
    name_resolution,
    parser::{self, ParserResult, ast::Ast, ids::TopLevelId},
};

pub type Compiler = inc_complete::Db<Storage>;
pub type CompilerHandle<'db> = inc_complete::DbHandle<'db, Storage>;

#[derive(Default, Serialize, Deserialize)]
pub struct Storage {
    files: HashMapStorage<SourceFile>,
    parse_results: HashMapStorage<Parse>,
    visible_definitions: HashMapStorage<VisibleDefinitions>,
    exported_definitions: HashMapStorage<ExportedDefinitions>,
    get_imports: HashMapStorage<GetImports>,
}

impl_storage!(Storage,
    files: SourceFile,
    parse_results: Parse,
    visible_definitions: VisibleDefinitions,
    exported_definitions: ExportedDefinitions,
    get_imports: GetImports,
);

///////////////////////////////////////////////////////////
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceFile {
    file_name: Rc<String>,
}
define_input!(0, SourceFile -> String, Storage);

pub fn set_source_file<'c>(file_name: &Rc<String>, text: String, db: &'c mut Compiler) {
    db.update_input(SourceFile { file_name: file_name.clone() }, text);
}

pub fn get_source_file<'c>(file_name: Rc<String>, db: &'c mut CompilerHandle) -> &'c String {
    db.get(SourceFile { file_name })
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parse {
    pub file_name: Rc<String>,
}

define_intermediate!(1, Parse -> ParserResult, Storage, parser::parse_impl);

pub fn parse_result<'c>(file_name: Rc<String>, db: &'c mut CompilerHandle) -> &'c ParserResult {
    db.get(Parse { file_name })
}

pub fn parse<'c>(file_name: Rc<String>, db: &'c mut CompilerHandle) -> (&'c Ast, &'c Errors) {
    let result = parse_result(file_name, db);
    (&result.ast, &result.errors)
}

pub fn parse_cloned<'c>(file_name: Rc<String>, db: &'c mut CompilerHandle) -> (Ast, Errors) {
    let (ast, errors) = parse(file_name, db);
    (ast.clone(), errors.clone())
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleDefinitions {
    pub file_name: Rc<String>,
}

/// We iterate over collected definitions within `visible_definitions_impl`. Since
/// collecting these can error, we need a stable iteration order, otherwise the order
/// we issue errors would be nondeterministic. This is why we use a BTreeMap over a
/// HashMap, since hashmap iteration in rust has a nondeterministic ordering.
pub type Definitions = BTreeMap<Rc<String>, TopLevelId>;

define_intermediate!(2, VisibleDefinitions -> (Definitions, Errors), Storage, name_resolution::visible_definitions_impl);

pub fn get_globally_visible_definitions<'c>(
    file_name: Rc<String>, db: &'c mut Compiler,
) -> &'c <VisibleDefinitions as OutputType>::Output {
    db.get(VisibleDefinitions { file_name })
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedDefinitions {
    pub file_name: Rc<String>,
}

define_intermediate!(3, ExportedDefinitions -> (Definitions, Errors), Storage, name_resolution::exported_definitions_impl);

pub fn get_exported_definitions<'c>(
    file_name: Rc<String>, db: &'c mut CompilerHandle,
) -> &'c <ExportedDefinitions as OutputType>::Output {
    db.get(ExportedDefinitions { file_name })
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetImports {
    pub file_name: Rc<String>,
}

define_intermediate!(4, GetImports -> Vec<(Rc<String>, Location)>, Storage, name_resolution::get_imports_impl);

pub fn get_imports<'c>(file_name: Rc<String>, db: &'c mut Compiler) -> &'c <GetImports as OutputType>::Output {
    db.get(GetImports { file_name })
}
