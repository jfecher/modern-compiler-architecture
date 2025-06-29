use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};
use types::TypeBindings;

use crate::{
    errors::{Error, Errors},
    incremental::{self, CompilerHandle, GetStatement, GetType, Resolve, TypeCheck},
    name_resolution::{Origin, ResolutionResult},
    parser::{
        ast::{Expression, TopLevelStatement},
        ids::{ExprId, TopLevelId},
    },
    type_inference::types::{TopLevelDefinitionType, Type, TypeVariableId},
};

pub mod types;

/// Get the type of the name defined by this TopLevelItemId.
/// If this doesn't define a name we return the Unit type by default.
///
/// This is very similar to but separate from `type_check_impl`. We separate these
/// because `type_check_impl` will always type check the contents of a definition,
/// and we don't want other definitions to depend on the contents of another definition
/// if the other definition provides a type annotation. Without type annotations the two
/// functions should be mostly equivalent.
pub fn get_type_impl(context: &GetType, compiler: &CompilerHandle) -> TopLevelDefinitionType {
    incremental::enter_query();
    let statement = compiler.get(GetStatement(context.0.clone()));
    incremental::println(format!("Get type of {statement}"));

    let typ = match statement {
        TopLevelStatement::Import { .. } => TopLevelDefinitionType::unit(),
        TopLevelStatement::Print(..) => TopLevelDefinitionType::unit(),
        TopLevelStatement::Definition(definition) => {
            if let Some(typ) = &definition.typ {
                TopLevelDefinitionType::from_ast_type(typ)
            } else {
                let result = compiler.get(TypeCheck(context.0.clone()));
                result.typ.clone()
            }
        },
    };
    incremental::exit_query();
    typ
}

/// Actually type check a statement and its contents.
/// Unlike `get_type_impl`, this always type checks the expressions inside a statement
/// to ensure they type check correctly.
pub fn type_check_impl(context: &TypeCheck, compiler: &CompilerHandle) -> TypeCheckResult {
    incremental::enter_query();
    let statement = GetStatement(context.0.clone()).get(compiler);
    incremental::println(format!("Type checking {statement}"));

    let resolve = Resolve(context.0.clone()).get(compiler);
    let mut checker = TypeChecker::new(context.0.clone(), resolve, compiler);

    let typ = match statement {
        TopLevelStatement::Import { .. } => TopLevelDefinitionType::unit(),
        TopLevelStatement::Print(expression, _) => {
            checker.check_expr(&expression);
            TopLevelDefinitionType::unit()
        },
        TopLevelStatement::Definition(definition) => {
            let actual_type = checker.check_expr(&definition.body);

            if let Some(typ) = &definition.typ {
                let expected = Type::from_ast_type(typ);
                checker.unify(&actual_type, &expected, definition.body.id());
                checker.generalize(&expected)
            } else {
                checker.generalize(&actual_type)
            }
        },
    };

    incremental::exit_query();
    checker.finish(typ)
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeCheckResult {
    pub typ: TopLevelDefinitionType,
    pub expr_types: BTreeMap<ExprId, Type>,
    pub errors: Errors,
}

struct TypeChecker<'local, 'inner> {
    compiler: &'local CompilerHandle<'inner>,
    origins: BTreeMap<ExprId, Origin>,
    expr_types: BTreeMap<ExprId, Type>,
    bindings: TypeBindings,
    item: TopLevelId,
    next_id: u32,
    errors: Errors,
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    fn new(item: TopLevelId, resolve: ResolutionResult, compiler: &'local CompilerHandle<'inner>) -> Self {
        Self {
            compiler,
            item,
            bindings: Default::default(),
            origins: resolve.origins,
            next_id: 0,
            expr_types: Default::default(),
            errors: resolve.errors,
        }
    }

    fn finish(self, typ: TopLevelDefinitionType) -> TypeCheckResult {
        TypeCheckResult { typ, expr_types: self.expr_types, errors: self.errors }
    }

    fn next_type_variable(&mut self) -> Type {
        let id = TypeVariableId(self.next_id);
        self.next_id += 1;
        Type::TypeVariable(id)
    }

    fn store_and_return_type(&mut self, expr: ExprId, typ: Type) -> Type {
        self.expr_types.insert(expr, typ.clone());
        typ
    }

    fn lookup_type(&mut self, expr: ExprId) -> Type {
        match self.origins.get(&expr) {
            // A name resolution error occurred, don't issue a type error as well
            None => Type::Error,
            Some(Origin::Parameter(id)) => self.expr_types.get(id).unwrap().clone(),
            Some(Origin::TopLevelDefinition(id)) => {
                // We may recursively type check here. This can lead to infinite
                // recursion for mutually recursive functions with inferred types.
                // To simplify type inference for this toy compiler, we don't handle this case.
                let typ = GetType(id.clone()).get(self.compiler);
                self.instantiate(&typ)
            },
        }
    }

    /// Returns the type of `+` or `-`. Returns `None` if `name` is not `+` or `-`.
    fn try_get_type_for_builtin(&self, name: &str) -> Option<Type> {
        if name == "+" || name == "-" {
            Some(Type::Function {
                parameter: Arc::new(Type::Int),
                return_type: Arc::new(Type::Function { parameter: Arc::new(Type::Int), return_type: Arc::new(Type::Int) }),
            })
        } else {
            None
        }
    }

    /// Generalize a type, making it generic. Any holes in the type become generic types.
    fn generalize(&mut self, typ: &Type) -> TopLevelDefinitionType {
        // The `generics` list in `TopLevelDefinitionType` can only map `Type::Generic` so
        // we have to manually replace type variables here with generics with somewhat arbitrary
        // names.
        let typ = self.replace_type_variables_with_named_generics(typ);

        // TODO: We should exclude any type variables which "escape" when generalizing.
        // See http://okmij.org/ftp/ML/generalization.html.
        let generics = typ.find_all_generics();
        TopLevelDefinitionType::new(generics, typ)
    }

    /// Replaces any `Type::TypeVariable`s with `Type::Generic`s.
    /// Their names are simply their integer ids converted to a string to ensure they do not
    /// collide with the names of any existing generics in the same type.
    fn replace_type_variables_with_named_generics(&mut self, typ: &Type) -> Type {
        match typ {
            Type::Error => Type::Error,
            Type::Unit => Type::Unit,
            Type::Int => Type::Int,
            Type::Generic(name) => Type::Generic(name.clone()),
            Type::TypeVariable(id) => {
                let name = Arc::new(id.0.to_string());
                // FIXME: We have to provide an ExprId when creating the generic identifier but we
                // lack the context to create a fresh one. It's not too important for this toy
                // compiler but a real one would probably want to rework type generalization in
                // general.
                let id = ExprId::new(u32::MAX);
                Type::Generic(crate::parser::ast::Identifier { name, id })
            },
            Type::Function { parameter, return_type } => {
                let parameter = Arc::new(self.replace_type_variables_with_named_generics(parameter));
                let return_type = Arc::new(self.replace_type_variables_with_named_generics(return_type));
                Type::Function { parameter, return_type }
            },
        }
    }

    fn check_expr(&mut self, expr: &Expression) -> Type {
        match expr {
            Expression::IntegerLiteral(_, id) => self.store_and_return_type(*id, Type::Int),
            Expression::Variable(identifier) => {
                // If this is a built-in, get that type. Otherwise, lookup or query its type.
                let typ =
                    self.try_get_type_for_builtin(&identifier.name).unwrap_or_else(|| self.lookup_type(identifier.id));
                self.store_and_return_type(identifier.id, typ)
            },
            Expression::FunctionCall { function, argument, id } => {
                let return_type = self.next_type_variable();
                let expected = Type::Function {
                    parameter: Arc::new(self.check_expr(argument)),
                    return_type: Arc::new(return_type.clone()),
                };
                let actual = self.check_expr(function);
                self.unify(&actual, &expected, *id);
                self.store_and_return_type(*id, return_type)
            },
            Expression::Lambda { parameter_name, body, id } => {
                let parameter_type = self.next_type_variable();
                self.expr_types.insert(parameter_name.id, parameter_type.clone());
                let body_type = self.check_expr(body);
                let function_type =
                    Type::Function { parameter: Arc::new(parameter_type), return_type: Arc::new(body_type) };
                self.store_and_return_type(*id, function_type)
            },
        }
    }

    fn instantiate(&mut self, typ: &TopLevelDefinitionType) -> Type {
        let substitutions = typ.generics.iter().map(|name| (name.clone(), self.next_type_variable())).collect();
        typ.typ.substitute(&substitutions, &self.bindings)
    }

    fn unify(&mut self, actual: &Type, expected: &Type, id: ExprId) {
        match (actual, expected) {
            (Type::Error, _) | (_, Type::Error) => (),
            (Type::Unit, Type::Unit) => (),
            (Type::Int, Type::Int) => (),
            (Type::Generic(name1), Type::Generic(name2)) if name1.name == name2.name => (),
            (
                Type::Function { parameter: actual_parameter, return_type: actual_return_type },
                Type::Function { parameter: expected_parameter, return_type: expected_return_type },
            ) => {
                self.unify(&actual_parameter, &expected_parameter, id);
                self.unify(&actual_return_type, &expected_return_type, id);
            },
            // If the type variable is already bound to something, recur on that binding
            (Type::TypeVariable(type_var), expected) if self.bindings.contains_key(&type_var) => {
                let actual = self.bindings.get(type_var).unwrap().clone();
                self.unify(&actual, expected, id);
            },
            // If the type variable is already bound to something, recur on that binding
            (actual, Type::TypeVariable(type_var)) if self.bindings.contains_key(&type_var) => {
                let expected = self.bindings.get(type_var).unwrap().clone();
                self.unify(actual, &expected, id);
            },
            (type_var_type @ Type::TypeVariable(type_var), other)
            | (other, type_var_type @ Type::TypeVariable(type_var)) => {
                if type_var_type == other {
                    // Don't bind a type variable to itself
                } else if !type_var.occurs_in(other, &self.bindings) {
                    self.bindings.insert(*type_var, other.clone());
                } else {
                    // Error, binding here would make a recursive type!
                    let location = id.location(&self.item, self.compiler);
                    let actual = actual.display(&self.bindings).to_string();
                    let expected = expected.display(&self.bindings).to_string();
                    self.errors.push(Error::ExpectedType { actual, expected, location })
                }
            },
            (actual, expected) => {
                let location = id.location(&self.item, self.compiler);
                let actual = actual.display(&self.bindings).to_string();
                let expected = expected.display(&self.bindings).to_string();
                self.errors.push(Error::ExpectedType { actual, expected, location })
            },
        }
    }
}
