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

impl LocationData {
    /// Merge two locations
    pub fn to(&self, end: &LocationData) -> Location {
        assert_eq!(self.file_name, end.file_name);
        Rc::new(LocationData { file_name: self.file_name.clone(), start: self.start, end: end.end })
    }
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

pub type Errors = Vec<Error>;

/// Any diagnostic that the compiler can issue
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    ParserExpected { rule: String, found: Option<Token>, location: Location },
    NameAlreadyInScope { name: Rc<String>, first_location: Location, second_location: Location },
    ImportedNameAlreadyInScope { name: Rc<String>, first_location: Location, second_location: Location },
    UnknownImportFile { file_name: Rc<String>, location: Location },
    NameNotInScope { name: Rc<String>, location: Location },
    ExpectedType { actual: String, expected: String, location: Location },
    RecursiveType { typ: String, location: Location },
}

impl Error {
    pub fn message(&self) -> String {
        match self {
            Error::ParserExpected { rule, found, location } => {
                let found = found.as_ref().map_or("(end of input)".to_string(), ToString::to_string);
                format!("{location}: Expected {rule} but found `{found}`")
            },
            Error::NameAlreadyInScope { name, first_location, second_location } => {
                format!("{second_location}: `{name}` was already defined at {first_location}")
            },
            Error::ImportedNameAlreadyInScope { name, first_location, second_location } => {
                format!(
                    "{second_location}: This imports `{name}`, which has already been defined here: {first_location}"
                )
            },
            Error::UnknownImportFile { file_name, location } => {
                format!("{location}: Cannot read source file `{file_name}`, does it exist?")
            },
            Error::NameNotInScope { name, location } => {
                format!("{location}: `{name}` is not defined, was it a typo?")
            },
            Error::ExpectedType { actual, expected, location } => {
                format!("{location}: Expected type `{expected}` but found `{actual}`")
            },
            Error::RecursiveType { typ, location } => {
                format!("{location}: Binding here would create an infinitely recursive type with `{typ}`")
            },
        }
    }
}

impl std::fmt::Display for LocationData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.file_name, self.start.line_number)
    }
}
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}
