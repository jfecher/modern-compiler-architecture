use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Token {
    /// `:`
    Colon,
    /// `def`
    Def,
    /// `=`
    Equals,
    /// `fn`
    Fn,
    /// `import`
    Import,
    /// `Int`
    Int,
    /// An integer literal - these must be positive i64 values
    Integer(i64),
    /// `-`
    Minus,
    /// `{0}` (the given string)
    Name(String),
    /// `(`
    ParenLeft,
    /// `)`
    ParenRight,
    /// `+`
    Plus,
    /// `print`
    Print,
    /// `->`
    RightArrow,
    /// This character is not in the language - it is an error.
    /// We treat it as a token though since the lexer shouldn't error. It will get to the
    /// parser and the parser can error instead and decide how to recover.
    Unexpected(char),
}

impl Token {
    pub fn can_start_top_level_statement(&self) -> bool {
        matches!(self, Token::Def | Token::Import | Token::Print)
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Token::Colon => write!(f, ":"),
            Token::Def => write!(f, "def"),
            Token::Equals => write!(f, "="),
            Token::Fn => write!(f, "fn"),
            Token::Import => write!(f, "import"),
            Token::Int => write!(f, "Int"),
            Token::Integer(x) => write!(f, "{x}"),
            Token::Minus => write!(f, "-"),
            Token::Name(name) => write!(f, "{name}"),
            Token::ParenLeft => write!(f, "("),
            Token::ParenRight => write!(f, ")"),
            Token::Plus => write!(f, "+"),
            Token::Print => write!(f, "print"),
            Token::RightArrow => write!(f, "->"),
            Token::Unexpected(c) => write!(f, "{c}"),
        }
    }
}
