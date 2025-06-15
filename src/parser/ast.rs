use std::rc::Rc;

use super::ids::{ExprId, TopLevelId};

#[derive(Debug, PartialEq, Eq)]
pub struct Ast {
    pub statements: Vec<TopLevelStatement>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TopLevelStatement {
    Import { file_name: Identifier, id: TopLevelId },
    Definition(Definition),

    /// Print statements don't take a top-level id simply because we never
    /// need any more associated data for them.
    /// - Their type? Always `Unit`
    /// - Their location? We never happen to issue errors for them. In a real compiler we likely would.
    Print(Rc<Expression>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Identifier {
    pub name: Rc<String>,
    pub id: ExprId,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Definition {
    pub name: Identifier,
    pub typ: Option<Type>,
    pub body: Rc<Expression>,
    pub id: TopLevelId,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expression {
    IntegerLiteral(i64, ExprId),
    Variable(Identifier),
    FunctionCall { function: Rc<Expression>, argument: Rc<Expression>, id: ExprId },
    Lambda { parameter_name: Identifier, body: Rc<Expression>, id: ExprId },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Generic(Identifier),
    Function { parameter: Rc<Type>, return_type: Rc<Type> },
}
