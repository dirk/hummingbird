/// The goal of inference and unification is to take an AST of variable types
/// (unbound types) and resolve them all into real (funcs, objects) or generic
/// types.
///
/// At the end of the process there should be no `Type::Variable` variants
/// remaining in the AST.
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::parse_ast as past;
use super::parser::{Span, Token, Word};

mod builtins;
mod nodes;
mod printer;
mod scope;
mod typ;

pub use builtins::Builtins;
pub use nodes::*;
pub use printer::Printer;
pub use scope::{FuncScope, ModuleScope, Scope, ScopeLike};
pub use typ::{Func as TFunc, Generic, GenericConstraint, PropertyConstraint, Type, Variable};

type TypeResult<T> = Result<T, TypeError>;

pub fn translate_module(pmodule: past::Module) -> TypeResult<Module> {
    let scope = ModuleScope::new().into_scope();

    let mut statements = vec![];
    for pstatement in pmodule.statements.into_iter() {
        let statement = match pstatement {
            past::ModuleStatement::Func(pfunc) => {
                ModuleStatement::Func(translate_func(pfunc, scope.clone())?)
            }
            _ => unreachable!(),
        };
        statements.push(statement);
    }

    // Now that we've translated the whole module we can close all of the
    // statements to ensure all types are bound.
    let mut closed_statements = vec![];
    for statement in statements.into_iter() {
        closed_statements.push(statement.close(&mut RecursionTracker::new())?);
    }

    Ok(Module {
        statements: closed_statements,
    })
}

fn translate_func(pfunc: past::Func, scope: Scope) -> TypeResult<Func> {
    let name = pfunc.name.name.clone();
    // Build the `FuncArgument` nodes ahead of time so that they have types
    // in place.
    let arguments_nodes = pfunc
        .arguments
        .iter()
        .map(|argument| FuncArgument {
            name: argument.name.clone(),
            // TODO: Support argument type definitions.
            typ: Type::new_unbound(),
        })
        .collect::<Vec<_>>();

    // We'll build the arguments and return types once and then reuse them for
    // the recursive forward declaration and the real type.
    let arguments = arguments_nodes
        .iter()
        .map(|argument| argument.typ.clone())
        .collect::<Vec<_>>();
    let retrn = Type::new_unbound();

    let func_scope = FuncScope::new(Some(scope.clone())).into_scope();
    for argument_node in arguments_nodes.iter() {
        func_scope.add_local(&argument_node.name, argument_node.typ.clone())?;
    }
    // Build a forward declaration for the recursive call and add it to
    // the function's scope.
    let typ = Type::new_func(Some(name.clone()), arguments.clone(), retrn.clone());
    func_scope.add_local(&name, typ.clone())?;

    let body = match pfunc.body {
        past::FuncBody::Block(block) => {
            FuncBody::Block(translate_block(block, func_scope.clone())?)
        }
    };

    let implicit_retrn = body.typ();
    unify(&retrn, &implicit_retrn)?;

    // Add the function to its defining scope.
    scope.add_local(&name, typ.clone())?;

    Ok(Func {
        name: name.clone(),
        arguments: arguments_nodes,
        body,
        scope: func_scope,
        typ,
    })
}

fn translate_block(pblock: past::Block, scope: Scope) -> TypeResult<Block> {
    let mut statements = vec![];
    for pstatement in pblock.statements {
        let statement = match pstatement {
            past::BlockStatement::CommentLine(_) => continue,
            past::BlockStatement::Func(pfunc) => {
                BlockStatement::Func(translate_func(pfunc, scope.clone())?)
            }
            past::BlockStatement::Expression(pexpression) => {
                BlockStatement::Expression(translate_expression(&pexpression, scope.clone())?)
            }
        };
        statements.push(statement);
    }
    let typ = if statements.is_empty() {
        Type::new_empty_tuple()
    } else {
        statements.last().unwrap().typ()
    };
    Ok(Block {
        statements,
        span: pblock.span.clone(),
        typ,
    })
}

fn translate_expression(pexpression: &past::Expression, scope: Scope) -> TypeResult<Expression> {
    Ok(match pexpression {
        past::Expression::Identifier(pidentifier) => {
            let name = pidentifier.name.clone();
            let typ = scope.get_local(&name.name)?;
            Expression::Identifier(Identifier { name, typ })
        }
        past::Expression::Infix(pinfix) => {
            let lhs = translate_expression(&*pinfix.lhs, scope.clone())?;
            let rhs = translate_expression(&*pinfix.rhs, scope)?;
            // Left- and right-hand sides must be the same in an infix operation.
            unify(lhs.typ(), rhs.typ())?;
            let typ = rhs.typ().clone();
            Expression::Infix(Infix {
                lhs: Box::new(lhs),
                op: pinfix.op.clone(),
                rhs: Box::new(rhs),
                typ,
            })
        }
        past::Expression::LiteralInt(pliteral) => {
            let class = Builtins::get("Int");
            Expression::LiteralInt(LiteralInt {
                value: pliteral.value,
                typ: Type::new_object(class),
            })
        }
        past::Expression::PostfixCall(pcall) => {
            Expression::PostfixCall(translate_postfix_call(pcall, scope)?)
        }
        past::Expression::PostfixProperty(pproperty) => {
            Expression::PostfixProperty(translate_postfix_property(pproperty, scope)?)
        }
        _ => unreachable!("Cannot translate {:?}", pexpression),
    })
}

fn translate_postfix_call(pcall: &past::PostfixCall, scope: Scope) -> TypeResult<PostfixCall> {
    let target = translate_expression(&*pcall.target, scope.clone())?;

    let mut arguments = vec![];
    for argument in pcall.arguments.iter() {
        arguments.push(translate_expression(argument, scope.clone())?);
    }

    // The return type of the callable.
    let retrn = Type::new_unbound();

    let mut generic = Generic::new();
    generic.add_callable_constraint(
        arguments
            .iter()
            .map(|argument| argument.typ().clone())
            .collect::<Vec<_>>(),
        retrn.clone(),
    );
    // Unify to ensure target supports being called with the arguments and
    // return types.
    let intermediary = Type::new_variable(Variable::Generic(generic));
    unify(target.typ(), &intermediary).map_err(|err| err.with_span(pcall.span.clone()))?;

    Ok(PostfixCall {
        target: Box::new(target),
        arguments,
        typ: retrn,
    })
}

fn translate_postfix_property(
    pproperty: &past::PostfixProperty,
    scope: Scope,
) -> TypeResult<PostfixProperty> {
    let target = translate_expression(&*pproperty.target, scope.clone())?;

    // The ultimate type of getting the target's property.
    let typ = Type::new_unbound();

    // Set up a variable generic with a property constraint.
    let mut generic = Generic::new();
    generic.add_property_constraint(pproperty.property.name.clone(), typ.clone());
    // Apply unification to ensure the target supports having the
    // given property.
    let intermediary = Type::new_variable(Variable::Generic(generic));
    unify(target.typ(), &intermediary).map_err(|err| err.with_span(pproperty.span.clone()))?;

    Ok(PostfixProperty {
        target: Box::new(target),
        property: pproperty.property.clone(),
        typ,
    })
}

#[derive(Clone, Debug)]
pub enum TypeError {
    LocalAlreadyDefined {
        name: String,
    },
    LocalNotFound {
        name: String,
    },
    // PropertyAlreadyDefined {
    //     name: String,
    // },
    CannotUnify {
        first: Type,
        second: Type,
    },
    TypeMismatch {
        expected: Type,
        got: Type,
    },
    ArgumentsMismatch {
        expected: Vec<Type>,
        got: Vec<Type>,
    },
    RecursiveType {
        id: usize,
    },
    /// When we try to `Type#close` and run into an `Unbound`.
    UnexpectedUnbound {
        id: usize,
    },
    WithSpan {
        wrapped: Box<TypeError>,
        span: Span,
    },
}

impl TypeError {
    /// Reverse the sides of two-sided errors.
    fn reverse(self) -> Self {
        use TypeError::*;
        match self {
            CannotUnify { first, second } => CannotUnify {
                first: second,
                second: first,
            },
            TypeMismatch { expected, got } => TypeMismatch {
                expected: got,
                got: expected,
            },
            other @ _ => other,
        }
    }

    pub fn short_message(&self) -> String {
        use TypeError::*;
        let message = match self.unwrap() {
            LocalAlreadyDefined { .. } => "LocalAlreadyDefined",
            LocalNotFound { .. } => "LocalNotFound",
            // PropertyAlreadyDefined { .. } => "PropertyAlreadyDefined",
            CannotUnify { .. } => "CannotUnify",
            TypeMismatch { .. } => "TypeMismatch",
            ArgumentsMismatch { .. } => "ArgumentsMismatch",
            RecursiveType { .. } => "RecursiveType",
            UnexpectedUnbound { .. } => "UnexpectedUnbound",
            WithSpan { .. } => unreachable!(),
        };
        message.to_string()
    }

    pub fn label_message(&self) -> String {
        use TypeError::*;
        let message = match self.unwrap() {
            CannotUnify { .. } => "Trying to unify here",
            TypeMismatch { .. } => "Mismatch occurred here",
            WithSpan { .. } => unreachable!(),
            _ => "Here",
        };
        message.to_string()
    }

    pub fn span(&self) -> Option<Span> {
        use TypeError::*;
        match self {
            WithSpan { span, .. } => Some(span.clone()),
            _ => None,
        }
    }

    /// Returns the underlying error with all metadata layers peeled off.
    pub fn unwrap(&self) -> TypeError {
        use TypeError::*;
        match self {
            WithSpan { wrapped, .. } => wrapped.unwrap(),
            other @ _ => other.clone(),
        }
    }

    /// Add a span to mark the location of the error. If it already has a span
    /// it does *not* overwrite the span.
    pub fn with_span(self, span: Span) -> Self {
        if self.span().is_some() {
            return self;
        }
        TypeError::WithSpan {
            wrapped: Box::new(self),
            span,
        }
    }
}

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
                            return Err(TypeError::ArgumentsMismatch {
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
            if func1.arity() != func2.arity() {
                return Err(TypeError::TypeMismatch {
                    expected: typ1.clone(),
                    got: typ2.clone(),
                });
            }
            for (func1_argument, func2_argument) in
                func1.arguments.iter().zip(func2.arguments.iter())
            {
                unify(func1_argument, func2_argument)?;
            }
            // return unify(&*func1.retrn, &*func2.retrn);
            return Ok(());
        }
        _ => (),
    }

    Err(TypeError::CannotUnify {
        first: typ1.clone(),
        second: typ2.clone(),
    })
}

/// To safely handle recursive types we must register forward declarations
/// of types while closing them. That way when reclosing the same type we
/// return the forward declaration rather than actually closing again.
struct RecursionTracker(HashMap<usize, Type>);

impl RecursionTracker {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn check(&self, id: &usize) -> Option<Type> {
        if let Some(known) = self.0.get(id) {
            return Some(known.clone());
        }
        None
    }

    pub fn add(&mut self, id: usize, typ: Type) {
        self.0.insert(id, typ);
    }
}

/// Nodes/types which consume themselves to produce a new node/type where all
/// the types in themselves and their children have been closed.
trait Closable {
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self>
    where
        Self: Sized;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::super::parse_ast as past;
    use super::super::parser::{Location, Span, Token, Word};
    use super::scope::FuncScope;
    use super::{
        translate_expression, translate_func, unify, ScopeLike, Type, TypeError, Variable,
    };
    use crate::type_ast::{Builtins, Closable, FuncBody};

    impl Type {
        fn variable(&self) -> Variable {
            match self {
                Type::Variable(variable) => variable.borrow().clone(),
                _ => unreachable!("Cannot get variable from {:?}", self),
            }
        }
    }

    impl Variable {
        fn substitution(&self) -> Type {
            match self {
                Variable::Substitute(substitute) => *substitute.clone(),
                _ => unreachable!("Cannot get substitution from {:?}", self),
            }
        }
    }

    fn word(name: &str) -> Word {
        Word {
            name: name.to_string(),
            span: Span::unknown(),
        }
    }

    #[test]
    fn test_unify_unbound() -> Result<(), TypeError> {
        // Check with unbound on the left.
        let phantom = Type::new_phantom();
        let unbound = Type::new_unbound();
        unify(&unbound, &phantom)?;
        assert_eq!(phantom, unbound.variable().substitution());

        // And with unbound on the right.
        let phantom = Type::new_phantom();
        let unbound = Type::new_unbound();
        unify(&phantom, &unbound)?;
        assert_eq!(phantom, unbound.variable().substitution());

        Ok(())
    }

    #[test]
    fn test_translate_infix() -> Result<(), TypeError> {
        // Test simple addition of two integer literals.
        let pexpression = past::Expression::Infix(past::Infix {
            lhs: Box::new(past::Expression::LiteralInt(past::LiteralInt {
                value: 1,
                span: Span::unknown(),
            })),
            op: Token::Plus(Location::unknown()),
            rhs: Box::new(past::Expression::LiteralInt(past::LiteralInt {
                value: 2,
                span: Span::unknown(),
            })),
        });
        translate_expression(&pexpression, FuncScope::new(None).into_scope()).map(|_| ())?;

        // Add an unbound variable with an integer literal; check that the
        // variable is substituted to an integer.
        let pexpression = past::Expression::Infix(past::Infix {
            lhs: Box::new(past::Expression::Identifier(past::Identifier {
                name: word("foo"),
            })),
            op: Token::Plus(Location::unknown()),
            rhs: Box::new(past::Expression::LiteralInt(past::LiteralInt {
                value: 2,
                span: Span::unknown(),
            })),
        });
        let foo = Type::new_unbound();
        let scope = FuncScope::new(None).into_scope();
        scope.add_local("foo", foo.clone())?;
        translate_expression(&pexpression, scope).map(|_| ())?;
        assert_eq!(
            foo,
            Type::new_substitute(Type::new_object(Builtins::get("Int")))
        );

        Ok(())
    }

    #[test]
    fn test_translate_func() -> Result<(), TypeError> {
        let pfunc_inner = past::Func {
            name: word("foo_inner"),
            arguments: vec![],
            body: past::FuncBody::Block(past::Block {
                // statements: vec![past::BlockStatement::Expression(past::Expression::Infix(
                //     past::Infix {
                //         lhs: Box::new(past::Expression::Identifier(past::Identifier {
                //             name: word("bar"),
                //         })),
                //         op: Token::Plus(Location::unknown()),
                //         rhs: Box::new(past::Expression::LiteralInt(past::LiteralInt {
                //             value: 1,
                //             span: Span::unknown(),
                //         })),
                //     },
                // ))],
                statements: vec![past::BlockStatement::Expression(
                    past::Expression::Identifier(past::Identifier { name: word("bar") }),
                )],
                span: Span::unknown(),
            }),
            span: Span::unknown(),
        };

        let pfunc = past::Func {
            name: word("foo"),
            arguments: vec![word("bar")],
            body: past::FuncBody::Block(past::Block {
                // statements: vec![past::BlockStatement::Expression(past::Expression::Infix(
                //     past::Infix {
                //         lhs: Box::new(past::Expression::Identifier(past::Identifier {
                //             name: word("bar"),
                //         })),
                //         op: Token::Plus(Location::unknown()),
                //         rhs: Box::new(past::Expression::LiteralInt(past::LiteralInt {
                //             value: 1,
                //             span: Span::unknown(),
                //         })),
                //     },
                // ))],
                statements: vec![
                    past::BlockStatement::Func(pfunc_inner),
                    past::BlockStatement::Expression(past::Expression::Infix(past::Infix {
                        lhs: Box::new(past::Expression::Identifier(past::Identifier {
                            name: word("bar"),
                        })),
                        op: Token::Plus(Location::unknown()),
                        rhs: Box::new(past::Expression::LiteralInt(past::LiteralInt {
                            value: 1,
                            span: Span::unknown(),
                        })),
                    })),
                ],
                span: Span::unknown(),
            }),
            span: Span::unknown(),
        };

        let scope = FuncScope::new(None).into_scope();
        let func = translate_func(pfunc, scope)?;

        println!("{:?}", func);

        Ok(())
    }
}
