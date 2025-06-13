
#[derive(Debug, PartialEq, Eq)]
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
