use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::parser::ast::Identifier;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Type {
    /// This isn't a real type but can be useful to stop further type errors.
    /// It should generally be hidden from users - e.g. it unifies successfully with any other type.
    Error,
    Unit,
    Int,
    /// A user-supplied generic type. We don't want to bind over these like we do with type variables.
    Generic(Identifier),
    /// We represent type variables with unique ids and an external bindings map instead of a
    /// `Arc<RwLock<..>>` or similar because these need to be compared for equality, serialized, and
    /// be performant. We want the faster insertion of a local BTreeMap compared to a thread-safe
    /// version so we use a BTreeMap internally then freeze it in an Arc when finished to be
    /// able to access it from other threads.
    TypeVariable(TypeVariableId),
    Function {
        parameter: Arc<Type>,
        return_type: Arc<Type>,
    },
}

/// Maps type variables to their bindings
pub type TypeBindings = BTreeMap<TypeVariableId, Type>;

/// Maps generics to new types to instantiate them with
pub type Substitutions = BTreeMap<Arc<String>, Type>;

impl Type {
    pub fn from_ast_type(typ: &crate::parser::ast::Type) -> Type {
        match typ {
            crate::parser::ast::Type::Int => Type::Int,
            crate::parser::ast::Type::Generic(identifier) => Type::Generic(identifier.clone()),
            crate::parser::ast::Type::Function { parameter, return_type } => {
                let parameter = Arc::new(Self::from_ast_type(parameter));
                let return_type = Arc::new(Self::from_ast_type(return_type));
                Type::Function { parameter, return_type }
            },
        }
    }

    /// Substitutes any generics with the given names with the corresponding type in the map
    pub fn substitute(&self, substitutions: &Substitutions, bindings: &TypeBindings) -> Type {
        match self {
            Type::Error | Type::Unit | Type::Int => self.clone(),
            Type::TypeVariable(id) => {
                if let Some(binding) = bindings.get(&id) {
                    binding.substitute(substitutions, bindings)
                } else {
                    Type::TypeVariable(*id)
                }
            },
            Type::Generic(name) => {
                if let Some(substitution) = substitutions.get(&name.name) {
                    // Don't recur here, we've already substituted
                    substitution.clone()
                } else {
                    Type::Generic(name.clone())
                }
            }
            Type::Function { parameter, return_type } => {
                let parameter = Arc::new(parameter.substitute(substitutions, bindings));
                let return_type = Arc::new(return_type.substitute(substitutions, bindings));
                Type::Function { parameter, return_type }
            },
        }
    }

    pub fn display<'a, 'b>(&'a self, bindings: &'b TypeBindings) -> TypePrinter<'a, 'b> {
        TypePrinter { typ: self, bindings }
    }

    pub fn find_all_generics(&self) -> Vec<Arc<String>> {
        let mut found = BTreeSet::new();
        self.find_all_generics_helper(&mut found);
        found.into_iter().collect()
    }

    fn find_all_generics_helper(&self, found: &mut BTreeSet<Arc<String>>) {
        match self {
            Type::Error | Type::Unit | Type::Int | Type::TypeVariable(_) => (),
            Type::Generic(name) => { found.insert(name.name.clone()); },
            Type::Function { parameter, return_type } => {
                parameter.find_all_generics_helper(found);
                return_type.find_all_generics_helper(found);
            },
        }
    }
}

pub struct TypePrinter<'typ, 'bindings> {
    typ: &'typ Type,
    bindings: &'bindings TypeBindings,
}

impl std::fmt::Display for TypePrinter<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.fmt_type(&self.typ, f)
    }
}

impl TypePrinter<'_, '_> {
    fn fmt_type(&self, typ: &Type, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match typ {
            Type::Error => write!(f, "(error)"),
            Type::Unit => write!(f, "Unit"),
            Type::Int => write!(f, "Int"),
            Type::Generic(identifier) => write!(f, "{}", identifier.name),
            Type::TypeVariable(id) => {
                if let Some(binding) = self.bindings.get(&id) {
                    self.fmt_type(binding, f)
                } else {
                    write!(f, "{id}")
                }
            },
            Type::Function { parameter, return_type } => {
                if matches!(parameter.as_ref(), Type::Function { .. }) {
                    write!(f, "(")?;
                    self.fmt_type(parameter, f)?;
                    write!(f, ") -> ")?;
                } else {
                    self.fmt_type(parameter, f)?;
                    write!(f, " -> ")?;
                }
                self.fmt_type(return_type, f)
            },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TypeVariableId(pub u32);

impl TypeVariableId {
    pub(crate) fn occurs_in(self, other: &Type, bindings: &TypeBindings) -> bool {
        match other {
            Type::Error => false,
            Type::Unit => false,
            Type::Int => false,
            Type::Generic(_) => false,
            Type::TypeVariable(id) => {
                if let Some(binding) = bindings.get(&id) {
                    self.occurs_in(binding, bindings)
                } else {
                    self == *id
                }
            },
            Type::Function { parameter, return_type } => self.occurs_in(parameter, bindings) || self.occurs_in(return_type, bindings),
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
    pub generics: Vec<Arc<String>>,
    pub typ: Type,
}

impl TopLevelDefinitionType {
    pub fn new(generics: Vec<Arc<String>>, typ: Type) -> Self {
        Self { typ, generics }
    }

    pub fn unit() -> TopLevelDefinitionType {
        Self::new(Vec::new(), Type::Unit)
    }

    pub fn display<'a, 'b>(&'a self, bindings: &'b TypeBindings) -> TopLevelTypePrinter<'a, 'b> {
        TopLevelTypePrinter { typ: self, bindings }
    }

    pub fn from_ast_type(typ: &crate::parser::ast::Type) -> Self {
        let typ = Type::from_ast_type(typ);
        let generics = typ.find_all_generics();
        Self::new(generics, typ)
    }
}

pub struct TopLevelTypePrinter<'typ, 'bindings> {
    typ: &'typ TopLevelDefinitionType,
    bindings: &'bindings TypeBindings,
}

impl std::fmt::Display for TopLevelTypePrinter<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if !self.typ.generics.is_empty() {
            write!(f, "forall")?;
            for id in self.typ.generics.iter() {
                write!(f, " {}", id)?;
            }
            write!(f, ". ")?;
        }
        write!(f, "{}", self.typ.display(self.bindings))
    }
}
