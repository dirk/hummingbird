use super::super::parser::{Location, Span, Token, Word};
use super::scope::Scope;
use super::{Closable, Type, TypeResult};
use crate::type_ast::TypeError;

#[derive(Debug)]
pub struct Module {
    pub statements: Vec<ModuleStatement>,
}

#[derive(Debug)]
pub enum ModuleStatement {
    Func(Func),
}

impl Closable for ModuleStatement {
    fn close(self) -> TypeResult<Self> {
        use ModuleStatement::*;
        Ok(match self {
            Func(func) => Func(func.close()?),
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
    fn close(self) -> TypeResult<Self> {
        // Close the type first since that does important unbound-to-generic
        // auto-conversion.
        let typ = self.typ.close()?;
        let mut arguments = vec![];
        for argument in self.arguments {
            arguments.push(FuncArgument {
                name: argument.name,
                typ: argument.typ.close()?,
            });
        }
        Ok(Func {
            name: self.name,
            arguments,
            body: self.body.close()?,
            scope: self.scope.close()?,
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
    fn close(self) -> TypeResult<FuncBody> {
        use FuncBody::*;
        Ok(match self {
            Block(block) => Block(block.close()?),
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
    fn close(self) -> TypeResult<Self> {
        let mut statements = vec![];
        for statement in self.statements {
            statements.push(statement.close()?);
        }
        Ok(Block {
            statements,
            span: self.span,
            typ: self.typ.close()?,
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
    fn close(self) -> TypeResult<Self> {
        use BlockStatement::*;
        Ok(match self {
            Expression(expression) => Expression(expression.close()?),
            Func(func) => Func(func.close()?),
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
    fn close(self) -> TypeResult<Self> {
        use Expression::*;
        Ok(match self {
            Identifier(identifier) => Identifier(identifier.close()?),
            Infix(infix) => Infix(infix.close()?),
            literal @ LiteralInt(_) => literal,
            PostfixCall(call) => PostfixCall(call.close()?),
            PostfixProperty(property) => PostfixProperty(property.close()?),
        })
    }
}

#[derive(Debug)]
pub struct Identifier {
    pub name: Word,
    pub typ: Type,
}

impl Closable for Identifier {
    fn close(self) -> TypeResult<Self> {
        Ok(Identifier {
            name: self.name,
            typ: self.typ.close()?,
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
    fn close(self) -> TypeResult<Self> {
        Ok(Infix {
            lhs: Box::new(self.lhs.close()?),
            op: self.op,
            rhs: Box::new(self.rhs.close()?),
            typ: self.typ.close()?,
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
    fn close(self) -> TypeResult<Self> {
        let target = self.target.close()?;
        let mut arguments = vec![];
        for argument in self.arguments.into_iter() {
            arguments.push(argument.close()?);
        }
        let typ = self.typ.close()?;
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
    fn close(self) -> TypeResult<Self> {
        let typ = self.typ.close()?;
        Ok(PostfixProperty {
            target: Box::new(self.target.close()?),
            property: self.property,
            typ,
        })
    }
}
