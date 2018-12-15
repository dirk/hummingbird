use super::super::ir::layout;
use super::nodes::*;

struct Compiler {
    unit: layout::Unit,
    current: layout::SharedFunction,
}

impl Compiler {
    fn new() -> Self {
        let unit = layout::Unit::new();
        let current = unit.main_function();
        Self { unit, current }
    }

    fn compile_program(&mut self, program: &Program) {
        // We should start in the main function.
        assert_eq!(self.current, self.unit.main_function());

        // And we should end in the main function.
        assert_eq!(self.current, self.unit.main_function());
    }
}

pub fn compile(program: &Program) -> layout::Unit {
    let mut compiler = Compiler::new();
    compiler.compile_program(program);
    compiler.unit
}
