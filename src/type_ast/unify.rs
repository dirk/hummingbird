use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use super::scope::Scope;
use super::typ::{Func, Generic, GenericConstraint, Object, Type, Variable};
use super::{TypeError, TypeResult};

/// Unify a variable (mutable) generic with another generic.
pub fn unify_variable_generic_with_generic(
    destination: &Rc<RefCell<Variable>>,
    source: &Generic,
    scope: Scope,
) -> TypeResult<()> {
    enum Action {
        AddCallableConstraint(Vec<Type>, Type),
        AddPropertyConstraint(String, Type),
        None,
    }

    for constraint in source.constraints.iter() {
        use Action::*;
        use GenericConstraint::*;

        let action = {
            // Get an immutable borrow while we determine what to do.
            let destination = Ref::map(destination.borrow(), Variable::unwrap_generic);

            match constraint {
                Property(source_property) => {
                    // If the property already exists then unify their types,
                    // otherwise add it to the left side.
                    if let Some(destination_property) =
                        destination.get_property(&source_property.name)
                    {
                        unify(
                            &destination_property.typ,
                            &source_property.typ,
                            scope.clone(),
                        )?;
                        None
                    } else {
                        AddPropertyConstraint(
                            source_property.name.clone(),
                            source_property.typ.clone(),
                        )
                    }
                }
                Callable(source_callable) => {
                    if let Some(destination_callable) = destination.get_callable() {
                        if destination_callable.arguments.len() != source_callable.arguments.len() {
                            return Err(TypeError::ArgumentLengthMismatch {
                                expected: destination_callable.arguments.clone(),
                                got: source_callable.arguments.clone(),
                            });
                        }
                        for (destination_argument, source_argument) in destination_callable
                            .arguments
                            .iter()
                            .zip(source_callable.arguments.iter())
                        {
                            unify(destination_argument, source_argument, scope.clone())?;
                        }
                        unify(
                            &destination_callable.retrn,
                            &source_callable.retrn,
                            scope.clone(),
                        )?;
                        None
                    } else {
                        AddCallableConstraint(
                            source_callable.arguments.clone(),
                            source_callable.retrn.clone(),
                        )
                    }
                }
            }
        };

        let destination = &mut *RefMut::map(destination.borrow_mut(), Variable::unwrap_mut_generic);
        match action {
            AddCallableConstraint(arguments, retrn) => {
                destination.add_callable_constraint(arguments, retrn)
            }
            AddPropertyConstraint(name, typ) => destination.add_property_constraint(name, typ),
            None => (),
        }
    }
    Ok(())
}

/// Check if an object satisfies a generic's constraints.
fn object_satisfies_constraints(generic: &Generic, object: &Rc<Object>) -> TypeResult<()> {
    if generic.constraints.is_empty() {
        return Ok(());
    }
    // TODO: Actually check each constraint against the object.
    Err(TypeError::InternalError {
        message: format!(
            "Object doesn't satisfy constraints:\nobject: {:?}\nconstraints: {:?}",
            object, generic.constraints,
        ),
    })
}

pub fn unify(typ1: &Type, typ2: &Type, scope: Scope) -> TypeResult<()> {
    if typ1 == typ2 {
        return Ok(());
    }

    if let Type::Variable(var2) = typ2 {
        if let Variable::Substitute { substitute, .. } = &*var2.borrow() {
            return unify(typ1, substitute, scope);
        }
    }

    if let Type::Variable(var1) = typ1 {
        enum Action {
            // Make `typ1` a substitute for another type.
            Substitute(Type),
            // Used to unify a different type with `typ2` (eg. if `typ1` is a
            // substitute).
            Reunify(Type),
            // Unify both variable generics.
            UnifyGenerics,
            UnifyGenericWithObject(Rc<Object>),
            UnifyGenericWithFunc(Rc<Func>),
        }
        use Action::*;

        let action = match &*var1.borrow() {
            Variable::Generic { .. } => {
                // WARNING: These branches rely on implicit returns and
                //   therefore *must* stay exhaustive.
                match typ2 {
                    Type::Object(object2) => UnifyGenericWithObject(object2.clone()),
                    Type::Variable(var2) => {
                        // If this is a generic and the other type is unbound
                        // then update the other type to be a substitute for
                        // this type.
                        if var2.borrow().is_unbound() {
                            *var2.borrow_mut() = Variable::Substitute {
                                scope,
                                substitute: Box::new(typ1.clone()),
                            };
                            return Ok(());
                        // If it's also a variable generic then we can attempt to
                        // union the two sets of generic constraints.
                        } else if var2.borrow().is_generic() {
                            UnifyGenerics
                        } else {
                            return Err(TypeError::TypeMismatch {
                                expected: typ1.clone(),
                                got: typ2.clone(),
                            });
                        }
                    }
                    Type::Func(func2) => UnifyGenericWithFunc(func2.clone()),
                    _ => {
                        return Err(TypeError::TypeMismatch {
                            expected: typ1.clone(),
                            got: typ2.clone(),
                        })
                    }
                }
            }
            // If we're a substitute then unify whatever we're substituted with
            // with `typ2`.
            Variable::Substitute { substitute, .. } => Reunify(*substitute.clone()),
            // If we're unbound then inherit whatever the other type is.
            Variable::Unbound { .. } => Substitute(typ2.clone()),
        };

        return match action {
            Substitute(typ) => {
                *var1.borrow_mut() = Variable::Substitute {
                    scope,
                    substitute: Box::new(typ),
                };
                Ok(())
            }
            Reunify(typ) => unify(&typ, typ2, scope),
            UnifyGenerics => {
                let variable2 = typ2.unwrap_variable();
                // If both types are variable generics then first merge the
                // right into the left.
                {
                    let variable1 = typ1.unwrap_variable();
                    let generic2 = Ref::map(variable2.borrow(), Variable::unwrap_generic);
                    unify_variable_generic_with_generic(variable1, &generic2, scope.clone())?;
                }
                // Then make the right a substitute for the left.
                *variable2.borrow_mut() = Variable::Substitute {
                    scope,
                    substitute: Box::new(typ1.clone()),
                };
                Ok(())
            }
            UnifyGenericWithObject(object) => {
                {
                    // First ensure the object satisfies our constraints.
                    let generic = Ref::map(var1.borrow(), Variable::unwrap_generic);
                    object_satisfies_constraints(&generic, &object)?;
                }
                // If the constraints are all satisfied then we can substitute
                // ourselves for the object.
                *var1.borrow_mut() = Variable::Substitute {
                    scope,
                    substitute: Box::new(Type::Object(object)),
                };
                Ok(())
            }
            UnifyGenericWithFunc(func) => {
                {
                    let generic = Ref::map(var1.borrow(), Variable::unwrap_generic);
                    // TODO: Implement a func_satisfies_constraints function.
                    if !generic.constraints.is_empty() {
                        panic!("Cannot yet unify non-empty generic with func")
                    }
                }
                // If all the constraints are satisfied then we can substitute
                // ourselves for the func.
                *var1.borrow_mut() = Variable::Substitute {
                    scope,
                    substitute: Box::new(Type::Func(func)),
                };
                Ok(())
            }
        };
    }

    // If the other type is a variable then unify with it as the first term so
    // that we can reuse the logic above.
    if let Type::Variable(_) = &typ2 {
        return unify(typ2, typ1, scope).map_err(|err| err.reverse());
    }

    match (typ1, typ2) {
        (Type::Func(func1), Type::Func(func2)) => {
            if func1.arity() != func2.arity() {
                return Err(TypeError::TypeMismatch {
                    expected: typ1.clone(),
                    got: typ2.clone(),
                });
            }
            let func1_arguments = func1.arguments.borrow();
            let func2_arguments = func2.arguments.borrow();
            for (func1_argument, func2_argument) in
                func1_arguments.iter().zip(func2_arguments.iter())
            {
                unify(func1_argument, func2_argument, scope.clone())?;
            }
            let func1_return = &*func1.retrn.borrow();
            let func2_return = &*func2.retrn.borrow();
            return unify(&func1_return, &func2_return, scope);
        }
        _ => (),
    }

    Err(TypeError::CannotUnify {
        first: typ1.clone(),
        second: typ2.clone(),
    })
}
