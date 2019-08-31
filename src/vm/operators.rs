use super::errors::VmError;
use super::gc::GcAllocator;
use super::value::Value;

#[inline]
pub fn op_add(lhs: Value, rhs: Value, gc: &mut GcAllocator) -> Result<Value, VmError> {
    match (&lhs, &rhs) {
        (Value::Integer(lhs), Value::Integer(rhs)) => Ok(Value::Integer(*lhs + *rhs)),
        (Value::String(lhs), Value::String(rhs)) => {
            let lhs = &**lhs;
            let rhs = &**rhs;
            let value = gc.allocate(lhs.clone() + rhs);
            Ok(Value::String(value))
        }
        _ => panic!("Cannot add {:?} and {:?}", lhs, rhs),
    }
}

#[inline]
pub fn op_equality(lhs: Value, rhs: Value) -> Result<Value, VmError> {
    let equal = match (&lhs, &rhs) {
        (Value::Integer(lhs), Value::Integer(rhs)) => (*lhs == *rhs),
        (Value::Function(lhs), Value::Function(rhs)) => lhs == rhs,
        (Value::String(lhs), Value::String(rhs)) => {
            let lhs = &**lhs;
            let rhs = &**rhs;
            lhs == rhs
        }
        _ => false,
    };
    Ok(Value::Boolean(equal))
}

#[inline]
pub fn op_less_than(lhs: Value, rhs: Value) -> Result<Value, VmError> {
    match (&lhs, &rhs) {
        (Value::Integer(lhs), Value::Integer(rhs)) => Ok(Value::Boolean(*lhs < *rhs)),
        _ => panic!("Cannot less-than {:?} and {:?}", lhs, rhs),
    }
}

#[inline]
pub fn op_property(target: Value, value: String) -> Result<Value, VmError> {
    match &target {
        Value::BuiltinObject(object) => match object.get_property(&value) {
            Some(found) => Ok(found),
            None => Err(VmError::new_property_not_found(target, value)),
        },
        Value::Module(module) => {
            if let Some(found) = module.get_export(&value) {
                Ok(found)
            } else {
                Err(VmError::new_property_not_found(target, value))
            }
        }
        _ => Err(VmError::new_argument_error(format!(
            "Cannot get a property of {:?}",
            target.type_name(),
        ))),
    }
}

/// Only `null` and `false` are falsy. Everything else is truthy.
#[inline]
pub fn is_truthy(value: Value) -> Result<bool, VmError> {
    match &value {
        Value::Null => Ok(false),
        Value::Boolean(value) => Ok(*value),
        _ => Ok(true),
    }
}
