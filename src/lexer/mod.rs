use std::{sync::Arc, str::CharIndices};
use tokens::Token;

use crate::{
    errors::{Location, LocationData, Position},
    incremental::{CompilerHandle, get_source_file},
};

pub mod tokens;

/// Lex an entire file, returning a vector of tokens in the file
pub fn lex_file(file_name: Arc<String>, db: &CompilerHandle) -> Vec<(Token, Location)> {
    let source_file_text = get_source_file(file_name.clone(), db);
    let lexer = Lexer::new(&source_file_text, file_name);
    // Calls `self.next()` until it returns `None`, collecting
    // all tokens into a `Vec<Token>`
    lexer.collect()
}

struct Lexer<'src> {
    /// `CharIndices` in Rust is an iterator which iterates over
    /// a string's characters and provides the index for each character.
    source_file: CharIndices<'src>,
    source_file_len: usize,

    current_char: char,
    next_char: char,

    file_name: Arc<String>,
    current_position: Position,

    /// Rust uses UTF-8 so the byte index of the current and next character may
    /// be more than 1 byte apart.
    current_byte_index: usize,
    next_byte_index: usize,
}

impl<'src> Lexer<'src> {
    fn new(source_file_text: &'src str, file_name: Arc<String>) -> Lexer<'src> {
        let mut lexer = Lexer {
            source_file_len: source_file_text.len(),
            source_file: source_file_text.char_indices(),
            current_char: '\0',
            next_char: '\0',
            current_byte_index: 0,
            next_byte_index: 0,
            file_name,
            current_position: Position::start(),
        };
        lexer.advance();
        lexer.advance();
        lexer.current_position = Position::start();
        lexer
    }

    /// Advance the position in the input by 1 character, updating
    /// the values of `self.current_char`, `self.next_char`, and `self.current_index`.
    ///
    /// If there is no remaining input, the next character is set to '\0' instead.
    fn advance(&mut self) {
        (self.current_byte_index, self.current_char) = (self.next_byte_index, self.next_char);
        (self.next_byte_index, self.next_char) = self.source_file.next().unwrap_or((self.source_file_len, '\0'));

        self.current_position.byte_index = self.current_byte_index;
        self.current_position.column_number += 1;

        if self.current_char == '\n' {
            self.current_position.line_number += 1;
            self.current_position.column_number = 0;
        }
    }

    fn location(&self, start: Position, end: Position) -> Location {
        Arc::new(LocationData { file_name: self.file_name.clone(), start, end })
    }

    /// Create a Location with the end Position being `self.current_position`
    fn location_from(&self, start: Position) -> Location {
        self.location(start, self.current_position)
    }

    fn next_token(&mut self) -> Option<(Token, Location)> {
        let start = self.current_position;

        let advance_with = |this: &mut Self, token| {
            this.advance();
            Some((token, this.location_from(start)))
        };

        match self.current_char {
            '=' => advance_with(self, Token::Equals),
            ':' => advance_with(self, Token::Colon),
            '-' if self.next_char == '>' => {
                self.advance();
                self.advance();
                Some((Token::RightArrow, self.location_from(start)))
            },
            '-' => advance_with(self, Token::Minus),
            '+' => advance_with(self, Token::Plus),
            '(' => advance_with(self, Token::ParenLeft),
            ')' => advance_with(self, Token::ParenRight),
            '/' if self.next_char == '/' => {
                while self.current_char != '\0' && self.current_char != '\n' {
                    self.advance();
                }
                self.next_token()
            },
            c if c.is_whitespace() => self.lex_whitespace(),
            c if c.is_ascii_digit() => self.lex_integer(),
            c if c.is_alphanumeric() => self.lex_word(),
            '\0' => None, // End of input
            // Unexpected token. We can't error so give it to the parser to error there
            unexpected => advance_with(self, Token::Unexpected(unexpected)),
        }
    }

    /// Lex whitespace by skipping it and returning the token after it
    fn lex_whitespace(&mut self) -> Option<(Token, Location)> {
        while self.current_char.is_whitespace() {
            self.advance();
        }
        self.next_token()
    }

    /// When lexing a word we have to see if it is a keyword or an arbitrary name
    fn lex_word(&mut self) -> Option<(Token, Location)> {
        let mut word = String::new();
        let start = self.current_position;

        while self.current_char.is_alphanumeric() || self.current_char == '_' {
            word.push(self.current_char);
            self.advance();
        }

        let token = match word.as_str() {
            "def" => Token::Def,
            "fn" => Token::Fn,
            "import" => Token::Import,
            "Int" => Token::Int,
            "print" => Token::Print,
            _other => Token::Name(word),
        };

        let location = self.location_from(start);
        Some((token, location))
    }

    /// Lex a positive, 64-bit integer
    fn lex_integer(&mut self) -> Option<(Token, Location)> {
        let mut integer = 0;
        let start = self.current_position;

        while self.current_char.is_ascii_digit() {
            let digit =
                self.current_char.to_digit(10).expect("We already verified this is a valid ascii base-10 digit");
            integer = integer * 10 + digit as i64;
            self.advance();
        }

        let location = self.location_from(start);
        Some((Token::Integer(integer), location))
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = (Token, Location);
    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}
