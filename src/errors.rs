use crate::lexer::tokens::Token;

pub enum Error {
    /// Expected {expected} but found {found}
    ExpectedToken { expected: Token, found: Option<Token> },

    /// Expected {rule} but found {found}
    ExpectedRule { rule: &'static str, found: Option<Token> },
}

impl Error {
    pub fn message(&self) -> String {
        match self {
            Error::ExpectedToken { expected, found } => {
                let found = found.as_ref().map_or("(end of input)".to_string(), ToString::to_string);
                format!("Expected `{expected}` but found `{found}`")
            }
            Error::ExpectedRule { rule, found } => {
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
