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
            typ: self.typ.close()?,
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
        })
    }
}

#[derive(Debug)]
pub enum BlockStatement {
    Expression(Expression),
    Func(Func),
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
}

impl Expression {
    pub fn typ(&self) -> &Type {
        use Expression::*;
        match self {
            Identifier(identifier) => &identifier.typ,
            Infix(infix) => &infix.typ,
            LiteralInt(literal) => &literal.typ,
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
