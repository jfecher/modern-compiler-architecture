use std::rc::Rc;

use crate::errors::Location;

#[derive(Debug)]
pub struct Ast {
    pub statements: Vec<TopLevelStatement>,
}

#[derive(Debug)]
pub enum TopLevelStatement {
    Import { file_name: Identifier },
    Definition(Definition),
    Print(Rc<Expression>),
}

#[derive(Debug)]
pub struct Identifier {
    pub name: Rc<String>,
    pub location: Location,
}

#[derive(Debug)]
pub struct Definition {
    pub name: Identifier,
    pub typ: Option<Type>,
    pub body: Rc<Expression>,
}

#[derive(Debug)]
pub enum Expression {
    IntegerLiteral(i64),
    Variable { name: Identifier },
    FunctionCall { function: Rc<Expression>, argument: Rc<Expression> },
    Lambda { parameter_name: Identifier, body: Rc<Expression> },
}

#[derive(Debug)]
pub enum Type {
    Int,
    Generic { name: Identifier },
    Function { parameter: Rc<Type>, return_type: Rc<Type> },
}
