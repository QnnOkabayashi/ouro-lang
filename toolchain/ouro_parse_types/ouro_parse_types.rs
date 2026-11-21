//! Nodes produced by the parser.

use ouro_index_vec::IndexVec;
use ouro_tokenize_types::Token;

ouro_index_vec::define_index_type! {
    pub struct Node = u32;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    FileBegin,
    FileEnd,
    Pub,
    Struct,
    StructIdent,
    StructBodyBegin,
    StructFieldIdent,
    StructFieldEnd,
    StructBodyEnd,
    Fn,
    FnIdent,
    FnParams,
    FnParamIdent,
    FnParamEnd,
    FnParamsEnd,
    FnBodyBegin,
    FnBodyEnd,
    Let,
    LetIdent,
    LetEq,
    LetEnd,
    Const,
    ConstIdent,
    ConstEq,
    ConstEnd,
    Lt,
    Gt,
    Le,
    Ge,
    EqEq,
    Add,
    Sub,
    Mul,
    Div,
    Not,
    Neg,
    Block,
    BlockEnd,
    If,
    Else,
    IfEnd,
    Ident,
    IntLit,
    Dot,
    Field,
    Call,
    CallArgIdent,
    CallEnd,
    Str,
    I32Keyword,
    TypeKeyword,
    BuiltinAt,
    BuiltinIdent,
}

impl NodeKind {
    pub fn has_token(self) -> bool {
        !matches!(self, NodeKind::FileBegin | NodeKind::FileEnd)
    }
}

#[derive(Debug)]
pub struct Nodes {
    pub nodes: IndexVec<Node, NodeKind>,
    pub tokens: IndexVec<Node, Token>,
}

impl Nodes {
    pub fn push(&mut self, token: Token, node_kind: NodeKind) {
        self.nodes.push(node_kind);
        self.tokens.push(token);
    }
}
