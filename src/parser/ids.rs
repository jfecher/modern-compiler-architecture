use std::{hash::Hasher, rc::Rc};

use serde::{Deserialize, Serialize};

use crate::{
    errors::Location,
    incremental::{self, CompilerHandle},
    parser::ast::Expression,
};

/// A `TopLevelId` is a 64-bit hash uniquely identifying a particular
/// `TopLevelStatement` node. Since these are attached to each node, and we cache
/// nodes by value, any time an Id changes, the compiler will see the
/// associated node as having changed. For this reason, we want to try
/// to make these Ids as stable as possible when the source program changes.
/// Since Ids must be globally unique (ie. across all files), we usually hash the file path containing
/// the Ast node, in addition to the node itself. This means if a file is renamed
/// every Ast node will be marked as changed but this should be rare enough to be okay.
/// Beyond that, how we hash nodes differs depending on the type of node. See
/// the associated `new` functions for explanations on how each is handled.
///
/// Also note that these Ids are only meant to identify an Ast node - they should
/// not be used to answer the question "has this Ast node changed?" since they
/// do not hash all fields of a node.
///
/// Since the Ast is immutable, this id is also used to associate additional
/// data with an Ast including its Location, and later on its Type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TopLevelId {
    file_path: Rc<String>,
    content_hash: u64,
}

impl TopLevelId {
    /// Imports hash the file path containing the import node as well
    /// as the string argument on what to import. It is possible (although an error)
    /// to have multiple `import` statements in the same file importing the same file,
    /// to handle this there is also a `collision` counter such that each name collision
    /// within a file increments this and is given a different Id as a result.
    pub fn new_import(file_path: Rc<String>, import_name: &str, collision: u32) -> TopLevelId {
        hash(file_path, (import_name, collision))
    }

    /// Similar to imports, definitions are hashed from their source file, definition
    /// name, and a collision variable to disambiguate multiple definitions of the same name.
    ///
    /// Unfortunately, this means any time a definition is renamed it will have to be recompiled.
    pub fn new_definition(file_path: Rc<String>, definition_name: &str, collision: u32) -> TopLevelId {
        hash(file_path, (definition_name, collision))
    }

    /// Print statements only have their expression contents so we just hash that.
    /// This means any time what we print is changed we recompile the print statement, but
    /// unlike definitions, this is usually desired.
    pub fn new_print(file_path: Rc<String>, expr: &Expression, collision: u32) -> TopLevelId {
        hash(file_path, (expr, collision))
    }

    pub(crate) fn location(&self, db: &mut CompilerHandle) -> Location {
        let result = incremental::parse_result(self.file_path.clone(), db);
        result.top_level_data[self].location.clone()
    }
}

impl std::fmt::Display for TopLevelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.file_path, self.content_hash)
    }
}

fn hash(file_path: Rc<String>, x: impl std::hash::Hash) -> TopLevelId {
    let mut hasher = std::hash::DefaultHasher::new();
    x.hash(&mut hasher);
    TopLevelId { file_path, content_hash: hasher.finish() }
}

/// An ExprId is a bit different from a top-level id in that we make no attempt
/// to keep these stable across minor changes over multiple compilations. Each
/// new expressions simply receives the next available ExprId from a counter.
///
/// These are however kept independent from each `TopLevelStatement`. Each `TopLevelStatement`
/// that may contain an expression (definitions and print statements) has its own
/// context where expression ids start from zero. This way, although changing any
/// expression within a top-level statement will cause the entire statement to change,
/// this change is still isolated from any other top-level statement in the program.
///
/// These can afford to be a bit smaller than `TopLevelId`s since they're reset for each
/// `TopLevelStatement` and they're generated from a monotonically-increasing counter
/// rather than a hash.
///
/// Since the Ast is immutable, these ExprIds are used to associate more data with
/// a particular node. For example, name resolution fills out any links to definitions,
/// and type inference associates a type with every ExprId.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExprId(u32);

impl ExprId {
    pub fn new(id: u32) -> ExprId {
        ExprId(id)
    }

    pub(crate) fn location(&self, item: &TopLevelId, db: &mut CompilerHandle) -> Location {
        let result = incremental::parse_result(item.file_path.clone(), db);
        result.top_level_data[item].expr_locations[self].clone()
    }
}

impl std::fmt::Display for ExprId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
