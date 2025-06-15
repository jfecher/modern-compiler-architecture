//! This file contains the program's parser.
//!
//! It is a hand-written recursive descent parser which gives us more control over
//! parser recovery. I've used handwritten recursive descent parsers, parser generators,
//! and parser combinators in the past but always recommend hand-writing a recursive
//! descent parser for compilers now. They can give you more control over error messages
//! than parser generators, more control over recovery than parser generators and most
//! parser combinators, and more control over performance than all of the above. They
//! do tend to be more ad-hoc, but this lets us more easily implement less common features.
//! For example, this parser assigns each expression a unique ID using an internal counter.
//! Something as simple as this can be impossible with most parser generators or parser
//! combinators which don't support monadic state.
//!
//! Notable features:
//! - Concurrency: None. We parse a single source file top to bottom.
//! - Incrementality: On each source file's Ast. We always re-parse an entire Ast
//!   when the corresponding file changes, although the Ast after that point can
//!   be split up into each top-level statement so that changes in one statement
//!   do not affect another. See `parser/id.rs` for more information on how
//!   top-level statements are identified as the same definition across multiple
//!   compilations and changes to the source file.
//! - Fault-tolerant: The parser should never fail to produce an Ast. This means
//!   we return an Ast alongside any errors that occurred instead of returning
//!   an Ast _or_ errors. Depending on the source program we may be more or less
//!   successful on how useful the resulting Ast is though. For this example compiler
//!   when a parse error occurs during a top-level statement we simply skip to the
//!   next token in the input which may start a top-level statement. This may be
//!   more difficult if your language doesn't have tokens dedicated to only starting
//!   top-level statements like this example language does. Another good substitute
//!   here would be if you have an error within a block delimited by some brackets: `{  }`
//!   to skip to the ending bracket token `}` and try to continue from there. In
//!   a more mature compiler, you'd expect to see many recover strategies. For example,
//!   if we fail to parse a type we could log the error, create a default `Error` type, and skip
//!   to either the next parameter or an `=` token, depending on the context we're
//!   parsing the type in. The `Error` type (or more generally an `Error` node) is a good
//!   general strategy for parser recovery when you have to return a valid Ast. You
//!   can fill in your missing node with `Error` instead then use that to try to ignore
//!   errors there in the future. For types this means if a type fails to parse you can
//!   also filter out type errors with that error type since error types should always
//!   correctly type check (and should be hidden from users).
use std::rc::Rc;

use ast::{Ast, Definition, Expression, Identifier, TopLevelStatement, Type};
use ids::{ExprId, TopLevelId};
use inc_complete::{Computation, DbHandle};

use crate::{errors::{Error, Location, LocationData, Position}, lexer::{self, tokens::Token}};

pub mod ast;
pub mod ids;
mod ast_printer;

struct Parser {
    tokens: Vec<(Token, Location)>,
    current_token_index: usize,

    file_name: Rc<String>,
    errors: Vec<Error>,

    /// Each expression within a top-level statement receives a monotonically increasing
    /// ExprId. This value starts at 0 and is reset in each top-level statement.
    next_expr_id: u32,
}

pub fn parse_impl(file_name: &Rc<String>, db: &mut DbHandle<impl Computation>) -> (Ast, Vec<Error>) {
    let tokens = lexer::lex_file(file_name.clone(), db);
    let mut parser = Parser::new(file_name.clone(), tokens);
    let ast = parser.parse();
    (ast, parser.errors)
}

impl Parser {
    fn new(file_name: Rc<String>, tokens: Vec<(Token, Location)>) -> Self {
        Parser { file_name, tokens, errors: Vec::new(), current_token_index: 0, next_expr_id: 0 }
    }

    /// Returns the current token, or None if we've reached the end of input
    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.current_token_index).map(|(token, _)| token)
    }

    /// Returns the current location, or the location of the last token if
    /// we've reached the end of input. If there are no tokens at all, an
    /// empty Location is returned.
    fn current_location(&self) -> Location {
        match self.tokens.get(self.current_token_index) {
            Some((_, location)) => location.clone(),
            None => match self.tokens.last() {
                Some((_, location)) => location.clone(),
                None => {
                    // Corner case: file doesn't contain a single token
                    let position = Position::start();
                    let file_name = self.file_name.clone();
                    Rc::new(LocationData { file_name, start: position, end: position })
                }
            }
        }
    }

    /// Returns the current token, or None if we've reached the end of input,
    /// along with the current location. If we've reached the end of input,
    /// the last token's location is used.
    fn current_token_and_location(&self) -> (Option<&Token>, Location) {
        (self.current_token(), self.current_location())
    }

    /// Advance to the next token
    fn advance(&mut self) {
        self.current_token_index += 1;
    }

    /// If the current token is the given token, advance the input, and return true.
    /// Return false otherwise (and do not advance the input).
    fn accept(&mut self, token: Token) -> bool {
        if self.current_token() == Some(&token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn next_expr_id(&mut self) -> ExprId {
        let id = ExprId::new(self.next_expr_id);
        self.next_expr_id += 1;
        id
    }

    /// If the current token is the given token, advance the input.
    /// Otherwise, issue an `expected _ but found _` error.
    fn expect(&mut self, token: Token) -> Result<(), Error> {
        if self.accept(token.clone()) {
            Ok(())
        } else {
            let (actual, location) = self.current_token_and_location();
            let actual = actual.cloned();
            Err(Error::ExpectedToken { expected: token, found: actual, location })
        }
    }

    /// Skips all tokens from the current token until the next token in the
    /// stream for which `predicate` returns false.
    ///
    /// E.g. `parser.skip_while(|token| token == Token::Equals)` will skip
    /// all the `=` tokens and stop at the next token that is not `=`.
    fn skip_while(&mut self, predicate: impl Fn(&Token) -> bool) {
        while self.current_token().is_some_and(&predicate) {
            self.advance();
        }
    }

    /// Parse the program!
    fn parse(&mut self) -> Ast {
        let statements = self.parse_top_level_statements();

        if !self.current_token().is_none() {
            // We have unparsed input
            let (found, location) = self.current_token_and_location();
            let found = found.cloned();
            self.errors.push(Error::ExpectedRule { rule: "top level statement", found, location });
        }

        Ast { statements  }
    }

    /// Recovers to the next top level statement (or the end of input)
    /// by skipping all tokens until one that can start a new top level statement
    fn recover_to_next_top_level_statement(&mut self) {
        self.skip_while(|token| !token.can_start_top_level_statement());
    }

    /// Parse multiple top level statements.
    ///
    /// If any fail to parse, we log the error then skip to the beginning of the
    /// next top level statement and continue parsing from there
    ///
    /// top_level_statements: top_level_statement*
    fn parse_top_level_statements(&mut self) -> Vec<TopLevelStatement> {
        let mut statements = Vec::new();

        while let Some(token) = self.current_token() {
            if !token.can_start_top_level_statement() {
                let found = Some(token.clone());
                let location = self.current_location();
                self.errors.push(Error::ExpectedRule { rule: "top level statement", found, location });
                self.recover_to_next_top_level_statement();

                // We can possibly skip to the end of input above but we're at a valid
                // stopping point so it shouldn't be an error.
                if self.current_token().is_none() {
                    break;
                }
            }

            match self.parse_top_level_statement() {
                Ok(statement) => statements.push(statement),
                Err(error) => {
                    self.errors.push(error);
                    self.recover_to_next_top_level_statement();
                }
            }
        }

        statements
    }

    /// Parse a top level statement - expects the input to already
    /// be on a token such that `token.can_start_top_level_statement()` is true.
    ///
    /// top_level_statement: definition | import | print
    fn parse_top_level_statement(&mut self) -> Result<TopLevelStatement, Error> {
        self.next_expr_id = 0;
        let token = self.current_token()
            .expect("`parse_top_level_statements` should ensure this method isn't called when we're at the end of input");

        assert!(token.can_start_top_level_statement());

        match token {
            Token::Def => self.parse_definition(),
            Token::Import => self.parse_import(),
            Token::Print => self.parse_print(),
            _ => unreachable!("parse_top_level_statement should only be called on a token which may start a top_level_statement"),
        }
    }

    /// definition: "def" name (":" type)? "=" expr
    fn parse_definition(&mut self) -> Result<TopLevelStatement, Error> {
        self.expect(Token::Def)?;
        let name = self.parse_name()?;

        let mut typ = None;
        if self.accept(Token::Colon) {
            typ = Some(self.parse_type()?);
        }

        self.expect(Token::Equals)?;
        let body = Rc::new(self.parse_expr()?);

        // TODO: Handle collisions
        let id = TopLevelId::new_definition(&self.file_name, &name.name, 0);
        Ok(TopLevelStatement::Definition(Definition { name, typ, body, id }))
    }

    /// import: "import" name
    fn parse_import(&mut self) -> Result<TopLevelStatement, Error> {
        self.expect(Token::Import)?;
        let file_name = self.parse_name()?;

        // TODO: Handle collisions
        let id = TopLevelId::new_import(&self.file_name, &file_name.name, 0);
        Ok(TopLevelStatement::Import { file_name, id })
    }

    /// print: "print" expr
    fn parse_print(&mut self) -> Result<TopLevelStatement, Error> {
        self.expect(Token::Print)?;
        let expr = self.parse_expr()?;
        Ok(TopLevelStatement::Print(Rc::new(expr)))
    }

    /// expr: lambda | infix_expr
    fn parse_expr(&mut self) -> Result<Expression, Error> {
        if self.current_token() == Some(&Token::Fn) {
            self.parse_lambda()
        } else {
            self.parse_infix_expr()
        }
    }

    /// lambda: "fn" name+ "->" expr
    fn parse_lambda(&mut self) -> Result<Expression, Error> {
        self.expect(Token::Fn)?;
        let mut parameters = vec![self.parse_name()?];

        // The remaining parameters are optional so don't error if they're not there
        while let Ok(arg) = self.parse_name() {
            parameters.push(arg);
        }

        self.expect(Token::RightArrow)?;
        let body = self.parse_expr()?;

        // Lambdas with more than one parameter are desugared into nested lambdas
        // each with exactly one parameter
        let mut expr = body;
        for parameter_name in parameters.into_iter().rev() {
            let body = Rc::new(expr);
            let id = self.next_expr_id();
            expr = Expression::Lambda { parameter_name, body, id };
        }

        Ok(expr)
    }

    /// expr: expr + call
    ///     | expr - call
    ///     | call
    fn parse_infix_expr(&mut self) -> Result<Expression, Error> {
        let expr = self.parse_call()?;

        // `a + b` and `a - b` are represented as function calls: `(+) a b` and `(-) a b`
        let operator = |this: &mut Self, name: &str, expr| -> Result<_, Error> {
            this.advance();
            let id = this.next_expr_id();
            let name = Identifier { name: Rc::new(name.into()), id };
            let function = Rc::new(Expression::Variable(name));
            let lhs = Rc::new(expr);
            let rhs = Rc::new(this.parse_call()?);

            let id = this.next_expr_id();
            let call1 = Rc::new(Expression::FunctionCall { function, argument: lhs, id });
            let id = this.next_expr_id();
            Ok(Expression::FunctionCall { function: call1, argument: rhs, id })
        };

        match self.current_token() {
            Some(Token::Plus) => operator(self, "+", expr),
            Some(Token::Minus) => operator(self, "-", expr),
            _ => Ok(expr),
        }
    }

    /// call: call atom
    ///     | atom
    fn parse_call(&mut self) -> Result<Expression, Error> {
        let mut atom = self.parse_atom()?;

        while let Ok(argument) = self.parse_atom() {
            let function = Rc::new(atom);
            let argument = Rc::new(argument);
            atom = Expression::FunctionCall { function, argument, id: self.next_expr_id() };
        }

        Ok(atom)
    }

    /// atom: name | integer | "(" expr ")"
    fn parse_atom(&mut self) -> Result<Expression, Error> {
        match self.current_token() {
            Some(Token::Name(name)) => {
                let name = Rc::new(name.clone());
                let name = Identifier { name, id: self.next_expr_id() };
                self.advance();
                Ok(Expression::Variable(name))
            }
            Some(Token::Integer(x)) => {
                let x = *x;
                self.advance();
                Ok(Expression::IntegerLiteral(x, self.next_expr_id()))
            }
            Some(Token::ParenLeft) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::ParenRight)?;
                Ok(expr)
            }
            other => {
                let location = self.current_location();
                Err(Error::ExpectedRule { rule: "an expression", found: other.cloned(), location })
            }
        }
    }

    /// type: basic_type
    ///     | basic_type "->" type
    fn parse_type(&mut self) -> Result<Type, Error> {
        let typ = self.parse_basic_type()?;

        if self.accept(Token::RightArrow) {
            let parameter = Rc::new(typ);
            let return_type = Rc::new(self.parse_type()?);
            Ok(Type::Function { parameter, return_type })
        }  else {
            Ok(typ)
        }
    }

    /// basic_type: "Int" | name | "(" type ")"
    fn parse_basic_type(&mut self) -> Result<Type, Error> {
        match self.current_token() {
            Some(Token::Int) => {
                self.advance();
                Ok(Type::Int)
            }
            Some(Token::Name(name)) => {
                let name = Rc::new(name.clone());
                let name = Identifier { name, id: self.next_expr_id() };
                self.advance();
                Ok(Type::Generic(name))
            }
            Some(Token::ParenLeft) => {
                self.advance();
                let typ = self.parse_type()?;
                self.expect(Token::ParenRight)?;
                Ok(typ)
            }
            other => {
                let location = self.current_location();
                Err(Error::ExpectedRule { rule: "a type", found: other.cloned(), location })
            }
        }
    }

    /// name: [a-zA-Z][a-zA-Z0-9]*
    fn parse_name(&mut self) -> Result<Identifier, Error> {
        match self.current_token() {
            Some(Token::Name(name)) => {
                let name = Rc::new(name.clone());
                self.advance();
                Ok(Identifier { name, id: self.next_expr_id() })
            }
            other => {
                let found = other.cloned();
                let location = self.current_location();
                Err(Error::ExpectedRule { rule: "a name", found, location })
            }
        }
    }
}
