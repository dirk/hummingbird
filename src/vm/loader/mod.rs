use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::super::ast;
use super::super::ast_to_ir;
use super::super::ir;
use super::super::parser;
use super::super::target::bytecode;
use super::errors::VmError;
use super::frame::Closure;

mod loaded_function;
mod loaded_module;

pub use loaded_function::{BytecodeFunction, LoadedFunction};
pub use loaded_module::LoadedModule;
use loaded_module::WeakLoadedModule;

lazy_static! {
    static ref DEBUG_ALL: bool = env::var("DEBUG_ALL").is_ok();
    static ref DEBUG_AST: bool = (*DEBUG_ALL || env::var("DEBUG_AST").is_ok());
    static ref DEBUG_IR: bool = (*DEBUG_ALL || env::var("DEBUG_IR").is_ok());
    static ref DEBUG_BYTECODE: bool = (*DEBUG_ALL || env::var("DEBUG_BYTECODE").is_ok());
}

pub struct Loader {
    load_path: Vec<PathBuf>,
    loaded_modules: HashMap<PathBuf, LoadedModule>,
    builtins_closure: Closure,
}

impl Loader {
    pub fn new(builtins_closure: Closure) -> Self {
        Self {
            load_path: vec![],
            loaded_modules: HashMap::new(),
            builtins_closure,
        }
    }

    /// Searches load-path for a file with the given name.
    ///
    /// If given a local path (normally for the module from which the import
    /// originated) then it will support relative imports; otherwise only
    /// absolute imports from the load-path are supported.
    pub fn load_file_by_name(
        &mut self,
        name: String,
        relative_import_path: Option<PathBuf>,
    ) -> Result<(LoadedModule, bool), VmError> {
        let result = self
            .search(name.clone(), relative_import_path)
            .and_then(|resolved| self.load_file(resolved));
        match result {
            Ok(details) => Ok(details),
            Err(error) => Err(VmError::new_load_file(name, error)),
        }
    }

    fn search(
        &self,
        name: String,
        relative_import_path: Option<PathBuf>,
    ) -> Result<PathBuf, Box<dyn Error>> {
        let is_relative = name.starts_with("./") || name.starts_with("../");
        let path = Path::new(&name);

        match (is_relative, relative_import_path) {
            (true, Some(relative_import_path)) => {
                // FIXME: Make providing the ".hb" extension in the path optional.
                return match relative_import_path.join(path).canonicalize() {
                    Ok(resolved) => Ok(resolved),
                    Err(error) =>{
                        Err(Box::new(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Error resolving relative path {:?}: {:?}", path, error),
                        )))
                    }
                }
            },
            (true, None) => {
                unreachable!("Cannot import from a relative path when there is no relative import path to search")
            }
            _ => (),
        };
        unreachable!("Cannot search non-relative yet ({:?})", name)
    }

    /// Load a file at a given path. This will not search load-paths!
    ///
    /// Returns a tuple of the loaded module and whether or not the module
    /// was already loaded.
    pub fn load_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(LoadedModule, bool), Box<dyn Error>> {
        let canonicalized = path
            .as_ref()
            .canonicalize()
            .expect("Could not canonicalize path");

        if let Some(existing) = self.loaded_modules.get(&canonicalized) {
            if existing.initialized() {
                return Ok((existing.clone(), true));
            } else {
                panic!("Circular dependency detected: {:?}", canonicalized)
            }
        }

        let new = load_file(canonicalized.clone(), Some(self.builtins_closure.clone()))?;
        self.loaded_modules.insert(canonicalized, new.clone());
        Ok((new, false))
    }

    /// Tries to remove the module. Should be called if the module fails to be
    /// initialized (eg. its `ModuleFrame` is unwound).
    pub fn unload(&mut self, module: &LoadedModule) -> bool {
        let path: PathBuf = module.name().into();
        self.loaded_modules.remove(&path).is_some()
    }
}

fn read_and_parse_file<P: AsRef<Path>>(path: P) -> Result<(ast::Module, String), Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    Ok((parser::parse(source.clone()), source))
}

pub fn compile_ast_into_module(
    ast_module: &ast::Module,
    name: String,
    source: String,
    ast_flags: ast_to_ir::CompilationFlags,
    // The highest closure in the system; normally should hold all the builtins.
    builtins_closure: Option<Closure>,
) -> Result<LoadedModule, Box<dyn Error>> {
    let ir_module = ast_to_ir::compile(ast_module, ast_flags);
    if *DEBUG_IR {
        println!("IR({}):", name);
        ir::printer::Printer::new(std::io::stdout()).print_module(&ir_module)?;
        println!();
    }

    let bytecode_module = ir::compiler::compile(&ir_module);
    if *DEBUG_BYTECODE {
        println!("Bytecode({}):", name);
        bytecode::printer::Printer::new(std::io::stdout()).print_module(&bytecode_module)?;
        println!();
    }

    let loaded_module =
        LoadedModule::from_bytecode(bytecode_module, name, source, builtins_closure);
    Ok(loaded_module)
}

pub fn load_file<P: AsRef<Path>>(
    path: P,
    builtins_closure: Option<Closure>,
) -> Result<LoadedModule, Box<dyn Error>> {
    let name = path
        .as_ref()
        .to_str()
        .expect("Couldn't convert path to string")
        .to_owned();

    let (ast_module, source) = read_and_parse_file(&name)?;
    if *DEBUG_AST {
        println!("AST({}):", name);
        ast::printer::Printer::new(std::io::stdout()).print_module(ast_module.clone())?;
        println!();
    }

    compile_ast_into_module(
        &ast_module,
        name,
        source,
        Default::default(),
        builtins_closure,
    )
}
