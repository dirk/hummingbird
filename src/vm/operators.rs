use std::error;

use super::value::Value;

#[inline]
pub fn op_add(lhs: Value, rhs: Value) -> Result<Value, Box<dyn error::Error>> {
    match (&lhs, &rhs) {
        (Value::Integer(lhs), Value::Integer(rhs)) => {
            Ok(Value::Integer(*lhs + *rhs))
        },
        _ => panic!("Cannot add {:?} and {:?}", lhs, rhs)
    }
}

#[inline]
pub fn op_less_than(lhs: Value, rhs: Value) -> Result<Value, Box<dyn error::Error>> {
    match (&lhs, &rhs) {
        (Value::Integer(lhs), Value::Integer(rhs)) => Ok(Value::Boolean(*lhs < *rhs)),
        _ => panic!("Cannot less-than {:?} and {:?}", lhs, rhs)
    }
}

#[inline]
pub fn is_truthy(value: Value) -> Result<bool, Box<dyn error::Error>> {
    match &value {
        Value::Boolean(value) => Ok(*value),
        Value::Integer(value) => Ok(*value != 0),
        _ => Ok(true),
    }
}
