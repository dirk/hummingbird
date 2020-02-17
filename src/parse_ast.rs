use super::parser::{Span, Token, Word};

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub statements: Vec<ModuleStatement>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModuleStatement {
    CommentLine(CommentLine),
    Func(Func),
    Import(Import),
    Struct(Struct),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommentLine {
    pub content: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Func {
    pub name: Word,
    pub arguments: Vec<Word>,
    pub body: FuncBody,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FuncBody {
    Block(Block),
}

impl FuncBody {
    pub fn span(&self) -> Span {
        use FuncBody::*;
        match self {
            Block(block) => block.span.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Var {
    pub name: Word,
    pub initializer: Option<Expression>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Import {
    pub path: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Struct {
    pub name: Word,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub statements: Vec<BlockStatement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlockStatement {
    CommentLine(CommentLine),
    Expression(Expression),
    Var(Var),
    Func(Func),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Identifier(Identifier),
    Infix(Infix),
    LiteralInt(LiteralInt),
    PostfixCall(PostfixCall),
    PostfixProperty(PostfixProperty),
}

impl Expression {
    pub fn span(&self) -> Span {
        use Expression::*;
        match self {
            Identifier(identifier) => identifier.name.span.clone(),
            Infix(infix) => {
                let start = infix.lhs.span().start;
                let end = infix.rhs.span().end;
                Span::new(start, end)
            }
            LiteralInt(literal) => literal.span.clone(),
            PostfixCall(call) => call.span.clone(),
            PostfixProperty(property) => property.span.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Identifier {
    pub name: Word,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Infix {
    pub lhs: Box<Expression>,
    pub op: Token,
    pub rhs: Box<Expression>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiteralInt {
    pub value: i64,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PostfixCall {
    pub target: Box<Expression>,
    pub arguments: Vec<Expression>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PostfixProperty {
    pub target: Box<Expression>,
    pub property: Word,
    pub span: Span,
}
