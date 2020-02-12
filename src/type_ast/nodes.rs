use super::super::parser::{Location, Span, Token, Word};
use super::scope::Scope;
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
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        use ModuleStatement::*;
        Ok(match self {
            Func(func) => Func(func.close(tracker)?),
        })
    }
}

#[derive(Debug)]
pub struct Func {
    pub name: String,
    pub arguments: Vec<FuncArgument>,
    pub body: FuncBody,
    pub scope: Scope,
    pub typ: Type,
}

impl Closable for Func {
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        // Close the type first since that does important unbound-to-generic
        // auto-conversion.
        let typ = self.typ.close(tracker)?;
        let mut arguments = vec![];
        for argument in self.arguments {
            arguments.push(FuncArgument {
                name: argument.name,
                typ: argument.typ.close(tracker)?,
            });
        }
        Ok(Func {
            name: self.name,
            arguments,
            body: self.body.close(tracker)?,
            scope: self.scope.close(tracker)?,
            typ,
        })
    }
}

#[derive(Debug)]
pub struct FuncArgument {
    pub name: String,
    pub typ: Type,
}

#[derive(Debug)]
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
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<FuncBody> {
        use FuncBody::*;
        Ok(match self {
            Block(block) => Block(block.close(tracker)?),
        })
    }
}

#[derive(Debug)]
pub struct Block {
    pub statements: Vec<BlockStatement>,
    pub span: Span,
    /// The implicit return of the block.
    pub typ: Type,
}

impl Closable for Block {
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        let mut statements = vec![];
        for statement in self.statements {
            statements.push(statement.close(tracker)?);
        }
        Ok(Block {
            statements,
            span: self.span,
            typ: self.typ.close(tracker)?,
        })
    }
}

#[derive(Debug)]
pub enum BlockStatement {
    Expression(Expression),
    Func(Func),
}

impl BlockStatement {
    pub fn typ(&self) -> Type {
        use BlockStatement::*;
        match self {
            Expression(expression) => expression.typ().clone(),
            Func(func) => func.typ.clone(),
        }
    }
}

impl Closable for BlockStatement {
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        use BlockStatement::*;
        Ok(match self {
            Expression(expression) => Expression(expression.close(tracker)?),
            Func(func) => Func(func.close(tracker)?),
        })
    }
}

#[derive(Debug)]
pub enum Expression {
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
            Identifier(identifier) => &identifier.typ,
            Infix(infix) => &infix.typ,
            LiteralInt(literal) => &literal.typ,
            PostfixCall(call) => &call.typ,
            PostfixProperty(property) => &property.typ,
        }
    }
}

impl Closable for Expression {
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        use Expression::*;
        Ok(match self {
            Identifier(identifier) => Identifier(identifier.close(tracker)?),
            Infix(infix) => Infix(infix.close(tracker)?),
            literal @ LiteralInt(_) => literal,
            PostfixCall(call) => PostfixCall(call.close(tracker)?),
            PostfixProperty(property) => PostfixProperty(property.close(tracker)?),
        })
    }
}

#[derive(Debug)]
pub struct Identifier {
    pub name: Word,
    pub typ: Type,
}

impl Closable for Identifier {
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        Ok(Identifier {
            name: self.name,
            typ: self.typ.close(tracker)?,
        })
    }
}

#[derive(Debug)]
pub struct Infix {
    pub lhs: Box<Expression>,
    pub op: Token,
    pub rhs: Box<Expression>,
    pub typ: Type,
}

impl Closable for Infix {
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        Ok(Infix {
            lhs: Box::new(self.lhs.close(tracker)?),
            op: self.op,
            rhs: Box::new(self.rhs.close(tracker)?),
            typ: self.typ.close(tracker)?,
        })
    }
}

#[derive(Debug)]
pub struct LiteralInt {
    pub value: i64,
    pub typ: Type,
}

#[derive(Debug)]
pub struct PostfixCall {
    pub target: Box<Expression>,
    pub arguments: Vec<Expression>,
    pub typ: Type,
}

impl Closable for PostfixCall {
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        let target = self.target.close(tracker)?;
        let mut arguments = vec![];
        for argument in self.arguments.into_iter() {
            arguments.push(argument.close(tracker)?);
        }
        let typ = self.typ.close(tracker)?;
        Ok(PostfixCall {
            target: Box::new(target),
            arguments,
            typ,
        })
    }
}

#[derive(Debug)]
pub struct PostfixProperty {
    pub target: Box<Expression>,
    pub property: Word,
    pub typ: Type,
}

impl Closable for PostfixProperty {
    fn close(self, tracker: &mut RecursionTracker) -> TypeResult<Self> {
        let typ = self.typ.close(tracker)?;
        Ok(PostfixProperty {
            target: Box::new(self.target.close(tracker)?),
            property: self.property,
            typ,
        })
    }
}
