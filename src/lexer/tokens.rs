
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
    RightArrow
}
