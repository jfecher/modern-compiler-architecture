use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::lexer::tokens::Token;

pub type Location = Rc<LocationData>;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationData {
    pub file_name: Rc<String>,
    pub start: Position,
    pub end: Position,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub byte_index: usize,
    pub line_number: u32,
    pub column_number: u32,
}

impl Position {
    pub fn start() -> Position {
        Position { byte_index: 0, line_number: 1, column_number: 1 }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    /// Expected {rule} but found {found}
    ParserExpected { rule: String, found: Option<Token>, location: Location },
}

impl Error {
    pub fn message(&self) -> String {
        match self {
            Error::ParserExpected { rule, found, location: _ } => {
                let found = found.as_ref().map_or("(end of input)".to_string(), ToString::to_string);
                format!("Expected {rule} but found `{found}`")
            },
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}
