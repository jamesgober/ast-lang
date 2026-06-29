//! Behavioural tests for the traversal and transform machinery against a known
//! tree, checking exact visit order, pruning, stopping, and structural copying.

#![allow(clippy::unwrap_used)]

mod common;

use ast_lang::{Arena, Flow, Id, Node, Visitor, transform, walk};
use common::{Expr, Shape, build, deep_chain};

/// Records the sequence of enter and leave events as `(tag, span_start)`, so a
/// test can assert the exact traversal order.
#[derive(Default)]
struct Trace {
    events: Vec<(char, u32)>,
}

impl Visitor<Expr> for Trace {
    fn enter(&mut self, _arena: &Arena<Expr>, _id: Id<Expr>, node: &Expr) -> Flow {
        self.events.push(('e', node.span().start().to_u32()));
        Flow::Continue
    }
    fn leave(&mut self, _arena: &Arena<Expr>, _id: Id<Expr>, node: &Expr) {
        self.events.push(('l', node.span().start().to_u32()));
    }
}

/// `(1 + 2)`: two leaves under an `Add`. With spans derived from allocation order,
/// the left leaf has start 0, the right leaf start 1, and the `Add` start 2.
fn one_plus_two() -> (Arena<Expr>, Id<Expr>) {
    let mut arena = Arena::new();
    let shape = Shape::Add(Box::new(Shape::Lit), Box::new(Shape::Lit));
    let root = build(&shape, &mut arena);
    (arena, root)
}

#[test]
fn test_preorder_enters_parent_before_children_in_source_order() {
    let (arena, root) = one_plus_two();
    let mut trace = Trace::default();
    walk(&arena, root, &mut trace);

    let enters: Vec<u32> = trace
        .events
        .iter()
        .filter(|(tag, _)| *tag == 'e')
        .map(|(_, s)| *s)
        .collect();
    // Add (start 2) entered first, then left leaf (0), then right leaf (1).
    assert_eq!(enters, [2, 0, 1]);
}

#[test]
fn test_postorder_leaves_children_before_parent() {
    let (arena, root) = one_plus_two();
    let mut trace = Trace::default();
    walk(&arena, root, &mut trace);

    let leaves: Vec<u32> = trace
        .events
        .iter()
        .filter(|(tag, _)| *tag == 'l')
        .map(|(_, s)| *s)
        .collect();
    // Left leaf (0), right leaf (1), then the Add (2) last.
    assert_eq!(leaves, [0, 1, 2]);
}

#[test]
fn test_full_event_sequence_is_balanced() {
    let (arena, root) = one_plus_two();
    let mut trace = Trace::default();
    walk(&arena, root, &mut trace);
    // Enter Add, enter+leave left, enter+leave right, leave Add.
    assert_eq!(
        trace.events,
        [('e', 2), ('e', 0), ('l', 0), ('e', 1), ('l', 1), ('l', 2)]
    );
}

#[test]
fn test_skip_children_prunes_the_subtree_but_still_leaves() {
    // Skip descending into any `Add`, so its leaves are never entered.
    struct SkipAdds {
        entered: usize,
        left: usize,
    }
    impl Visitor<Expr> for SkipAdds {
        fn enter(&mut self, _: &Arena<Expr>, _: Id<Expr>, node: &Expr) -> Flow {
            self.entered += 1;
            if matches!(node, Expr::Add(..)) {
                Flow::SkipChildren
            } else {
                Flow::Continue
            }
        }
        fn leave(&mut self, _: &Arena<Expr>, _: Id<Expr>, _: &Expr) {
            self.left += 1;
        }
    }

    let (arena, root) = one_plus_two();
    let mut v = SkipAdds {
        entered: 0,
        left: 0,
    };
    walk(&arena, root, &mut v);
    // Only the Add is entered; its two leaves are pruned. It is still left.
    assert_eq!(v.entered, 1);
    assert_eq!(v.left, 1);
}

#[test]
fn test_stop_halts_the_walk_with_no_further_events() {
    // Stop the moment the first leaf is entered.
    struct StopAtLeaf {
        events: usize,
    }
    impl Visitor<Expr> for StopAtLeaf {
        fn enter(&mut self, _: &Arena<Expr>, _: Id<Expr>, node: &Expr) -> Flow {
            self.events += 1;
            if matches!(node, Expr::Lit(..)) {
                Flow::Stop
            } else {
                Flow::Continue
            }
        }
        fn leave(&mut self, _: &Arena<Expr>, _: Id<Expr>, _: &Expr) {
            self.events += 1;
        }
    }

    let (arena, root) = one_plus_two();
    let mut v = StopAtLeaf { events: 0 };
    walk(&arena, root, &mut v);
    // Enter Add, enter left leaf -> Stop. Exactly two events, no leaves.
    assert_eq!(v.events, 2);
}

#[test]
fn test_out_of_range_root_visits_nothing() {
    let arena: Arena<Expr> = Arena::new();
    // A handle from another, larger arena names a slot this empty one lacks.
    let (other, root) = one_plus_two();
    let _ = &other;

    let mut trace = Trace::default();
    walk(&arena, root, &mut trace);
    assert!(trace.events.is_empty());
}

#[test]
fn test_transform_identity_copies_shape_and_spans() {
    let mut src = Arena::new();
    let shape = Shape::Add(
        Box::new(Shape::Neg(Box::new(Shape::Lit))),
        Box::new(Shape::Lit),
    );
    let root = build(&shape, &mut src);

    let mut dst = Arena::new();
    let new_root = transform(&src, root, &mut dst, |node| node).expect("root is live");

    // Same number of nodes, and the same spans walked in the same order.
    assert_eq!(dst.len(), src.len());

    let mut src_spans = Trace::default();
    walk(&src, root, &mut src_spans);
    let mut dst_spans = Trace::default();
    walk(&dst, new_root, &mut dst_spans);
    assert_eq!(src_spans.events, dst_spans.events);
}

#[test]
fn test_transform_invalid_root_is_none() {
    let (src, _root) = one_plus_two();
    let foreign = {
        // A root handle that does not exist in a fresh empty source.
        let empty: Arena<Expr> = Arena::new();
        let _ = &empty;
        // Build a separate arena just to mint an out-of-range handle.
        let (mut big, mut last) = (Arena::new(), None);
        for i in 0..10 {
            last = Some(big.alloc(Expr::Lit(i, ast_lang::Span::new(0, 1))));
        }
        last.unwrap()
    };
    let mut dst = Arena::new();
    // `foreign` indexes beyond `src` (which has only 3 nodes).
    let _ = &src;
    assert_eq!(transform(&src, foreign, &mut dst, |n| n), None);
    assert!(dst.is_empty());
}

#[test]
fn test_deep_tree_walks_and_transforms_without_stack_overflow() {
    // A chain far deeper than any safe recursion depth: proves the traversal and
    // the rebuild are both iterative.
    let (arena, root) = deep_chain(200_000);

    struct Count(usize);
    impl Visitor<Expr> for Count {
        fn enter(&mut self, _: &Arena<Expr>, _: Id<Expr>, _: &Expr) -> Flow {
            self.0 += 1;
            Flow::Continue
        }
    }
    let mut count = Count(0);
    walk(&arena, root, &mut count);
    assert_eq!(count.0, 200_001);

    let mut dst = Arena::new();
    let new_root = transform(&arena, root, &mut dst, |n| n).expect("root is live");
    assert_eq!(dst.len(), 200_001);
    assert!(dst.contains(new_root));
}
