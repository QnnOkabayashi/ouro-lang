use crate::node::{Node, NodeImpl, SubtreeSize};
use index_vec::IndexSlice;

#[derive(Copy, Clone)]
enum Char {
    Introducer,
    Leaf,
}

impl Char {
    fn character(self) -> char {
        match self {
            Char::Introducer => '╭',
            Char::Leaf => '├',
        }
    }
}

pub fn pprint(
    nodes: &IndexSlice<Node, [NodeImpl]>,
    mut print_node: impl FnMut(Node, &mut String),
) -> String {
    // Go through nodes backwards and fill this up, then when we print the nodes with
    // a forward pass its indentation will be at the top of the stack :)
    let mut indentation = Vec::with_capacity(nodes.len());

    // The top one says how many we have to go through until we can dedent again.
    // The length is how many to indent.
    let mut remaining_stack = Vec::new();
    for node_impl in nodes.iter().rev() {
        while let Some(&0) = remaining_stack.last() {
            remaining_stack.pop();
        }

        let mut ch = Char::Leaf;

        // remaining_stack will be empty if this is a top level node.
        if let Some(remaining_at_this_level) = remaining_stack.last_mut() {
            *remaining_at_this_level -= 1;
            if *remaining_at_this_level == 0 {
                ch = Char::Introducer;
            }
        }

        indentation.push((remaining_stack.len(), ch));

        if let Some(SubtreeSize(subtree_size)) = node_impl.kind.subtree_size() {
            remaining_stack.push(subtree_size);
        }
    }

    let mut out = String::new();
    for (node, _) in nodes.iter_enumerated() {
        let (indent, ch) = indentation.pop().unwrap();
        for _ in 0..indent {
            out.push_str("│ ");
        }
        out.push(ch.character());
        out.push('─');
        // out.push('▶');
        print_node(node, &mut out);
        out.push('\n');
    }
    out
}
