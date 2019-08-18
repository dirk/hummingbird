use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone)]
struct BuiltinFunction(Rc<dyn Fn(Vec<Value>) -> Value>);

impl BuiltinFunction {
    pub fn new(call_target: Rc<dyn Fn(Vec<Value>) -> Value>) -> Self {
        Self(call_target)
    }

    pub fn call(&self, arguments: Vec<Value>) -> Value {
        let call_target = &self.0;
        call_target(arguments)
    }
}

fn builtin_println(arguments: Vec<Value>) -> Value {
    Value::Null
}

trait Scope {
    fn get(&self, name: String) -> Option<Value>;
}

struct BuiltinScope(Rc<HashMap<String, Value>>);

impl BuiltinScope {
    fn bootstrap() -> Self {
        let mut locals = HashMap::<String, Value>::new();
        locals.insert(
            "println".to_string(),
            BuiltinFunction::new(Rc::new(builtin_println)).into(),
        );
        Self(Rc::new(locals))
    }
}

impl Scope for BuiltinScope {
    fn get(&self, name: String) -> Option<Value> {
        self.0.get(&name).map(Value::to_owned)
    }
}

#[derive(Clone)]
enum Value {
    Null,
    BuiltinFunction(BuiltinFunction),
}

impl From<BuiltinFunction> for Value {
    fn from(builtin_function: BuiltinFunction) -> Self {
        Self::BuiltinFunction(builtin_function)
    }
}

struct StaticScope {
    locals: HashMap<String, Value>,
    parent: BuiltinScope,
}

impl Scope for StaticScope {
    fn get(&self, name: String) -> Option<Value> {
        if let Some(value) = self.locals.get(&name) {
            Some(value.to_owned())
        } else {
            self.parent.get(name)
        }
    }
}

pub struct Vm {
    builtin_scope: BuiltinScope,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            builtin_scope: BuiltinScope::bootstrap(),
        }
    }
}
