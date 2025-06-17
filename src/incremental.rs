use std::rc::Rc;

use inc_complete::{define_input, define_intermediate, impl_storage, storage::HashMapStorage, OutputType};
use serde::{Deserialize, Serialize};

use crate::{errors::Error, parser::{ast::Ast, parse_impl}};

pub type Compiler = inc_complete::Db<Storage>;
pub type CompilerHandle<'db> = inc_complete::DbHandle<'db, Storage>;

#[derive(Default, Serialize, Deserialize)]
pub struct Storage {
    files: HashMapStorage<SourceFile>,
    parse_results: HashMapStorage<Parse>,
}

impl_storage!(Storage,
    files: SourceFile,
    parse_results: Parse,
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
define_intermediate!(1, Parse -> (Ast, Vec<Error>), Storage, parse_impl);

pub fn parse<'c>(file_name: Rc<String>, db: &'c mut Compiler) -> &'c <Parse as OutputType>::Output {
    db.get(Parse { file_name })
}
