/// The goal of inference and unification is to take an AST of variable types
/// (unbound types) and resolve them all into real (funcs, objects) or generic
/// types.
///
/// At the end of the process there should be no `Type::Variable` variants
/// remaining in the AST.
use std::cell::{Cell, RefCell};
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
pub use typ::{Func as TFunc, Type, Variable};

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
        closed_statements.push(statement.close()?);
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
    func_scope.add_local(&name, typ.clone());

    let body = match pfunc.body {
        past::FuncBody::Block(block) => {
            FuncBody::Block(translate_block(block, func_scope.clone())?)
        }
    };

    // There should be no more inference necessary at this point so close the
    // function and add it to its defining scope.
    let typ = typ.close()?;
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
    Ok(Block {
        statements,
        span: pblock.span.clone(),
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
        _ => unreachable!("Cannot translate {:?}", pexpression),
    })
}

#[derive(Debug)]
pub enum TypeError {
    LocalAlreadyDefined {
        name: String,
    },
    LocalNotFound {
        name: String,
    },
    CannotUnify {
        first: Type,
        second: Type,
    },
    TypeMismatch {
        expected: Type,
        got: Type,
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
}

pub fn unify(typ1: &Type, typ2: &Type) -> Result<(), TypeError> {
    if typ1 == typ2 {
        return Ok(());
    }

    if let Type::Variable(var1) = typ1 {
        enum Action {
            // Make `typ1` a substitute for another type.
            Substitute(Type),
            // Used to unify a different type with `typ2` (eg. if `typ1` is a
            // substitute).
            Unify(Type),
        }
        use Action::*;

        let action = match &*var1.borrow() {
            // If this is a generic and the other type is unbound then
            // update the other type to also be unbound.
            generic @ Variable::Generic(_) => {
                if let Type::Variable(var2) = typ2 {
                    if var2.borrow().is_unbound() {
                        *var2.borrow_mut() = generic.clone();
                        return Ok(());
                    }
                }
                return Err(TypeError::TypeMismatch {
                    expected: typ1.clone(),
                    got: typ2.clone(),
                });
            }
            // If we're a substitute then unify whatever we're substituted with
            // with `typ2`.
            Variable::Substitute(substitute) => Unify(*substitute.clone()),
            // If we're unbound then inherit whatever the other type is.
            Variable::Unbound { .. } => Substitute(typ2.clone()),
        };

        return match action {
            Substitute(typ) => {
                *var1.borrow_mut() = Variable::Substitute(Box::new(typ));
                Ok(())
            }
            Unify(typ) => unify(&typ, typ2),
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

/// Nodes/types which consume themselves to produce a new node/type where all
/// the types in themselves and their children have been closed.
trait Closable {
    fn close(self) -> TypeResult<Self>
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
        let func = translate_func(pfunc, scope)?.close()?;

        println!("{:?}", func);

        Ok(())
    }
}
