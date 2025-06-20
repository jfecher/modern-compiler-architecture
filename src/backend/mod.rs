use crate::{incremental::{self, parse, CompileFile, CompilerHandle, TypeCheck}, parser::ast::{Expression, TopLevelStatement}};

pub fn compile_file_impl(context: &CompileFile, compiler: &mut CompilerHandle) -> String {
    incremental::enter_query();
    incremental::println(format!("Compiling {}", context.file_name));

    let ast = parse(context.file_name.clone(), compiler).0.clone();
    let mut text = String::new();

    for statement in ast.statements.iter() {
        // Since we're compiling to python we don't actually need any type informtation
        // but we still want to type check and any real compiler would need the information
        // so we type check each top-level item anyway.
        compiler.get(TypeCheck(statement.id().clone()));

        match statement {
            TopLevelStatement::Import { file_name, id: _ } => {
                let translated_name = file_name.name.replace(".ex", "");
                text += &format!("from {translated_name} import *\n");
            },
            TopLevelStatement::Definition(definition) => {
                text += &format!("\n{} = {}\n", definition.name, expr_string(&definition.body));
            },
            TopLevelStatement::Print(expression, _) => {
                text += &format!("print({})\n", expr_string(expression));
            },
        }
    }

    incremental::exit_query();
    text
}

fn expr_string(expr: &Expression) -> String {
    match expr {
        Expression::IntegerLiteral(x, _) => x.to_string(),
        Expression::Variable(identifier) => {
            if identifier.name.as_ref() == "+" || identifier.name.as_ref() == "-" {
                format!("(lambda x: lambda y: x {} y)", identifier)
            } else {
                identifier.to_string()
            }
        }
        Expression::FunctionCall { function, argument, id: _ } => {
            if matches!(function.as_ref(), Expression::Lambda { .. }) {
                format!("({})({})", expr_string(function), expr_string(argument))
            } else {
                format!("{}({})", expr_string(function), expr_string(argument))
            }
        },
        Expression::Lambda { parameter_name, body, id: _ } => {
            format!("lambda {}: {}", parameter_name, expr_string(body))
        },
    }
}
