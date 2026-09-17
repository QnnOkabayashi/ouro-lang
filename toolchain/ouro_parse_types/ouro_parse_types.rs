//! Nodes produced by the parser.

use ouro_index_vec::IndexVec;
use ouro_tokenize_types::Token;

ouro_index_vec::define_index_type! {
    pub struct Node = u32;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    Add,
    Sub,
    Mul,
    Div,
    Not,
    Neg,
    Block,
    BlockEnd,
    Ident,
    IntLit,
    Dot,
    Field,
    Call,
    CallEnd,
    Str,
    I32Keyword,
    TypeKeyword,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    FileBegin,
    FileEnd,
    Pub,
    Struct,
    StructBodyBegin,
    StructFieldIdent,
    StructFieldComma,
    StructBodyEnd,
    Fn,
    FnIdent,
    FnParams,
    FnParamsIdent,
    FnParamsEnd,
    FnBodyBegin,
    FnBodyEnd,
    Let,
    LetIdent,
    LetEq,
    Const,
    ConstIdent,
    ConstEq,
    Expr(ExprKind),
    BuiltinAmpersand,
    BuiltinIdent,
    EndOfExpr,
}

impl NodeKind {
    pub fn is_ident(self) -> bool {
        matches!(
            self,
            NodeKind::StructFieldIdent
                | NodeKind::FnIdent
                | NodeKind::FnParamsIdent
                | NodeKind::LetIdent
                | NodeKind::ConstIdent
                | NodeKind::Expr(ExprKind::Ident)
                | NodeKind::BuiltinIdent
        )
    }

    pub fn has_token(self) -> bool {
        match self {
            NodeKind::FileBegin | NodeKind::FileEnd => false,
            _ => true,
        }
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
