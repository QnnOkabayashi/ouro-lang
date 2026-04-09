//! Nodes produced by the parser.

use index_vec::Idx;

use crate::tokenize::Token;

index_vec::define_index_type! {
    pub struct Node = u32;
}

index_vec::define_index_type! {
    pub struct Nominal = u32;
}

pub struct Counter<I> {
    pub next: I,
}

impl<I: Idx> Counter<I> {
    pub fn new() -> Self {
        Counter {
            next: I::from_usize(0),
        }
    }

    pub fn next(&mut self) -> I {
        let next = self.next;
        self.next = I::from_usize(next.index() + 1);
        next
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NodeImpl {
    pub token: Token,
    /// Note that this is only for dynamic sized nodes.
    /// Nodes that have a fixed number of subtrees do not use this
    pub kind: NodeKind,
}

index_vec::define_index_type! {
    /// A syntactic function.
    pub struct SynFn = u32;
    DEBUG_FORMAT = "SynFn({})";
}

index_vec::define_index_type! {
    /// A capture.
    pub struct Capture = u32;
    DEBUG_FORMAT = "Capture({})";
}

index_vec::define_index_type! {
    /// A syntactic reference.
    pub struct SynRef = u32;
    DEBUG_FORMAT = "SynRef({})";
}

index_vec::define_index_type! {
    /// A syntactic definition.
    pub struct SynDef = u32;
    DEBUG_FORMAT = "SynDef({})";
}

index_vec::define_index_type! {
    /// A semantic reference.
    pub struct SemRef = u32;
    DEBUG_FORMAT = "SemRef({})";
}

#[derive(Copy, Clone, Debug)]
pub struct SubtreeSize(pub u32);

#[derive(Copy, Clone, Debug)]
pub enum NodeKind {
    Struct(Nominal),
    StructIdent(SynDef),
    StructBody,
    StructFieldIdent,
    StructFieldColon,
    StructFieldComma(SubtreeSize),
    StructBodyEnd(SubtreeSize),
    Fn,
    FnIdent(SynDef),
    FnParams,
    FnParamsParam(SynDef),
    FnParamsEnd(SubtreeSize),
    FnBody,
    FnBodyEnd(SubtreeSize),
    Let,
    LetIdent(SynDef),
    LetEq,
    LetSemi,
    ExprAdd,
    ExprSub,
    ExprMul,
    ExprDiv,
    ExprNot,
    ExprNeg,
    ExprBlock,
    ExprBlockEnd(SubtreeSize),
    ExprIdent(SynRef),
    ExprInt,
    ExprDot,
    ExprField(SemRef),
    ExprCall,
    ExprCallComma,
    ExprCallEnd(SubtreeSize),
}

impl NodeKind {
    pub fn subtree_size(self) -> Option<SubtreeSize> {
        match self {
            NodeKind::StructBodyEnd(subtree_size)
            | NodeKind::FnParamsEnd(subtree_size)
            | NodeKind::FnBodyEnd(subtree_size)
            | NodeKind::ExprBlockEnd(subtree_size)
            | NodeKind::ExprCallEnd(subtree_size) => Some(subtree_size),
            _ => None,
        }
    }
}
