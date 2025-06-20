use super::ast::{Definition, Expression, Identifier, Program, TopLevelStatement, Type};

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, statement) in self.statements.iter().enumerate() {
            if i != 0 {
                writeln!(f)?;
            }
            write!(f, "{statement}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for TopLevelStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopLevelStatement::Import { file_name, id: _ } => {
                write!(f, "import {file_name}")
            },
            TopLevelStatement::Definition(definition) => {
                write!(f, "{definition}")
            },
            TopLevelStatement::Print(expression, _id) => {
                write!(f, "print {expression}")
            },
        }
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl std::fmt::Display for Definition {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "def {}", self.name)?;
        if let Some(typ) = self.typ.as_ref() {
            write!(f, ": {typ}")?;
        }

        write!(f, " = {}", self.body)
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let should_parenthesize =
            |expr: &Expression| matches!(expr, Expression::FunctionCall { .. } | Expression::Lambda { .. });

        match self {
            Expression::IntegerLiteral(x, _id) => write!(f, "{x}"),
            Expression::Variable(identifier) => write!(f, "{identifier}"),
            Expression::FunctionCall { function, argument, id: _ } => {
                if matches!(function.as_ref(), Expression::Lambda { .. }) {
                    write!(f, "({function})")?;
                } else {
                    write!(f, "{function}")?;
                }

                if should_parenthesize(&argument) { write!(f, " ({argument})") } else { write!(f, " {argument}") }
            },
            Expression::Lambda { parameter_name, body, id: _ } => {
                write!(f, "fn {parameter_name} -> {body}")
            },
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Generic(identifier) => write!(f, "{identifier}"),
            Type::Function { parameter, return_type } => {
                if matches!(parameter.as_ref(), Type::Function { .. }) {
                    write!(f, "({parameter}) -> {return_type}")
                } else {
                    write!(f, "{parameter} -> {return_type}")
                }
            },
        }
    }
}
