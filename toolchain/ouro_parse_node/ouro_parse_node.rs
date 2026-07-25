//! Nodes produced by the parser.

use ouro_tokenize::Token;

ouro_index_vec::define_index_type! {
    pub struct Node = u32;
}

#[derive(Copy, Clone, Debug)]
pub struct NodeImpl {
    pub token: Token,
    pub kind: NodeKind,
}

ouro_index_vec::define_index_type! {
    /// A syntactic reference.
    pub struct SynRef = u32;
    DEBUG_FORMAT = "SynRef({})";
}

#[derive(Copy, Clone, Debug)]
pub enum NodeKind {
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
    LetSemi,
    Const,
    ConstIdent,
    ConstEq,
    ConstSemi,
    ExprAdd,
    ExprSub,
    ExprMul,
    ExprDiv,
    ExprNot,
    ExprNeg,
    ExprBlock,
    ExprBlockEnd,
    ExprIdent(SynRef),
    ExprInt,
    ExprDot,
    ExprField,
    ExprCall,
    ExprCallComma,
    ExprCallEnd,
    ExprStr,
}

impl NodeKind {
    pub fn has_subtree_size(self) -> bool {
        matches!(
            self,
            NodeKind::StructFieldComma
                | NodeKind::StructBodyEnd
                | NodeKind::FnParamsEnd
                | NodeKind::FnBodyEnd
                | NodeKind::ExprBlockEnd
                | NodeKind::ExprCallEnd
        )
    }

    pub fn is_introducer(self) -> bool {
        matches!(
            self,
            NodeKind::Struct
                | NodeKind::Fn
                | NodeKind::FnParams
                | NodeKind::ExprBlock
                | NodeKind::ExprCall
                | NodeKind::StructFieldIdent
        )
    }
}
