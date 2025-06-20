use std::rc::Rc;

use serde::{Deserialize, Serialize};

use super::ids::{ExprId, TopLevelId};

pub type Ast = Rc<Program>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub statements: Vec<TopLevelStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopLevelStatement {
    Import { file_name: Identifier, id: TopLevelId },
    Definition(Definition),
    Print(Rc<Expression>, TopLevelId),
}

impl TopLevelStatement {
    pub fn id(&self) -> &TopLevelId {
        match self {
            TopLevelStatement::Import { id, .. } => id,
            TopLevelStatement::Definition(definition) => &definition.id,
            TopLevelStatement::Print(_, id) => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identifier {
    pub name: Rc<String>,
    pub id: ExprId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Definition {
    pub name: Identifier,
    pub typ: Option<Type>,
    pub body: Rc<Expression>,
    pub id: TopLevelId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expression {
    IntegerLiteral(i64, ExprId),
    Variable(Identifier),
    FunctionCall { function: Rc<Expression>, argument: Rc<Expression>, id: ExprId },
    Lambda { parameter_name: Identifier, body: Rc<Expression>, id: ExprId },
}

impl Expression {
    pub fn id(&self) -> ExprId {
        match self {
            Expression::IntegerLiteral(_, id) => *id,
            Expression::Variable(identifier) => identifier.id,
            Expression::FunctionCall { id, .. } => *id,
            Expression::Lambda { id, .. } => *id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    Int,
    Generic(Identifier),
    Function { parameter: Rc<Type>, return_type: Rc<Type> },
}
