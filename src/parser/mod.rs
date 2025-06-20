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
use std::{collections::BTreeMap, rc::Rc};

use ast::{Ast, Definition, Expression, Identifier, Program, TopLevelStatement, Type};
use ids::{ExprId, TopLevelId};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{Error, Errors, Location, LocationData, Position},
    incremental::{self, CompilerHandle, Parse},
    lexer::{self, tokens::Token},
};

pub mod ast;
mod ast_printer;
pub mod ids;

struct Parser {
    tokens: Vec<(Token, Location)>,
    current_token_index: usize,

    file_name: Rc<String>,
    errors: Vec<Error>,

    /// Each expression within a top-level statement receives a monotonically increasing
    /// ExprId. This value starts at 0 and is reset in each top-level statement.
    next_expr_id: u32,

    /// Each ExprId is only valid within the context of a TopLevelId, so this will
    /// need to go inside `top_level_data` when we are finished with the current top
    /// level item. Until then it is easier to have this separate so that we do not need
    /// a TopLevelId to push an expression's location.
    expr_locations: BTreeMap<ExprId, Location>,

    top_level_data: BTreeMap<TopLevelId, TopLevelMetaData>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParserResult {
    pub ast: Ast,
    pub errors: Errors,
    pub top_level_data: BTreeMap<TopLevelId, TopLevelMetaData>,
}

/// Additional metadata on a TopLevelStatement
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopLevelMetaData {
    pub location: Location,
    pub expr_locations: BTreeMap<ExprId, Location>,
}

pub fn parse_impl(params: &Parse, db: &mut CompilerHandle) -> ParserResult {
    incremental::enter_query();
    incremental::println(format!("Parsing {}", params.file_name));

    let tokens = lexer::lex_file(params.file_name.clone(), db);
    let mut parser = Parser::new(params.file_name.clone(), tokens);
    let ast = parser.parse();

    incremental::exit_query();
    ParserResult { ast: Rc::new(ast), errors: parser.errors, top_level_data: parser.top_level_data }
}

impl Parser {
    fn new(file_name: Rc<String>, tokens: Vec<(Token, Location)>) -> Self {
        Parser {
            file_name,
            tokens,
            errors: Vec::new(),
            current_token_index: 0,
            next_expr_id: 0,
            top_level_data: BTreeMap::new(),
            expr_locations: BTreeMap::new(),
        }
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
                },
            },
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

    /// Creates a new ExprId and stores the given Location at that id
    fn next_expr_id(&mut self, location: Location) -> ExprId {
        let id = ExprId::new(self.next_expr_id);
        self.next_expr_id += 1;
        self.expr_locations.insert(id, location);
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
            let rule = format!("`{token}`");
            Err(Error::ParserExpected { rule, found: actual, location })
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
    fn parse(&mut self) -> Program {
        let statements = self.parse_top_level_statements();

        if !self.current_token().is_none() {
            // We have unparsed input
            let (found, location) = self.current_token_and_location();
            let found = found.cloned();
            let rule = "top level statement".to_string();
            self.errors.push(Error::ParserExpected { rule, found, location });
        }

        Program { statements }
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
                let rule = "top level statement".to_string();
                self.errors.push(Error::ParserExpected { rule, found, location });
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
                },
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
        let token = self.current_token().expect(
            "`parse_top_level_statements` should ensure this method isn't called when we're at the end of input",
        );

        assert!(token.can_start_top_level_statement());

        match token {
            Token::Def => self.parse_definition(),
            Token::Import => self.parse_import(),
            Token::Print => self.parse_print(),
            _ => unreachable!(
                "parse_top_level_statement should only be called on a token which may start a top_level_statement"
            ),
        }
    }

    /// Stores the location of a TopLevelItem, as well as the location and information
    /// of any ExprIds within this item.
    fn store_top_level_metadata(&mut self, id: TopLevelId, location: Location) {
        let meta = TopLevelMetaData { location, expr_locations: std::mem::take(&mut self.expr_locations) };
        self.top_level_data.insert(id.clone(), meta);
    }

    /// definition: "def" name (":" type)? "=" expr
    fn parse_definition(&mut self) -> Result<TopLevelStatement, Error> {
        let start = self.current_location();
        self.expect(Token::Def)?;

        let name = self.parse_name()?;

        let mut typ = None;
        if self.accept(Token::Colon) {
            typ = Some(self.parse_type()?);
        }

        self.expect(Token::Equals)?;
        let body = Rc::new(self.parse_expr()?);

        // TODO: Handle collisions
        let id = TopLevelId::new_definition(self.file_name.clone(), &name.name, 0);
        let location = start.to(&self.current_location());
        self.store_top_level_metadata(id.clone(), location);

        Ok(TopLevelStatement::Definition(Definition { name, typ, body, id }))
    }

    /// import: "import" name
    fn parse_import(&mut self) -> Result<TopLevelStatement, Error> {
        let start = self.current_location();
        self.expect(Token::Import)?;
        let mut file_name = self.parse_name()?;

        // Hack: Adding the .ex suffix here lets us share this suffix in the Rc
        // much more easily without having to cache it and add code to translate between
        // the module name and the file name everywhere else.
        file_name.name = Rc::new(format!("{}.ex", file_name.name));

        // TODO: Handle collisions
        let id = TopLevelId::new_import(self.file_name.clone(), &file_name.name, 0);
        let location = start.to(&self.current_location());
        self.store_top_level_metadata(id.clone(), location);

        Ok(TopLevelStatement::Import { file_name, id })
    }

    /// print: "print" expr
    fn parse_print(&mut self) -> Result<TopLevelStatement, Error> {
        let start = self.current_location();
        self.expect(Token::Print)?;
        let expr = self.parse_expr()?;
        let location = start.to(&self.current_location());

        // TODO: Handle collisions
        let id = TopLevelId::new_print(self.file_name.clone(), &expr, 0);
        self.store_top_level_metadata(id.clone(), location);

        Ok(TopLevelStatement::Print(Rc::new(expr), id))
    }

    /// expr: lambda | infix_expr
    fn parse_expr(&mut self) -> Result<Expression, Error> {
        if self.current_token() == Some(&Token::Fn) { self.parse_lambda() } else { self.parse_infix_expr() }
    }

    /// lambda: "fn" name+ "->" expr
    fn parse_lambda(&mut self) -> Result<Expression, Error> {
        let start = self.current_location();
        self.expect(Token::Fn)?;
        let mut parameters = vec![self.parse_name()?];

        // The remaining parameters are optional so don't error if they're not there
        while let Ok(arg) = self.parse_name() {
            parameters.push(arg);
        }

        self.expect(Token::RightArrow)?;
        let body = self.parse_expr()?;
        let location = start.to(&self.current_location());

        // Lambdas with more than one parameter are desugared into nested lambdas
        // each with exactly one parameter
        let mut expr = body;
        for parameter_name in parameters.into_iter().rev() {
            let body = Rc::new(expr);
            let id = self.next_expr_id(location.clone());
            expr = Expression::Lambda { parameter_name, body, id };
        }

        Ok(expr)
    }

    /// expr: expr + call
    ///     | expr - call
    ///     | call
    fn parse_infix_expr(&mut self) -> Result<Expression, Error> {
        let start = self.current_location();
        let mut expr = self.parse_call()?;

        // `a + b` and `a - b` are represented as function calls: `(+) a b` and `(-) a b`
        let operator = |this: &mut Self, name: &str, expr| -> Result<_, Error> {
            let operator_location = this.current_location();
            this.advance();
            let id = this.next_expr_id(operator_location);
            let name = Identifier { name: Rc::new(name.into()), id };

            let function = Rc::new(Expression::Variable(name));
            let lhs = Rc::new(expr);
            let rhs = Rc::new(this.parse_call()?);
            let call_location = start.to(&this.current_location());

            let id = this.next_expr_id(call_location.clone());
            let call1 = Rc::new(Expression::FunctionCall { function, argument: lhs, id });
            let id = this.next_expr_id(call_location);
            Ok(Expression::FunctionCall { function: call1, argument: rhs, id })
        };

        while matches!(self.current_token(), Some(Token::Plus | Token::Minus)) {
            expr = operator(self, "+", expr)?;
        }
        
        Ok(expr)
    }

    /// call: call atom
    ///     | atom
    fn parse_call(&mut self) -> Result<Expression, Error> {
        let start = self.current_location();
        let mut atom = self.parse_atom()?;

        while let Ok(argument) = self.parse_atom() {
            let function = Rc::new(atom);
            let argument = Rc::new(argument);
            let location = start.to(&self.current_location());
            atom = Expression::FunctionCall { function, argument, id: self.next_expr_id(location) };
        }

        Ok(atom)
    }

    /// atom: name | integer | "(" expr ")"
    fn parse_atom(&mut self) -> Result<Expression, Error> {
        match self.current_token_and_location() {
            (Some(Token::Name(name)), location) => {
                let name = Rc::new(name.clone());
                let name = Identifier { name, id: self.next_expr_id(location) };
                self.advance();
                Ok(Expression::Variable(name))
            },
            (Some(Token::Integer(x)), location) => {
                let x = *x;
                self.advance();
                Ok(Expression::IntegerLiteral(x, self.next_expr_id(location)))
            },
            (Some(Token::ParenLeft), _) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::ParenRight)?;
                Ok(expr)
            },
            (other, location) => {
                let rule = "an expression".to_string();
                Err(Error::ParserExpected { rule, found: other.cloned(), location })
            },
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
        } else {
            Ok(typ)
        }
    }

    /// basic_type: "Int" | name | "(" type ")"
    fn parse_basic_type(&mut self) -> Result<Type, Error> {
        match self.current_token() {
            Some(Token::Int) => {
                self.advance();
                Ok(Type::Int)
            },
            Some(Token::Name(name)) => {
                let name = Rc::new(name.clone());
                let location = self.current_location();
                let name = Identifier { name, id: self.next_expr_id(location) };
                self.advance();
                Ok(Type::Generic(name))
            },
            Some(Token::ParenLeft) => {
                self.advance();
                let typ = self.parse_type()?;
                self.expect(Token::ParenRight)?;
                Ok(typ)
            },
            other => {
                let location = self.current_location();
                let rule = "a type".to_string();
                Err(Error::ParserExpected { rule, found: other.cloned(), location })
            },
        }
    }

    /// name: [a-zA-Z][a-zA-Z0-9]*
    fn parse_name(&mut self) -> Result<Identifier, Error> {
        match self.current_token_and_location() {
            (Some(Token::Name(name)), location) => {
                let name = Rc::new(name.clone());
                self.advance();
                Ok(Identifier { name, id: self.next_expr_id(location) })
            },
            (other, location) => {
                let found = other.cloned();
                let rule = "a name".to_string();
                Err(Error::ParserExpected { rule, found, location })
            },
        }
    }
}
