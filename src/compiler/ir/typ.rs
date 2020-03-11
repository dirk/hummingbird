use super::super::super::type_ast::{self as ast};
use super::super::vecs_equal::vecs_equal;
use super::Func;

#[derive(Clone)]
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
