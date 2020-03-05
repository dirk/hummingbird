use super::super::parser::{Location, Span, Token, Word};
use super::scope::{Scope, ScopeResolution};
use super::{Closable, RecursionTracker, Type, TypeResult};

#[derive(Debug)]
pub struct Module {
    pub statements: Vec<ModuleStatement>,
}

#[derive(Debug)]
pub enum ModuleStatement {
    Func(Func),
}

impl Closable for ModuleStatement {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        use ModuleStatement::*;
        Ok(match self {
            Func(func) => Func(func.close(tracker, scope)?),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Closure {
    pub arguments: Vec<FuncArgument>,
    pub body: Box<ClosureBody>,
    pub scope: Scope,
    pub typ: Type,
}

impl Closable for Closure {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        let typ = Type::close_func(self.typ, tracker, scope.clone())?;
        let mut arguments = vec![];
        for argument in self.arguments {
            arguments.push(FuncArgument {
                name: argument.name,
                typ: argument.typ.close(tracker, scope.clone())?,
            })
        }
        let body = (*self.body).close(tracker, scope.clone())?;
        Ok(Closure {
            arguments,
            body: Box::new(body),
            scope: self.scope.close(tracker, scope)?,
            typ,
        })
    }
}

#[derive(Clone, Debug)]
pub enum ClosureBody {
    Block(Block),
    Expression(Expression),
}

impl ClosureBody {
    pub fn typ(&self) -> &Type {
        use ClosureBody::*;
        match self {
            Block(block) => &block.typ,
            Expression(expression) => expression.typ(),
        }
    }
}

impl Closable for ClosureBody {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        use ClosureBody::*;
        Ok(match self {
            Block(block) => Block(block.close(tracker, scope)?),
            Expression(expression) => Expression(expression.close(tracker, scope)?),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: String,
    pub arguments: Vec<FuncArgument>,
    pub body: FuncBody,
    // The scope of variables defined within the function.
    pub scope: Scope,
    pub typ: Type,
}

impl Closable for Func {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        // We call `close_func` directly here since that does extra work:
        //   - It closes the `Func` type. This should only be closed once so
        //     there are special checks for it.
        //   - It does unbound-to-generic auto-conversion.
        let typ = Type::close_func(self.typ, tracker, scope.clone())?;
        let mut arguments = vec![];
        for argument in self.arguments {
            arguments.push(FuncArgument {
                name: argument.name,
                typ: argument.typ.close(tracker, scope.clone())?,
            });
        }
        Ok(Func {
            name: self.name,
            arguments,
            body: self.body.close(tracker, scope.clone())?,
            scope: self.scope.close(tracker, scope)?,
            typ,
        })
    }
}

#[derive(Clone, Debug)]
pub struct FuncArgument {
    pub name: String,
    pub typ: Type,
}

#[derive(Clone, Debug)]
pub enum FuncBody {
    Block(Block),
}

impl FuncBody {
    pub fn typ(&self) -> Type {
        use FuncBody::*;
        match self {
            Block(block) => block.typ.clone(),
        }
    }
}

impl Closable for FuncBody {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<FuncBody> {
        use FuncBody::*;
        Ok(match self {
            Block(block) => Block(block.close(tracker, scope)?),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<BlockStatement>,
    pub span: Span,
    /// The implicit return of the block.
    pub typ: Type,
}

impl Closable for Block {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        let mut statements = vec![];
        for statement in self.statements {
            statements.push(statement.close(tracker, scope.clone())?);
        }
        Ok(Block {
            statements,
            span: self.span,
            typ: self.typ.close(tracker, scope)?,
        })
    }
}

#[derive(Clone, Debug)]
pub enum BlockStatement {
    Expression(Expression),
    Func(Func),
    Var(Var),
}

impl BlockStatement {
    pub fn typ(&self) -> Type {
        use BlockStatement::*;
        match self {
            Expression(expression) => expression.typ().clone(),
            Func(func) => func.typ.clone(),
            Var(var) => var.typ.clone(),
        }
    }
}

impl Closable for BlockStatement {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        use BlockStatement::*;
        Ok(match self {
            Expression(expression) => Expression(expression.close(tracker, scope)?),
            Func(func) => Func(func.close(tracker, scope)?),
            Var(var) => Var(var.close(tracker, scope)?),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Var {
    pub name: Word,
    pub initializer: Option<Expression>,
    pub typ: Type,
}

impl Closable for Var {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        let initializer = match self.initializer {
            Some(expression) => Some(expression.close(tracker, scope.clone())?),
            None => None,
        };
        Ok(Self {
            name: self.name,
            initializer,
            typ: self.typ.close(tracker, scope)?,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Expression {
    Closure(Closure),
    Identifier(Identifier),
    Infix(Infix),
    LiteralInt(LiteralInt),
    PostfixCall(PostfixCall),
    PostfixProperty(PostfixProperty),
}

impl Expression {
    pub fn typ(&self) -> &Type {
        use Expression::*;
        match self {
            Closure(closure) => &closure.typ,
            Identifier(identifier) => &identifier.typ,
            Infix(infix) => &infix.typ,
            LiteralInt(literal) => &literal.typ,
            PostfixCall(call) => &call.typ,
            PostfixProperty(property) => &property.typ,
        }
    }
}

impl Closable for Expression {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        use Expression::*;
        Ok(match self {
            Closure(closure) => Closure(closure.close(tracker, scope)?),
            Identifier(identifier) => Identifier(identifier.close(tracker, scope)?),
            Infix(infix) => Infix(infix.close(tracker, scope)?),
            literal @ LiteralInt(_) => literal,
            PostfixCall(call) => PostfixCall(call.close(tracker, scope)?),
            PostfixProperty(property) => PostfixProperty(property.close(tracker, scope)?),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Identifier {
    pub name: Word,
    pub resolution: ScopeResolution,
    pub typ: Type,
}

impl Closable for Identifier {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        Ok(Identifier {
            name: self.name,
            resolution: self.resolution.close(tracker, scope.clone())?,
            typ: self.typ.close(tracker, scope)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Infix {
    pub lhs: Box<Expression>,
    pub op: Token,
    pub rhs: Box<Expression>,
    pub typ: Type,
}

impl Closable for Infix {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        Ok(Infix {
            lhs: Box::new(self.lhs.close(tracker, scope.clone())?),
            op: self.op,
            rhs: Box::new(self.rhs.close(tracker, scope.clone())?),
            typ: self.typ.close(tracker, scope)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct LiteralInt {
    pub value: i64,
    pub typ: Type,
}

#[derive(Clone, Debug)]
pub struct PostfixCall {
    pub target: Box<Expression>,
    pub arguments: Vec<Expression>,
    pub typ: Type,
}

impl Closable for PostfixCall {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        let target = self.target.close(tracker, scope.clone())?;
        let mut arguments = vec![];
        for argument in self.arguments.into_iter() {
            arguments.push(argument.close(tracker, scope.clone())?);
        }
        let typ = self.typ.close(tracker, scope)?;
        Ok(PostfixCall {
            target: Box::new(target),
            arguments,
            typ,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PostfixProperty {
    pub target: Box<Expression>,
    pub property: Word,
    pub typ: Type,
}

impl Closable for PostfixProperty {
    fn close(self, tracker: &mut RecursionTracker, scope: Scope) -> TypeResult<Self> {
        let target = self.target.close(tracker, scope.clone())?;
        let typ = self.typ.close(tracker, scope)?;
        Ok(PostfixProperty {
            target: Box::new(target),
            property: self.property,
            typ,
        })
    }
}
