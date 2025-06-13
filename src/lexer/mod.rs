use std::str::CharIndices;

use tokens::Token;

pub mod tokens;

/// Lex an entire file, returning a vector of tokens in the file
pub fn lex_file(source_file_text: &str) -> Vec<Token> {
    let lexer = Lexer::new(source_file_text);
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

    /// Rust uses UTF-8 so the byte index of the current and next character may
    /// be more than 1 byte apart.
    current_byte_index: usize,
    next_byte_index: usize,
}

impl<'src> Lexer<'src> {
    fn new(source_file_text: &'src str) -> Lexer<'src> {
        let mut lexer = Lexer {
            source_file_len: source_file_text.len(),
            source_file: source_file_text.char_indices(),
            current_char: '\0',
            next_char: '\0',
            current_byte_index: 0,
            next_byte_index: 0,
        };
        lexer.advance();
        lexer.advance();
        lexer
    }

    /// Advance the position in the input by 1 character, updating
    /// the values of `self.current_char`, `self.next_char`, and `self.current_index`.
    ///
    /// If there is no remaining input, the next character is set to '\0' instead.
    fn advance(&mut self) {
        (self.current_byte_index, self.current_char) = (self.next_byte_index, self.next_char);
        (self.next_byte_index, self.next_char) = self.source_file.next().unwrap_or((self.source_file_len, '\0'));
    }

    fn next_token(&mut self) -> Option<Token> {
        match self.current_char {
            '=' => {
                self.advance();
                Some(Token::Equals)
            }
            ':' => {
                self.advance();
                Some(Token::Colon)
            }
            '-' if self.next_char == '>' => {
                self.advance();
                self.advance();
                Some(Token::RightArrow)
            }
            '-' => {
                self.advance();
                Some(Token::Minus)
            }
            '+' => {
                self.advance();
                Some(Token::Plus)
            }
            c if c.is_whitespace() => self.lex_whitespace(),
            c if c.is_alphanumeric() => self.lex_word(),
            c if c.is_ascii_digit() => self.lex_integer(),
            '\0' => None, // End of input
            unexpected => {
                // Unexpected token
                todo!("Unexpected char `{unexpected}`")
            }
        }
    }

    /// Lex whitespace by skipping it and returning the token after it
    fn lex_whitespace(&mut self) -> Option<Token> {
        while self.current_char.is_whitespace() {
            self.advance();
        }
        self.next_token()
    }

    /// When lexing a word we have to see if it is a keyword or an arbitrary name
    fn lex_word(&mut self) -> Option<Token> {
        let mut word = String::new();

        while self.current_char.is_alphanumeric() {
            word.push(self.current_char);
            self.advance();
        }

        match word.as_str() {
            "def" => Some(Token::Def),
            "fn" => Some(Token::Fn),
            "Int" => Some(Token::Int),
            "module" => Some(Token::Module),
            "print" => Some(Token::Print),
            _other => Some(Token::Name(word)),
        }
    }

    /// Lex a positive, 64-bit integer
    fn lex_integer(&mut self) -> Option<Token> {
        let mut integer = 0;

        while self.current_char.is_ascii_digit() {
            let digit = self.current_char.to_digit(10).expect("We already verified this is a valid ascii base-10 digit");
            integer = integer * 10 + digit as i64;
            self.advance();
        }

        Some(Token::Integer(integer))
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}
