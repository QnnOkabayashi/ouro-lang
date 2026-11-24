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
}

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

#[derive(Default)]
struct DedupEntry {
    head: MaxOr<Ref>,
    last_scope: MaxOr<Node>,
}

pub fn resolve(nodes: &Nodes, spans: &ByteSpans, source: &str) -> Resolve {
    let mut scope_start = Node::from_raw(0);
    let mut stack = Vec::new();
    let mut dedup: HashMap<&str, DedupEntry> = HashMap::new();
    let mut refs = IndexVec::new();
    let mut next: IndexVec<Ref, MaxOr<Ref>> = IndexVec::new();
    let mut duplicates = Vec::new();

    for (node, node_kind) in nodes.nodes.iter_enumerated() {
        match inst(*node_kind) {
            None => {}
            Some(Inst::Def) => stack.push(node),
            Some(Inst::Ref) => {
                let entry = dedup
                    .entry(spans.to_span(nodes.tokens[node]).lookup(source))
                    .or_default();
                next.push(entry.head);
                entry.head = MaxOr::new(refs.push(RefImpl { node: scope_start }));
            }
            Some(Inst::Open) => {
                stack.push(scope_start);
                scope_start = node;
            }
            Some(Inst::Close) => {
                loop {
                    let popped_node = stack.pop().expect("shouldn't run out of elements because we stop as soon as we hit matching open");
                    match inst(nodes.nodes[popped_node]) {
                        Some(Inst::Open) => {
                            scope_start = popped_node;
                            break;
                        }
                        Some(Inst::Def) => {
                            let source = spans.to_span(nodes.tokens[popped_node]).lookup(source);
                            let entry = dedup.entry(source).or_default();
                            if entry.last_scope.into_non_max() == Some(scope_start) {
                                duplicates.push(popped_node);
                                continue;
                            }
                            entry.last_scope = MaxOr::new(scope_start);

                            while let Some(head_ref) = entry.head.into_non_max() {
                                if refs[head_ref].node < scope_start {
                                    break;
                                }
                                refs[head_ref].node = popped_node;
                                entry.head = next[head_ref];
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

    Resolve { refs, duplicates }
}

#[derive(Debug)]
pub struct Resolve {
    pub refs: IndexVec<Ref, RefImpl>,
    pub duplicates: Vec<Node>,
}
