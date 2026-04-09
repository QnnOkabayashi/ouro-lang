use crate::node::{Counter, Node, NodeImpl, NodeKind, Nominal, SynDef, SynRef};
use crate::resolver::{Builtin, Def};
use crate::span::Offset;
use crate::tokenize::Token;
use index_vec::IndexSlice;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Builtin(Builtin),
    SynRef(SynRef),
    Undefined,
}

impl Value {
    fn add(&mut self, other: Self) {
        match (self, other) {
            (Value::Int(lhs), Value::Int(rhs)) => *lhs += rhs,
            (lhs, rhs) => panic!("unsupported operation: {lhs:?} + {rhs:?}"),
        }
    }
}

pub fn eval(
    input: &str,
    ends: &IndexSlice<Token, [Offset]>,
    nodes: &IndexSlice<Node, [NodeImpl]>,
    syn_refs: &IndexSlice<SynRef, [Def]>,
    syn_defs: Counter<SynDef>,
    _nominals: Counter<Nominal>,
) {
    let mut stack: Vec<Value> = Vec::new();
    let mut _defs: Box<IndexSlice<Node, [Value]>> =
        index_vec::index_box![Value::Undefined; syn_defs.next.index()];
    for node in nodes {
        match node.kind {
            NodeKind::Struct(_nominal) => todo!(),
            NodeKind::StructIdent(_syn_def) => todo!(),
            NodeKind::StructBody => todo!(),
            NodeKind::StructFieldIdent => todo!(),
            NodeKind::StructFieldColon => todo!(),
            NodeKind::StructFieldComma(_subtree_size) => todo!(),
            NodeKind::StructBodyEnd(_subtree_size) => todo!(),
            NodeKind::Fn => {} // do nothing idk
            NodeKind::FnIdent(_syn_def) => todo!(),
            NodeKind::FnParams => {}
            NodeKind::FnParamsParam(_syn_def) => {
                // There is an expression on the stack, it is the type of this
                // param.
            }
            NodeKind::FnParamsEnd(_subtree_size) => todo!(),
            NodeKind::FnBody => todo!(),
            NodeKind::FnBodyEnd(_subtree_size) => todo!(),
            NodeKind::Let => todo!(),
            NodeKind::LetIdent(_syn_def) => todo!(),
            NodeKind::LetEq => todo!(),
            NodeKind::LetSemi => todo!(),
            NodeKind::ExprAdd => {
                let Some(rhs) = stack.pop() else { panic!() };
                let Some(lhs) = stack.last_mut() else {
                    panic!()
                };
                lhs.add(rhs);
            }
            NodeKind::ExprSub => todo!(),
            NodeKind::ExprMul => todo!(),
            NodeKind::ExprDiv => todo!(),
            NodeKind::ExprNot => todo!(),
            NodeKind::ExprNeg => todo!(),
            NodeKind::ExprBlock => todo!(),
            NodeKind::ExprBlockEnd(_subtree_size) => todo!(),
            NodeKind::ExprIdent(syn_ref) => {
                match syn_refs[syn_ref] {
                    Def::Local { def: _ } => {
                        // wtf do we do now
                    }
                    Def::NonLocal(Some(builtin)) => stack.push(Value::Builtin(builtin)),
                    Def::NonLocal(None) => panic!("unbound name ref"),
                }
            }
            NodeKind::ExprInt => {
                let src = crate::tokenize::span(node.token, ends).lookup(input);
                // unfortunately, the lexer is very lax.
                // src is anything that matches [0-9][a-zA-Z0-9_]*
                // It could be binary, hex, or just an actual int, or an error.
                // To keep things simple right now, we'll just parse as u64 and hope
                // for the best.
                let int = src.parse().expect("welp");
                stack.push(Value::Int(int));
            }
            NodeKind::ExprDot => todo!(),
            NodeKind::ExprField(_sem_ref) => todo!(),
            NodeKind::ExprCall => todo!(),
            NodeKind::ExprCallComma => todo!(),
            NodeKind::ExprCallEnd(_subtree_size) => todo!(),
        }
    }
}
