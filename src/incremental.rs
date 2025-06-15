use std::rc::Rc;

use inc_complete::{define_input, define_intermediate};

use crate::{errors::Error, parser::{ast::Ast, parse_impl}};

define_input!(SourceFile { file_name: Rc<String> }, get_source_file, String);

define_intermediate!(Parse { &file_name: Rc<String> }, parse, (Ast, Vec<Error>), parse_impl);

pub type Compiler = inc_complete::Db<(SourceFile, Parse)>;
