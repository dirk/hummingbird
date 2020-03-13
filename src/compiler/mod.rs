pub mod ir;
mod opaque;
mod path_to_name;
pub mod target;
mod vecs_equal;

use ir::IrError;

enum CompileError {
    /// Error occurring during the IR sub-stage.
    Ir(IrError),
}
