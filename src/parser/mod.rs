use std::rc::Rc;

use ast::{Ast, Definition, Expression, Identifier, TopLevelStatement, Type};

use crate::{errors::{Error, Location, LocationData, Position}, lexer::tokens::Token};

pub mod ast;

struct Parser {
    tokens: Vec<(Token, Location)>,
    current_token_index: usize,

    file_name: Rc<String>,
    errors: Vec<Error>
}

pub fn parse(file_name: Rc<String>, tokens: Vec<(Token, Location)>) -> (Ast, Vec<Error>) {
    let mut parser = Parser::new(file_name, tokens);
    let ast = parser.parse();
    (ast, parser.errors)
}

impl Parser {
    fn new(file_name: Rc<String>, tokens: Vec<(Token, Location)>) -> Self {
        Parser { file_name, tokens, errors: Vec::new(), current_token_index: 0 }
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

        Ok(TopLevelStatement::Definition(Definition { name, typ, body }))
    }

    /// import: "import" name
    fn parse_import(&mut self) -> Result<TopLevelStatement, Error> {
        self.expect(Token::Import)?;
        let file_name = self.parse_name()?;
        Ok(TopLevelStatement::Import { file_name })
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
            expr = Expression::Lambda { parameter_name, body };
        }

        Ok(expr)
    }

    /// expr: expr + call
    ///     | expr - call
    ///     | call
    fn parse_infix_expr(&mut self) -> Result<Expression, Error> {
        let expr = self.parse_call()?;

        // `a + b` and `a - b` are represented as function calls: `(+) a b` and `(-) a b`
        let operator = |this: &mut Self, name: &str, expr, location| -> Result<_, Error> {
            this.advance();
            let name = Identifier { name: Rc::new(name.into()), location };
            let function = Rc::new(Expression::Variable { name });
            let lhs = Rc::new(expr);
            let rhs = Rc::new(this.parse_call()?);

            let call1 = Rc::new(Expression::FunctionCall { function, argument: lhs });
            Ok(Expression::FunctionCall { function: call1, argument: rhs })
        };

        match self.current_token_and_location() {
            (Some(Token::Plus), location) => operator(self, "+", expr, location),
            (Some(Token::Minus), location) => operator(self, "-", expr, location),
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
            atom = Expression::FunctionCall { function, argument };
        }

        Ok(atom)
    }

    /// atom: name | integer | "(" expr ")"
    fn parse_atom(&mut self) -> Result<Expression, Error> {
        match self.current_token_and_location() {
            (Some(Token::Name(name)), location) => {
                let name = Rc::new(name.clone());
                let name = Identifier { name, location };
                self.advance();
                Ok(Expression::Variable { name })
            }
            (Some(Token::Integer(x)), _) => {
                let x = *x;
                self.advance();
                Ok(Expression::IntegerLiteral(x))
            }
            (Some(Token::ParenLeft), _) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::ParenRight)?;
                Ok(expr)
            }
            (other, location) => {
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
        match self.current_token_and_location() {
            (Some(Token::Int), _) => {
                self.advance();
                Ok(Type::Int)
            }
            (Some(Token::Name(name)), location) => {
                let name = Rc::new(name.clone());
                let name = Identifier { name, location };
                self.advance();
                Ok(Type::Generic { name })
            }
            (Some(Token::ParenLeft), _) => {
                self.advance();
                let typ = self.parse_type()?;
                self.expect(Token::ParenRight)?;
                Ok(typ)
            }
            (other, location) => {
                Err(Error::ExpectedRule { rule: "a type", found: other.cloned(), location })
            }
        }
    }

    /// name: [a-zA-Z][a-zA-Z0-9]*
    fn parse_name(&mut self) -> Result<Identifier, Error> {
        match self.current_token_and_location() {
            (Some(Token::Name(name)), location) => {
                let name = Rc::new(name.clone());
                self.advance();
                Ok(Identifier { name, location })
            }
            (other, location) => {
                let found = other.cloned();
                Err(Error::ExpectedRule { rule: "a name", found, location })
            }
        }
    }
}
