use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use super::typ::{Generic, GenericConstraint, Type, Variable};
use super::{TypeError, TypeResult};

// Extract the variable type within a type.
fn unwrap_variable(typ: &Type) -> &Rc<RefCell<Variable>> {
    match typ {
        Type::Variable(variable) => variable,
        other @ _ => unreachable!("Not a variable type: {:?}", other),
    }
}

/// Unify a variable (mutable) generic with another generic.
pub fn unify_variable_generic_with_generic(destination: &Type, source: &Generic) -> TypeResult<()> {
    let destination = unwrap_variable(destination);

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
                        unify(&destination_property.typ, &source_property.typ)?;
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
                            unify(destination_argument, source_argument)?;
                        }
                        unify(&destination_callable.retrn, &source_callable.retrn)?;
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

pub fn unify(typ1: &Type, typ2: &Type) -> TypeResult<()> {
    if typ1 == typ2 {
        return Ok(());
    }

    if let Type::Variable(var2) = typ2 {
        if let Variable::Substitute(substitute) = &*var2.borrow() {
            return unify(typ1, substitute);
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
        }
        use Action::*;

        let action = match &*var1.borrow() {
            generic @ Variable::Generic(_) => {
                // WARNING: These if-elses rely on implicit returns and
                //   therefore *must* stay exhaustive.
                if let Type::Variable(var2) = typ2 {
                    // If this is a generic and the other type is unbound then
                    // update the other type to be a substitute for this type.
                    if var2.borrow().is_unbound() {
                        *var2.borrow_mut() = Variable::Substitute(Box::new(typ1.clone()));
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
                } else {
                    return Err(TypeError::TypeMismatch {
                        expected: typ1.clone(),
                        got: typ2.clone(),
                    });
                }
            }
            // If we're a substitute then unify whatever we're substituted with
            // with `typ2`.
            Variable::Substitute(substitute) => Reunify(*substitute.clone()),
            // If we're unbound then inherit whatever the other type is.
            Variable::Unbound { .. } => Substitute(typ2.clone()),
        };

        return match action {
            Substitute(typ) => {
                *var1.borrow_mut() = Variable::Substitute(Box::new(typ));
                Ok(())
            }
            Reunify(typ) => unify(&typ, typ2),
            UnifyGenerics => {
                let var2 = unwrap_variable(typ2);
                // If both types are variable generics then first merge the
                // right into the left.
                {
                    let generic = Ref::map(var2.borrow(), |variable| match variable {
                        Variable::Generic(generic) => generic,
                        _ => unreachable!(),
                    });
                    unify_variable_generic_with_generic(typ1, &generic)?;
                }
                // Then make the right a substitute for the left.
                *var2.borrow_mut() = Variable::Substitute(Box::new(typ1.clone()));
                Ok(())
            }
        };
    }

    // If the other type is a variable then unify with it as the first term so
    // that we can reuse the logic above.
    if let Type::Variable(_) = &typ2 {
        return unify(typ2, typ1).map_err(|err| err.reverse());
    }

    match (typ1, typ2) {
        (Type::Func(func1), Type::Func(func2)) => {
            let (func1_arguments, func1_return, func2_arguments, func2_return) = {
                let func1 = func1.borrow();
                let func2 = func2.borrow();
                if func1.arity() != func2.arity() {
                    return Err(TypeError::TypeMismatch {
                        expected: typ1.clone(),
                        got: typ2.clone(),
                    });
                }
                (
                    func1.arguments.clone(),
                    (*func1.retrn).clone(),
                    func2.arguments.clone(),
                    (*func2.retrn).clone(),
                )
            };
            for (func1_argument, func2_argument) in
                func1_arguments.iter().zip(func2_arguments.iter())
            {
                unify(func1_argument, func2_argument)?;
            }
            return unify(&func1_return, &func2_return);
        }
        _ => (),
    }

    Err(TypeError::CannotUnify {
        first: typ1.clone(),
        second: typ2.clone(),
    })
}
