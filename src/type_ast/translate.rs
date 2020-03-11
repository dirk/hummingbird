use super::super::parse_ast as past;
use super::nodes::*;
use super::scope::{ClosureScope, FuncScope, ModuleScope, Scope, ScopeLike};
use super::typ::{Generic, Type, Variable};
use super::{unify, Builtins, Closable, RecursionTracker, TypeError, TypeResult};

pub fn translate_module(pmodule: past::Module) -> TypeResult<Module> {
    let scope = ModuleScope::new().into_scope();

    let mut statements = vec![];
    for pstatement in pmodule.statements.into_iter() {
        let statement = match pstatement {
            past::ModuleStatement::Func(pfunc) => {
                ModuleStatement::Func(translate_func(&pfunc, scope.clone())?)
            }
            past::ModuleStatement::CommentLine(_) => continue,
            _ => unreachable!(),
        };
        statements.push(statement);
        // NOTE: Closing should already be done within the translation of every
        //   module-level statement:
        // statements.push(statement.close(&mut RecursionTracker::new(), scope.clone())?);
    }

    // Now that we've translated the whole module we can close all of the
    // statements to ensure all types are bound.
    // let mut closed_statements = vec![];
    // for statement in statements.into_iter() {
    //     closed_statements.push(statement.close(&mut RecursionTracker::new())?);
    // }

    Ok(Module { statements, scope })
}

fn translate_func(pfunc: &past::Func, scope: Scope) -> TypeResult<Func> {
    let name = pfunc.name.name.clone();
    // The scope that the function's arguments and body will be evaluated in.
    let func_scope = FuncScope::new(Some(scope.clone())).into_scope();

    // Build the `FuncArgument` nodes ahead of time so that they have types
    // in place.
    let arguments_nodes = pfunc
        .arguments
        .iter()
        .map(|argument| FuncArgument {
            name: argument.name.clone(),
            // TODO: Support argument type definitions.
            typ: Type::new_unbound(func_scope.clone()),
        })
        .collect::<Vec<_>>();

    for argument_node in arguments_nodes.iter() {
        func_scope.add_local(&argument_node.name, argument_node.typ.clone())?;
    }
    // Build a forward declaration for the recursive call.
    let retrn = Type::new_unbound(func_scope.clone());
    let typ = Type::new_func(
        Some(name.clone()),
        arguments_nodes
            .iter()
            .map(|argument| argument.typ.clone())
            .collect::<Vec<_>>(),
        retrn.clone(),
        scope.clone(),
    );
    // Add the function to its defining scope.
    // TODO: This should be an `add_static` once functions are static.
    scope.add_local(&name, typ.clone())?;

    let body = match &pfunc.body {
        past::FuncBody::Block(block) => {
            FuncBody::Block(translate_block(block, func_scope.clone())?)
        }
    };

    let implicit_retrn = body.typ();
    unify(&retrn, &implicit_retrn, func_scope.clone())?;

    Ok(Func {
        name: name.clone(),
        arguments: arguments_nodes,
        body,
        scope: func_scope.clone(),
        typ,
    }
    .close(&mut RecursionTracker::new(), func_scope)?)
}

fn translate_block(pblock: &past::Block, scope: Scope) -> TypeResult<Block> {
    let mut statements = vec![];
    for pstatement in &pblock.statements {
        let statement = match pstatement {
            past::BlockStatement::CommentLine(_) => continue,
            past::BlockStatement::Func(pfunc) => {
                BlockStatement::Func(translate_func(pfunc, scope.clone())?)
            }
            past::BlockStatement::Expression(pexpression) => {
                BlockStatement::Expression(translate_expression(&pexpression, scope.clone())?)
            }
            past::BlockStatement::Var(pvar) => {
                BlockStatement::Var(translate_var(pvar, scope.clone())?)
            }
        };
        statements.push(statement);
    }
    let typ = if statements.is_empty() {
        Type::new_empty_tuple(scope)
    } else {
        statements.last().unwrap().typ()
    };
    Ok(Block {
        statements,
        span: pblock.span.clone(),
        typ,
    })
}

fn translate_var(pvar: &past::Var, scope: Scope) -> TypeResult<Var> {
    let typ = Type::new_unbound(scope.clone());
    scope.add_local(&pvar.name.name, typ.clone())?;
    let initializer = match &pvar.initializer {
        Some(expression) => {
            let initializer = translate_expression(expression, scope.clone())?;
            unify(&typ, initializer.typ(), scope)?;
            Some(initializer)
        }
        None => None,
    };
    Ok(Var {
        name: pvar.name.clone(),
        initializer,
        typ,
    })
}

fn translate_expression(pexpression: &past::Expression, scope: Scope) -> TypeResult<Expression> {
    Ok(match pexpression {
        past::Expression::Closure(pclosure) => {
            Expression::Closure(translate_closure(pclosure, scope)?)
        }
        past::Expression::Identifier(pidentifier) => {
            let name = pidentifier.name.clone();
            let resolution = scope
                .get_local(&name.name)
                .map_err(|err| err.with_span(pidentifier.name.span.clone()))?;
            let typ = resolution.typ();
            Expression::Identifier(Identifier {
                name,
                resolution,
                typ,
            })
        }
        past::Expression::Infix(pinfix) => {
            let lhs = translate_expression(&*pinfix.lhs, scope.clone())?;
            let rhs = translate_expression(&*pinfix.rhs, scope.clone())?;
            // Left- and right-hand sides must be the same in an infix operation.
            unify(lhs.typ(), rhs.typ(), scope)?;
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
                typ: Type::new_object(class, scope),
            })
        }
        past::Expression::PostfixCall(pcall) => {
            Expression::PostfixCall(translate_postfix_call(pcall, scope)?)
        }
        past::Expression::PostfixProperty(pproperty) => {
            Expression::PostfixProperty(translate_postfix_property(pproperty, scope)?)
        }
    })
}

fn translate_closure(pclosure: &past::Closure, scope: Scope) -> TypeResult<Closure> {
    let closure_scope = ClosureScope::new(Some(scope.clone())).into_scope();

    let arguments_nodes = pclosure
        .arguments
        .iter()
        .map(|argument| FuncArgument {
            name: argument.name.clone(),
            typ: Type::new_unbound(closure_scope.clone()),
        })
        .collect::<Vec<_>>();

    let retrn = Type::new_unbound(closure_scope.clone());

    for argument_node in arguments_nodes.iter() {
        closure_scope.add_local(&argument_node.name, argument_node.typ.clone())?;
    }

    let body = match &*pclosure.body {
        past::ClosureBody::Block(block) => {
            ClosureBody::Block(translate_block(block, closure_scope.clone())?)
        }
        past::ClosureBody::Expression(expression) => {
            ClosureBody::Expression(translate_expression(expression, closure_scope.clone())?)
        }
    };

    let implicit_retrn = body.typ();
    unify(&retrn, implicit_retrn, closure_scope.clone())?;

    let typ = Type::new_func(
        None,
        arguments_nodes
            .iter()
            .map(|argument| argument.typ.clone())
            .collect::<Vec<_>>(),
        retrn.clone(),
        scope.clone(),
    );

    Ok(Closure {
        arguments: arguments_nodes,
        body: Box::new(body),
        scope: closure_scope.clone(),
        typ,
    }
    .close(&mut RecursionTracker::new(), closure_scope)?)
}

fn translate_postfix_call(pcall: &past::PostfixCall, scope: Scope) -> TypeResult<PostfixCall> {
    let target = translate_expression(&*pcall.target, scope.clone())?;

    let mut arguments = vec![];
    for argument in pcall.arguments.iter() {
        arguments.push(translate_expression(argument, scope.clone())?);
    }

    let retrn = {
        // If the target is already a callable then unify directly instead of
        // through a constraint.
        if let Some((target_arguments, target_retrn)) = target.typ().maybe_callable() {
            let call_arguments = arguments
                .iter()
                .map(|argument| argument.typ().clone())
                .collect::<Vec<_>>();
            if target_arguments.len() != call_arguments.len() {
                return Err(TypeError::ArgumentLengthMismatch {
                    expected: target_arguments,
                    got: call_arguments,
                });
            }
            let mut tracker = RecursionTracker::new();
            for (target_argument, call_argument) in
                target_arguments.iter().zip(call_arguments.iter())
            {
                let open_target_argument =
                    target_argument.open_duplicate(&mut tracker, scope.clone());
                unify(call_argument, &open_target_argument, scope.clone())?;
            }
            let call_retrn = Type::new_unbound(scope.clone());
            let open_target_retrn = target_retrn.open_duplicate(&mut tracker, scope.clone());
            unify(&call_retrn, &open_target_retrn, scope)?;
            call_retrn

        // Otherwise build a callable generic constraint as an intermediary
        // and unify through that.
        } else {
            // The return type of the callable.
            let retrn = Type::new_unbound(scope.clone());

            let mut generic = Generic::new(scope.clone());
            generic.add_callable_constraint(
                arguments
                    .iter()
                    .map(|argument| argument.typ().clone())
                    .collect::<Vec<_>>(),
                retrn.clone(),
            );
            // Unify to ensure target supports being called with the arguments and
            // return types.
            let intermediary = Type::new_variable(Variable::Generic {
                scope: scope.clone(),
                generic,
            });
            unify(target.typ(), &intermediary, scope)
                .map_err(|err| err.with_span(pcall.span.clone()))?;
            retrn
        }
    };

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
    let typ = Type::new_unbound(scope.clone());

    // Set up a variable generic with a property constraint.
    let mut generic = Generic::new(scope.clone());
    generic.add_property_constraint(pproperty.property.name.clone(), typ.clone());
    // Apply unification to ensure the target supports having the
    // given property.
    let intermediary = Type::new_variable(Variable::Generic {
        scope: scope.clone(),
        generic,
    });
    unify(target.typ(), &intermediary, scope)
        .map_err(|err| err.with_span(pproperty.span.clone()))?;

    Ok(PostfixProperty {
        target: Box::new(target),
        property: pproperty.property.clone(),
        typ,
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::parse_ast as past;
    use super::super::super::parser::{Location, Span, Token, Word};
    use super::super::scope::{ClosureScope, ScopeLike};
    use super::super::{Builtins, Type, TypeError, Variable};
    use super::{translate_expression, translate_func};

    fn word<S: AsRef<str>>(name: S) -> Word {
        Word {
            name: name.as_ref().to_string(),
            span: Span::unknown(),
        }
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
        translate_expression(&pexpression, ClosureScope::new(None).into_scope()).map(|_| ())?;

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
        let scope = ClosureScope::new(None).into_scope();
        let foo = Type::new_unbound(scope.clone());
        scope.add_local("foo", foo.clone())?;
        translate_expression(&pexpression, scope.clone()).map(|_| ())?;
        assert_eq!(
            foo,
            Type::new_substitute(Type::new_object(Builtins::get("Int"), scope.clone()), scope)
        );

        Ok(())
    }

    #[test]
    fn test_translate_func() -> Result<(), TypeError> {
        let pclosure = past::Closure {
            arguments: vec![],
            body: Box::new(past::ClosureBody::Block(past::Block {
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
            })),
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
                    past::BlockStatement::Expression(past::Expression::Closure(pclosure)),
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

        let scope = ClosureScope::new(None).into_scope();
        let func = translate_func(&pfunc, scope)?;

        println!("{:?}", func);

        Ok(())
    }
}
