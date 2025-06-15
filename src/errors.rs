use std::rc::Rc;

use crate::lexer::tokens::Token;

pub type Location = Rc<LocationData>;

#[derive(Debug, PartialEq, Eq)]
pub struct LocationData {
    pub file_name: Rc<String>,
    pub start: Position,
    pub end: Position,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

#[derive(PartialEq, Eq)]
pub enum Error {
    /// Expected {expected} but found {found}
    ExpectedToken { expected: Token, found: Option<Token>, location: Location },

    /// Expected {rule} but found {found}
    ExpectedRule { rule: &'static str, found: Option<Token>, location: Location },
}

impl Error {
    pub fn message(&self) -> String {
        match self {
            Error::ExpectedToken { expected, found, location: _ } => {
                let found = found.as_ref().map_or("(end of input)".to_string(), ToString::to_string);
                format!("Expected `{expected}` but found `{found}`")
            }
            Error::ExpectedRule { rule, found, location: _ } => {
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
