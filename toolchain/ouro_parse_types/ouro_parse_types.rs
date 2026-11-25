//! Nodes produced by the parser.

use ouro_index_vec::{IndexSlice, IndexVec};
use ouro_tokenize_types::{ByteSpans, Token};

ouro_index_vec::define_index_type! {
    #[derive(Default)]
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
    FnParamsEnd,
    FnBodyBegin,
    FnBodyEnd,
    Let,
    LetIdent,
    LetEq,
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
    Field,
    Call,
    CallArgIdent,
    CallEnd,
    Str,
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

    pub fn source<'a>(&'a self, spans: &'a ByteSpans, source: &'a str) -> NodeSource<'a> {
        NodeSource {
            tokens: &self.tokens,
            spans,
            source,
        }
    }
}

#[derive(Copy, Clone)]
pub struct NodeSource<'a> {
    pub tokens: &'a IndexSlice<Node, [Token]>,
    pub spans: &'a ByteSpans,
    pub source: &'a str,
}

impl<'a> NodeSource<'a> {
    pub fn text(&self, node: Node) -> &'a str {
        self.spans.to_span(self.tokens[node]).lookup(self.source)
    }
}
