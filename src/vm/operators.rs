use std::error;

use super::value::Value;

pub fn op_add(lhs: Value, rhs: Value) -> Result<Value, Box<dyn error::Error>> {
    match (&lhs, &rhs) {
        (Value::Integer(lhs), Value::Integer(rhs)) => Ok(Value::Integer(lhs + rhs)),
        _ => panic!("Cannot add {:?} and {:?}", lhs, rhs)
    }
}
