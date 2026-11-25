use ouro_index_vec::{IndexVec, MaxOr};
use ouro_parse_types::{Node, NodeKind, Nodes};
use ouro_tokenize_types::ByteSpans;
use std::collections::HashMap;

ouro_index_vec::define_index_type! {
    pub struct Ref = u32;
}

ouro_index_vec::define_index_type! {
    pub struct Item = u32;
}

#[derive(Copy, Clone, Debug)]
pub enum RefTarget {
    Unresolved { scope_start: Node, depth: u32 },
    Item { def_node: Node, item: Item },
    Local { def_node: Node, de_bruijn: u32 },
}

impl RefTarget {
    pub fn def_node(self) -> Option<Node> {
        match self {
            RefTarget::Item { def_node, .. } | RefTarget::Local { def_node, .. } => Some(def_node),
            RefTarget::Unresolved { .. } => None,
        }
    }
}

#[derive(Debug)]
pub struct RefImpl {
    pub target: RefTarget,
    pub next: MaxOr<Ref>,
}

enum StackEntry {
    Open {
        prev_scope_start: Node,
    },
    Item {
        def_node: Node,
        item: Item,
    },
    Local {
        prev_scope_start: Node,
        def_node: Node,
    },
}

#[derive(Default)]
struct DedupEntry {
    head: MaxOr<Ref>,
    last_scope: MaxOr<Node>,
}

pub fn resolve(nodes: &Nodes, spans: &ByteSpans, source: &str) -> Resolve {
    let mut scope_start = Node::from_raw(0);
    let mut local_depth = 0;
    let mut stack = Vec::new();
    let mut dedup: HashMap<&str, DedupEntry> = HashMap::new();
    let mut out = Resolve::default();

    for (node, &node_kind) in nodes.nodes.iter_enumerated() {
        match node_kind {
            NodeKind::StructIdent | NodeKind::FnIdent | NodeKind::ConstIdent => {
                stack.push(StackEntry::Item {
                    def_node: node,
                    item: out.items.push(node),
                });
            }
            NodeKind::FnParamIdent | NodeKind::LetIdent => {
                stack.push(StackEntry::Local {
                    prev_scope_start: scope_start,
                    def_node: node,
                });
                scope_start = node;
                local_depth += 1;
            }
            NodeKind::Ident => {
                let entry = dedup
                    .entry(spans.to_span(nodes.tokens[node]).lookup(source))
                    .or_default();
                entry.head = MaxOr::new(out.refs.push(RefImpl {
                    target: RefTarget::Unresolved {
                        scope_start,
                        depth: local_depth,
                    },
                    next: entry.head,
                }));
            }
            NodeKind::FileBegin
            | NodeKind::StructBodyBegin
            | NodeKind::FnParams
            | NodeKind::Block => {
                stack.push(StackEntry::Open {
                    prev_scope_start: scope_start,
                });
                scope_start = node;
            }
            NodeKind::FileEnd
            | NodeKind::StructBodyEnd
            | NodeKind::FnBodyEnd
            | NodeKind::BlockEnd => loop {
                let (prev_scope_start, def_node, item) = match stack.pop().expect(
                    "shouldn't run out of elements because we stop as soon as we hit matching open",
                ) {
                    StackEntry::Open { prev_scope_start } => {
                        scope_start = prev_scope_start;
                        break;
                    }
                    StackEntry::Item { def_node, item } => (scope_start, def_node, Some(item)),
                    StackEntry::Local {
                        prev_scope_start,
                        def_node,
                    } => (prev_scope_start, def_node, None),
                };

                let name = spans.to_span(nodes.tokens[def_node]).lookup(source);
                let entry = dedup.entry(name).or_default();

                if item.is_some() {
                    if entry.last_scope.into_non_max() == Some(scope_start) {
                        out.duplicates.push(def_node);
                        continue;
                    }
                    entry.last_scope = MaxOr::new(scope_start);
                }

                let first_ref = entry.head;
                let mut last_ref = MaxOr::max();
                while let Some(head_ref) = entry.head.into_non_max() {
                    let RefTarget::Unresolved {
                        scope_start: ref_scope_start,
                        depth: ref_depth,
                    } = out.refs[head_ref].target
                    else {
                        unreachable!("pending list only contains unresolved refs");
                    };
                    if ref_scope_start < scope_start {
                        break;
                    }
                    out.refs[head_ref].target = match item {
                        Some(item) => RefTarget::Item { def_node, item },
                        None => RefTarget::Local {
                            def_node,
                            de_bruijn: ref_depth - local_depth,
                        },
                    };
                    entry.head = out.refs[head_ref].next;
                    last_ref = MaxOr::new(head_ref);
                }
                if let Some(last_ref) = last_ref.into_non_max() {
                    out.refs[last_ref].next = first_ref;
                }
                scope_start = prev_scope_start;
                if item.is_none() {
                    local_depth -= 1;
                }
            },
            _ => {}
        }
    }

    out
}

#[derive(Debug, Default)]
pub struct Resolve {
    pub refs: IndexVec<Ref, RefImpl>,
    pub items: IndexVec<Item, Node>,
    pub duplicates: Vec<Node>,
}
