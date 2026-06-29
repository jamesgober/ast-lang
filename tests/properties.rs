//! Property tests for the traversal invariants, checked against the tree's own
//! structure over a wide space of randomly generated trees.

#![allow(clippy::unwrap_used)]

mod common;

use std::collections::{BTreeMap, BTreeSet};

use ast_lang::{Arena, Flow, Id, Node, Visitor, transform, walk};
use common::{Expr, Shape, build};
use proptest::prelude::*;

/// Records the order nodes are entered and left, by handle.
#[derive(Default)]
struct Rec {
    enters: Vec<Id<Expr>>,
    leaves: Vec<Id<Expr>>,
}

impl Visitor<Expr> for Rec {
    fn enter(&mut self, _: &Arena<Expr>, id: Id<Expr>, _: &Expr) -> Flow {
        self.enters.push(id);
        Flow::Continue
    }
    fn leave(&mut self, _: &Arena<Expr>, id: Id<Expr>, _: &Expr) {
        self.leaves.push(id);
    }
}

/// A strategy producing arbitrary expression-tree shapes, up to a bounded depth.
fn shape() -> impl Strategy<Value = Shape> {
    let leaf = Just(Shape::Lit);
    leaf.prop_recursive(8, 256, 2, |inner| {
        prop_oneof![
            inner.clone().prop_map(|s| Shape::Neg(Box::new(s))),
            (inner.clone(), inner).prop_map(|(a, b)| Shape::Add(Box::new(a), Box::new(b))),
        ]
    })
}

/// The direct children of a node, collected into a vector.
fn children_of(node: &Expr) -> Vec<Id<Expr>> {
    let mut kids = Vec::new();
    node.each_child(&mut |c| kids.push(c));
    kids
}

proptest! {
    /// A walk enters every node of the tree exactly once, and leaves each exactly
    /// once — the enter and leave counts both equal the node count.
    #[test]
    fn walk_visits_every_node_exactly_once(shape in shape()) {
        let mut arena = Arena::new();
        let root = build(&shape, &mut arena);

        let mut rec = Rec::default();
        walk(&arena, root, &mut rec);

        prop_assert_eq!(rec.enters.len(), arena.len());
        prop_assert_eq!(rec.leaves.len(), arena.len());

        let unique: BTreeSet<_> = rec.enters.iter().copied().collect();
        prop_assert_eq!(unique.len(), rec.enters.len());
    }

    /// Pre-order: a parent is entered before each of its children. Post-order: each
    /// child is left before its parent.
    #[test]
    fn parents_bracket_their_children(shape in shape()) {
        let mut arena = Arena::new();
        let root = build(&shape, &mut arena);

        let mut rec = Rec::default();
        walk(&arena, root, &mut rec);

        let enter_pos: BTreeMap<Id<Expr>, usize> =
            rec.enters.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let leave_pos: BTreeMap<Id<Expr>, usize> =
            rec.leaves.iter().enumerate().map(|(i, id)| (*id, i)).collect();

        for (id, node) in arena.iter() {
            for child in children_of(node) {
                prop_assert!(enter_pos[&id] < enter_pos[&child]);
                prop_assert!(leave_pos[&child] < leave_pos[&id]);
            }
        }
    }

    /// Children are entered in the source order the node yields them.
    #[test]
    fn children_are_entered_in_source_order(shape in shape()) {
        let mut arena = Arena::new();
        let root = build(&shape, &mut arena);

        let mut rec = Rec::default();
        walk(&arena, root, &mut rec);
        let enter_pos: BTreeMap<Id<Expr>, usize> =
            rec.enters.iter().enumerate().map(|(i, id)| (*id, i)).collect();

        for (_, node) in arena.iter() {
            let kids = children_of(node);
            for pair in kids.windows(2) {
                // An earlier child (in source order) is entered before a later one.
                prop_assert!(enter_pos[&pair[0]] < enter_pos[&pair[1]]);
            }
        }
    }

    /// An identity transform produces a tree with the same node count and the same
    /// multiset of spans as the source.
    #[test]
    fn identity_transform_preserves_count_and_spans(shape in shape()) {
        let mut src = Arena::new();
        let root = build(&shape, &mut src);

        let mut dst = Arena::new();
        let new_root = transform(&src, root, &mut dst, |node| node).unwrap();

        prop_assert_eq!(dst.len(), src.len());
        prop_assert!(dst.contains(new_root));

        let mut src_spans: Vec<_> = src.iter().map(|(_, n)| n.span()).collect();
        let mut dst_spans: Vec<_> = dst.iter().map(|(_, n)| n.span()).collect();
        src_spans.sort();
        dst_spans.sort();
        prop_assert_eq!(src_spans, dst_spans);
    }

    /// A transform walks the rebuilt tree to the same shape: entering the copy
    /// yields the same span sequence as entering the original.
    #[test]
    fn identity_transform_preserves_traversal_order(shape in shape()) {
        let mut src = Arena::new();
        let root = build(&shape, &mut src);
        let mut dst = Arena::new();
        let new_root = transform(&src, root, &mut dst, |node| node).unwrap();

        let order = |arena: &Arena<Expr>, root| {
            let mut rec = Rec::default();
            walk(arena, root, &mut rec);
            rec.enters
                .iter()
                .map(|id| arena.get(*id).unwrap().span())
                .collect::<Vec<_>>()
        };
        prop_assert_eq!(order(&src, root), order(&dst, new_root));
    }
}
