use super::super::super::type_ast::{self as ast};
use super::super::vecs_equal::vecs_equal;
use super::Func;

#[derive(Clone, Debug)]
pub enum Type {
    /// A type which can be represented by a value.
    Real(RealType),
    Abstract(AbstractType),
}

impl Type {
    pub fn into_real(self) -> RealType {
        use Type::*;
        match self {
            Real(real_type) => real_type,
            _ => panic!("Cannot convert to Real type"),
        }
    }

    pub fn is_equal(&self, other: &Type) -> bool {
        use Type::*;
        match (self, other) {
            (Real(self_real), Real(other_real)) => self_real.is_equal(other_real),
            (Abstract(self_abstract), Abstract(other_abstract)) => {
                self_abstract.is_equal(other_abstract)
            }
            _ => false,
        }
    }
}

// Types aren't actually safe to share between threads, but we have to trick
// the compiler into allowing it so that we can store them in the
// `BUILTINS_CACHE` lazy static.
unsafe impl Send for Type {}
unsafe impl Sync for Type {}

#[derive(Clone, Debug)]
pub enum RealType {
    FuncPtr(FuncPtrType),
    Int64,
    Tuple(TupleType),
}

impl RealType {
    pub fn is_equal(&self, other: &RealType) -> bool {
        use RealType::*;
        match (self, other) {
            (FuncPtr(self_func_ptr_type), FuncPtr(other_func_ptr_type)) => {
                let parameters_match = vecs_equal(
                    &self_func_ptr_type.parameters,
                    &other_func_ptr_type.parameters,
                    RealType::is_equal,
                );
                if !parameters_match {
                    return false;
                }
                self_func_ptr_type
                    .retrn
                    .is_equal(&other_func_ptr_type.retrn)
            }
            (Int64, Int64) => true,
            (Tuple(self_tuple_type), Tuple(other_tuple_type)) => vecs_equal(
                &self_tuple_type.members,
                &other_tuple_type.members,
                RealType::is_equal,
            ),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FuncPtrType {
    pub parameters: Vec<RealType>,
    pub retrn: Box<RealType>,
}

impl FuncPtrType {
    pub fn new(parameters: Vec<RealType>, retrn: RealType) -> Self {
        Self {
            parameters,
            retrn: Box::new(retrn),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TupleType {
    pub members: Vec<RealType>,
}

impl TupleType {
    pub fn unit() -> Self {
        Self::new(vec![])
    }

    pub fn new(members: Vec<RealType>) -> Self {
        Self { members }
    }
}

#[derive(Clone)]
pub enum AbstractType {
    UnspecializedFunc(Func),
}

impl AbstractType {
    pub fn is_equal(&self, other: &AbstractType) -> bool {
        use AbstractType::*;
        match (self, other) {
            (
                UnspecializedFunc(self_unspecialized_func),
                UnspecializedFunc(other_unspecialized_func),
            ) => self_unspecialized_func.is_equal(other_unspecialized_func),
        }
    }
}

impl std::fmt::Debug for AbstractType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use AbstractType::*;
        match self {
            UnspecializedFunc(func) => write!(f, "UnspecializedFunc({})", func.name()),
        }
    }
}
