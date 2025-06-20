use std::{collections::BTreeMap, rc::Rc};

use serde::{Deserialize, Serialize};

use crate::{
    errors::{Error, Errors},
    incremental::{self, CompilerHandle, Resolve},
    parser::{
        ast::{Expression, TopLevelStatement},
        ids::{ExprId, TopLevelId},
    },
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolutionResult {
    /// This resolution is for a single top level id so all expressions within are in the
    /// context of that id.
    pub origins: BTreeMap<ExprId, Origin>,
    pub errors: Errors,
}

struct Resolver<'local, 'inner> {
    item: TopLevelId,
    links: BTreeMap<ExprId, Origin>,
    errors: Errors,
    names_in_global_scope: BTreeMap<Rc<String>, TopLevelId>,
    parameters_in_scope: BTreeMap<Rc<String>, ExprId>,
    compiler: &'local mut CompilerHandle<'inner>,
}

/// Where was this variable defined?
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    /// This name comes from this top level definition
    TopLevelDefinition(TopLevelId),
    /// This name is the parameter of this lambda expression.
    /// Remember that all lambdas define only a single parameter.
    Parameter(ExprId),
}

pub fn resolve_impl(context: &Resolve, compiler: &mut CompilerHandle) -> ResolutionResult {
    incremental::enter_query();
    let statement = incremental::get_statement(context.0.clone(), compiler).clone();
    incremental::println(format!("Resolving {statement}"));

    let names_in_scope = incremental::get_globally_visible_definitions(context.0.file_path.clone(), compiler).0.clone();

    let mut resolver = Resolver::new(compiler, context.0.clone(), names_in_scope);

    match statement {
        TopLevelStatement::Import { .. } => (),
        TopLevelStatement::Definition(definition) => resolver.resolve_expr(&definition.body),
        TopLevelStatement::Print(expression, _) => resolver.resolve_expr(&expression),
    }

    incremental::exit_query();
    resolver.result()
}

impl<'local, 'inner> Resolver<'local, 'inner> {
    fn new(
        compiler: &'local mut CompilerHandle<'inner>, item: TopLevelId,
        names_in_scope: BTreeMap<Rc<String>, TopLevelId>,
    ) -> Self {
        Self {
            compiler,
            item,
            names_in_global_scope: names_in_scope,
            links: Default::default(),
            errors: Vec::new(),
            parameters_in_scope: Default::default(),
        }
    }

    fn result(self) -> ResolutionResult {
        ResolutionResult { origins: self.links, errors: self.errors }
    }

    fn lookup(&self, name: &Rc<String>) -> Option<Origin> {
        // Check local parameters first. They shadow global definitions
        if let Some(expr) = self.parameters_in_scope.get(name) {
            return Some(Origin::Parameter(*expr));
        }
        if let Some(statement) = self.names_in_global_scope.get(name) {
            return Some(Origin::TopLevelDefinition(statement.clone()));
        }
        None
    }

    fn link(&mut self, name: &Rc<String>, expr: ExprId) {
        if name.as_ref() == "+" || name.as_ref() == "-" {
            // Ignore built-ins
        } else if let Some(origin) = self.lookup(name) {
            self.links.insert(expr, origin);
        } else {
            let location = expr.location(&self.item, self.compiler);
            self.errors.push(Error::NameNotInScope { name: name.clone(), location });
        }
    }

    fn resolve_expr(&mut self, expression: &Expression) {
        match expression {
            Expression::IntegerLiteral(..) => (),
            Expression::Variable(identifier) => self.link(&identifier.name, identifier.id),
            Expression::FunctionCall { function, argument, id: _ } => {
                self.resolve_expr(&function);
                self.resolve_expr(&argument);
            },
            Expression::Lambda { parameter_name, body, id: _ } => {
                // Resolve body with the parameter name in scope
                let old_name = self.parameters_in_scope.insert(parameter_name.name.clone(), parameter_name.id);
                self.resolve_expr(&body);

                // Then remember to either remove the parameter name from scope, or if we shadowed
                // an existing name, then re-insert that one.
                if let Some(old_name) = old_name {
                    self.parameters_in_scope.insert(parameter_name.name.clone(), old_name);
                } else {
                    self.parameters_in_scope.remove(&parameter_name.name);
                }
            },
        }
    }
}
