use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serde::{Deserialize, Serialize};

use crate::parser::ast::Identifier;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Type {
    /// This isn't a real type but can be useful to stop further type errors.
    /// It should generally be hidden from users - e.g. it unifies successfully with any other type.
    Error,
    Unit,
    Int,
    Generic(Identifier),
    TypeVariable(TypeVariable),
    Function {
        parameter: Rc<Type>,
        return_type: Rc<Type>,
    },
}

impl Type {
    pub fn generalize(&self) -> TopLevelDefinitionType {
        TopLevelDefinitionType { type_variables: Vec::new(), typ: self.clone() }
        //todo!()
    }

    pub fn from_ast_type(typ: &crate::parser::ast::Type) -> Type {
        match typ {
            crate::parser::ast::Type::Int => Type::Int,
            crate::parser::ast::Type::Generic(identifier) => Type::Generic(identifier.clone()),
            crate::parser::ast::Type::Function { parameter, return_type } => {
                let parameter = Rc::new(Self::from_ast_type(parameter));
                let return_type = Rc::new(Self::from_ast_type(return_type));
                Type::Function { parameter, return_type }
            },
        }
    }

    /// Substitutes any unbound type variables with the given id with the corresponding type in the map
    pub fn substitute(&self, substitutions: &HashMap<TypeVariableId, Type>) -> Type {
        match self {
            Type::Error | Type::Unit | Type::Int | Type::Generic(_) => self.clone(),
            Type::TypeVariable(type_variable) => {
                let binding = type_variable.binding.borrow();
                match &*binding {
                    Some(binding) => binding.substitute(substitutions),
                    None => {
                        if let Some(substitution) = substitutions.get(&type_variable.id) {
                            substitution.clone()
                        } else {
                            self.clone()
                        }
                    },
                }
            },
            Type::Function { parameter, return_type } => {
                let parameter = Rc::new(parameter.substitute(substitutions));
                let return_type = Rc::new(return_type.substitute(substitutions));
                Type::Function { parameter, return_type }
            },
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Type::Error => write!(f, "(error)"),
            Type::Unit => write!(f, "Unit"),
            Type::Int => write!(f, "Int"),
            Type::Generic(identifier) => write!(f, "{}", identifier.name),
            Type::TypeVariable(type_variable) => {
                let binding = type_variable.binding.borrow();
                match &*binding {
                    Some(binding) => binding.fmt(f),
                    None => write!(f, "{}", type_variable.id),
                }
            },
            Type::Function { parameter, return_type } => {
                if matches!(parameter.as_ref(), Type::Function { .. }) {
                    write!(f, "({}) -> {}", parameter, return_type)
                } else {
                    write!(f, "{} -> {}", parameter, return_type)
                }
            },
        }
    }
}

/// A type variable is either unbound, or bound to a specific type.
/// These are essentially the holes that we fill in during type inference.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeVariable {
    /// Each type variable id is local to a TopLevelStatement, similar to ExprIds.
    pub id: TypeVariableId,
    pub binding: Rc<RefCell<Option<Type>>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TypeVariableId(pub u32);

impl TypeVariableId {
    pub(crate) fn occurs_in(&self, other: &Type) -> bool {
        match other {
            Type::Error => false,
            Type::Unit => false,
            Type::Int => false,
            Type::Generic(_) => false,
            Type::TypeVariable(type_variable) => {
                let binding = type_variable.binding.borrow();
                match &*binding {
                    Some(binding) => self.occurs_in(binding),
                    None => *self == type_variable.id,
                }
            },
            Type::Function { parameter, return_type } => self.occurs_in(parameter) || self.occurs_in(return_type),
        }
    }
}

impl std::fmt::Display for TypeVariableId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "_{}", self.0)
    }
}

/// A top level definition's type may be generalized (made generic).
/// Other definitions like parameters are never generic.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopLevelDefinitionType {
    pub type_variables: Vec<TypeVariableId>,
    pub typ: Type,
}

impl TopLevelDefinitionType {
    pub fn new(type_variables: Vec<TypeVariableId>, typ: Type) -> Self {
        Self { typ, type_variables }
    }

    pub fn unit() -> TopLevelDefinitionType {
        Self::new(Vec::new(), Type::Unit)
    }

    pub fn from_ast_type(ast_type: &crate::parser::ast::Type) -> Self {
        Type::from_ast_type(ast_type).generalize()
    }
}

impl std::fmt::Display for TopLevelDefinitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if !self.type_variables.is_empty() {
            write!(f, "forall")?;
            for id in self.type_variables.iter() {
                write!(f, " {}", id)?;
            }
            write!(f, ". ")?;
        }
        write!(f, "{}", self.typ)
    }
}
