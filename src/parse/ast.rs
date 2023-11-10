use super::{lexer::Word, location::Span};

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub imports: Vec<Import>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Import {
    pub whole: Option<ImportWhole>,
    pub members: Vec<ImportMember>,
    pub source: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportWhole {
    pub name: Word,
    pub alias: Option<Word>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportMember {
    pub name: Word,
    pub alias: Option<Word>,
}

pub struct Comment {
    pub span: Span,
}

pub struct CommentLine {
    pub span: Span,
    pub text: String,
}
