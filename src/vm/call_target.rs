use super::loader::LoadedFunction;

#[derive(Clone)]
pub struct CallTarget {
    pub function: LoadedFunction,
}
