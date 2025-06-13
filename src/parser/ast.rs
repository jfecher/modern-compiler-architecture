use std::rc::Rc;

#[derive(Debug)]
pub struct Ast {
    pub statements: Vec<TopLevelStatement>,
}

#[derive(Debug)]
pub enum TopLevelStatement {
    Import { file_name: String },
    Definition(Definition),
    Print(Rc<Expression>),
}

#[derive(Debug)]
pub struct Definition {
    pub name: String,
    pub typ: Option<Type>,
    pub body: Rc<Expression>,
}

#[derive(Debug)]
pub enum Expression {
    IntegerLiteral(i64),
    Variable { name: String },
    FunctionCall { function: Rc<Expression>, argument: Rc<Expression> },
    Lambda { parameter_name: String, body: Rc<Expression> },
}

#[derive(Debug)]
pub enum Type {
    Int,
    Generic { name: String },
    Function { parameter: Rc<Type>, return_type: Rc<Type> },
}
