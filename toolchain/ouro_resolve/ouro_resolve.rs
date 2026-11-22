use ouro_index_vec::{IndexVec, MaxOr};
use ouro_parse_types::{Node, NodeKind, Nodes};
use ouro_tokenize_types::ByteSpans;
use std::collections::HashMap;

ouro_index_vec::define_index_type! {
    pub struct Ref = u32;
}

#[derive(Debug)]
pub struct RefImpl {
    /// Either points to a Def or where the scope opened.
    pub node: Node,

    /// Linked list of other references to the same def.
    pub next: MaxOr<Ref>,
}
const _: () = assert!(size_of::<RefImpl>() == 8);

#[derive(Copy, Clone)]
enum Inst {
    Ref,
    Def,
    Open,
    Close,
}

fn inst(node_kind: NodeKind) -> Option<Inst> {
    match node_kind {
        // Def
        NodeKind::StructIdent
        | NodeKind::FnIdent
        | NodeKind::FnParamIdent
        | NodeKind::LetIdent
        | NodeKind::ConstIdent => Some(Inst::Def),

        // Ref
        NodeKind::Ident => Some(Inst::Ref),

        // Open
        NodeKind::FileBegin | NodeKind::StructBodyBegin | NodeKind::FnParams | NodeKind::Block => {
            Some(Inst::Open)
        }

        // Close
        NodeKind::FileEnd | NodeKind::StructBodyEnd | NodeKind::FnBodyEnd | NodeKind::BlockEnd => {
            Some(Inst::Close)
        }

        // Everything else
        _ => None,
    }
}

pub fn resolve(nodes: &Nodes, spans: &ByteSpans, source: &str) -> Resolve {
    let mut top_start = Node::from_raw(0);
    let mut stack = Vec::new();
    let mut dedup = HashMap::new();
    let mut refs = IndexVec::new();

    for (node, node_kind) in nodes.nodes.iter_enumerated() {
        match inst(*node_kind) {
            None => {}
            Some(Inst::Def) => stack.push(node),
            Some(Inst::Ref) => {
                let head = dedup
                    .entry(spans.to_span(nodes.tokens[node]).lookup(source))
                    .or_insert(MaxOr::max());
                *head = MaxOr::new(refs.push(RefImpl {
                    node: top_start,
                    next: *head,
                }));
            }
            Some(Inst::Open) => {
                stack.push(top_start);
                top_start = node;
            }
            Some(Inst::Close) => {
                loop {
                    let popped_node = stack.pop().expect("shouldn't run out of elements because we stop as soon as we hit matching open");
                    match inst(nodes.nodes[popped_node]) {
                        Some(Inst::Open) => {
                            top_start = popped_node;
                            break;
                        }
                        Some(Inst::Def) => {
                            let source = spans.to_span(nodes.tokens[popped_node]).lookup(source);
                            let Some(head) = dedup.get_mut(source) else {
                                continue;
                            };

                            while let Some(head_ref) = head.into_non_max() {
                                if refs[head_ref].node < top_start {
                                    break;
                                }
                                refs[head_ref].node = popped_node;
                                *head = refs[head_ref].next;
                            }
                        }
                        _ => unreachable!(
                            "should never reach here because we only push nodes that are classified as Def or Open"
                        ),
                    }
                }
            }
        }
    }

    Resolve { refs }
}

#[derive(Debug)]
pub struct Resolve {
    pub refs: IndexVec<Ref, RefImpl>,
}
