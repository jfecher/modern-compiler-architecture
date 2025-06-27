use std::{cell::Cell, collections::BTreeMap, sync::Arc};

use inc_complete::{OutputType, define_input, define_intermediate, impl_storage, storage::HashMapStorage};
use serde::{Deserialize, Serialize};

use crate::{
    backend, definition_collection, errors::{Errors, Location}, name_resolution::{self, ResolutionResult}, parser::{
        self, ast::{Ast, TopLevelStatement}, ids::TopLevelId, ParserResult
    }, type_inference::{self, types::TopLevelDefinitionType, TypeCheckResult}
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
    resolves: HashMapStorage<Resolve>,
    top_level_statement: HashMapStorage<GetTopLevelStatement>,
    get_types: HashMapStorage<GetType>,
    type_checks: HashMapStorage<TypeCheck>,
    compiled_files: HashMapStorage<CompileFile>,
}

impl_storage!(Storage,
    files: SourceFile,
    parse_results: Parse,
    visible_definitions: VisibleDefinitions,
    exported_definitions: ExportedDefinitions,
    get_imports: GetImports,
    resolves: Resolve,
    top_level_statement: GetTopLevelStatement,
    get_types: GetType,
    type_checks: TypeCheck,
    compiled_files: CompileFile,
);

std::thread_local! {
    // This is a helper to show us how many queries deep we are for our print outs
    static QUERY_NESTING: Cell<usize> = Cell::new(0);
}

pub fn enter_query() {
    QUERY_NESTING.with(|cell| {
        cell.set(cell.get() + 1);
    });
}

pub fn exit_query() {
    QUERY_NESTING.with(|cell| {
        cell.set(cell.get() - 1);
    });
}

pub fn println(msg: String) {
    let level = QUERY_NESTING.with(|cell| cell.get());
    let spaces = "  ".repeat(level);

    // Thread ids are usually in the form `ThreadId(X)` or `ThreadId(XX)`.
    // Add some padding to keep output aligned for both cases.
    println!("{:02?}: {spaces}- {msg}", std::thread::current().id());
}

///////////////////////////////////////////////////////////
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceFile {
    file_name: Arc<String>,
}
define_input!(0, SourceFile -> String, Storage);

pub fn set_source_file<'c>(file_name: &Arc<String>, text: String, db: &mut 'c Compiler) {
    db.update_input(SourceFile { file_name: file_name.clone() }, text);
}

pub fn get_source_file<'c>(file_name: Arc<String>, db: &'c CompilerHandle) -> String {
    db.get(SourceFile { file_name })
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parse {
    pub file_name: Arc<String>,
}

define_intermediate!(1, Parse -> ParserResult, Storage, parser::parse_impl);

pub fn parse_result<'c>(file_name: Arc<String>, db: &'c CompilerHandle) -> ParserResult {
    db.get(Parse { file_name })
}

pub fn parse<'c>(file_name: Arc<String>, db: &'c CompilerHandle) -> (Ast, Errors) {
    let result = parse_result(file_name, db);
    (result.ast, result.errors)
}

pub fn parse_cloned<'c>(file_name: Arc<String>, db: &'c CompilerHandle) -> (Ast, Errors) {
    let (ast, errors) = parse(file_name, db);
    (ast.clone(), errors.clone())
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleDefinitions {
    pub file_name: Arc<String>,
}

/// We iterate over collected definitions within `visible_definitions_impl`. Since
/// collecting these can error, we need a stable iteration order, otherwise the order
/// we issue errors would be nondeterministic. This is why we use a BTreeMap over a
/// HashMap, since hashmap iteration in rust has a nondeterministic ordering.
pub type Definitions = BTreeMap<Arc<String>, TopLevelId>;

define_intermediate!(2, VisibleDefinitions -> (Definitions, Errors), Storage, definition_collection::visible_definitions_impl);

pub fn get_globally_visible_definitions<'c>(
    file_name: Arc<String>, db: &'c CompilerHandle,
) -> <VisibleDefinitions as OutputType>::Output {
    db.get(VisibleDefinitions { file_name })
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedDefinitions {
    pub file_name: Arc<String>,
}

define_intermediate!(3, ExportedDefinitions -> (Definitions, Errors), Storage, definition_collection::exported_definitions_impl);

pub fn get_exported_definitions<'c>(
    file_name: Arc<String>, db: &'c CompilerHandle,
) -> <ExportedDefinitions as OutputType>::Output {
    db.get(ExportedDefinitions { file_name })
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetImports {
    pub file_name: Arc<String>,
}

define_intermediate!(4, GetImports -> Vec<(Arc<String>, Location)>, Storage, definition_collection::get_imports_impl);

pub fn get_imports<'c>(file_name: Arc<String>, db: &'c Compiler) -> <GetImports as OutputType>::Output {
    db.get(GetImports { file_name })
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolve(pub TopLevelId);

define_intermediate!(5, Resolve -> ResolutionResult, Storage, name_resolution::resolve_impl);

pub fn resolve<'c>(item: TopLevelId, db: &'c CompilerHandle) -> <Resolve as OutputType>::Output {
    db.get(Resolve(item))
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetTopLevelStatement(pub TopLevelId);

// This one is quick and simple, let's just define it here
define_intermediate!(6, GetTopLevelStatement -> TopLevelStatement, Storage, |context, compiler| {
    let target_id = &context.0;
    let ast = parse(target_id.file_path.clone(), compiler).0;

    for statement in ast.statements.iter() {
        if statement.id() == target_id {
            return statement.clone();
        }
    }
    panic!("No TopLevelStatement for id {target_id}")
});

pub fn get_statement<'c>(item: TopLevelId, db: &'c CompilerHandle) -> TopLevelStatement {
    db.get(GetTopLevelStatement(item))
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetType(pub TopLevelId);

define_intermediate!(7, GetType -> TopLevelDefinitionType, Storage, type_inference::get_type_impl);

pub fn get_type<'c>(item: TopLevelId, db: &'c CompilerHandle) -> <GetType as OutputType>::Output {
    db.get(GetType(item))
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeCheck(pub TopLevelId);

define_intermediate!(8, TypeCheck -> TypeCheckResult, Storage, type_inference::type_check_impl);

pub fn type_check<'c>(item: TopLevelId, db: &'c CompilerHandle) -> <TypeCheck as OutputType>::Output {
    db.get(TypeCheck(item))
}

///////////////////////////////////////////////////////////
#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileFile { pub file_name: Arc<String> }

define_intermediate!(9, CompileFile -> String, Storage, backend::compile_file_impl);

pub fn compile_file<'c>(file_name: Arc<String>, db: &'c mut Compiler) -> String {
    db.get(CompileFile { file_name })
}
