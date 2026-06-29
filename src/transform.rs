//! Tree rewriting: the [`transform`] driver.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use arena_lang::{Arena, Id};

use crate::{Node, Visitor, walk};

/// Collects node handles in post-order (children before their parent) by leaving
/// each node into a list as the walk unwinds.
struct PostOrder<N> {
    order: Vec<Id<N>>,
}

impl<N: Node> Visitor<N> for PostOrder<N> {
    fn leave(&mut self, _arena: &Arena<N>, id: Id<N>, _node: &N) {
        self.order.push(id);
    }
}

/// Rebuilds the tree rooted at `root` from `src` into `dst`, passing every node
/// through `f`, and returns the handle of the new root.
///
/// This is the rewrite (fold) operation: a copy of the subtree is built in the
/// destination arena, and `f` gets a chance to replace each node as it is rebuilt.
/// Nodes are processed **post-order** — every child is rebuilt and allocated into
/// `dst` before its parent — so when `f` receives a node, that node's child handles
/// have already been remapped to point at their new selves in `dst`. With `f` set
/// to the identity (`|node| node`), the result is a faithful deep copy: same shape,
/// same span on every node.
///
/// The traversal is the iterative [`walk`], so an arbitrarily deep tree is rebuilt
/// without recursing on the call stack. It assumes a tree — each node reachable
/// from `root` exactly once; a shared sub-node would be copied once per path to it.
///
/// # Returns
///
/// The handle of the rebuilt root in `dst`, or `None` if `root` names no live node
/// in `src`, or if `dst` runs out of capacity while allocating. Either way `dst`
/// is left holding whatever was allocated before the stop, and never panics.
///
/// # Examples
///
/// Deep-copy a tree, doubling every literal on the way:
///
/// ```
/// use ast_lang::{transform, walk, Arena, Flow, Id, Node, Span, Visitor};
///
/// #[derive(Clone)]
/// enum Expr {
///     Lit(i64, Span),
///     Add(Id<Expr>, Id<Expr>, Span),
/// }
///
/// impl Node for Expr {
///     fn span(&self) -> Span {
///         match self {
///             Expr::Lit(_, s) | Expr::Add(_, _, s) => *s,
///         }
///     }
///     fn each_child(&self, f: &mut dyn FnMut(Id<Self>)) {
///         if let Expr::Add(a, b, _) = self {
///             f(*a);
///             f(*b);
///         }
///     }
///     fn map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self {
///         match self {
///             Expr::Lit(v, s) => Expr::Lit(*v, *s),
///             Expr::Add(a, b, s) => Expr::Add(f(*a), f(*b), *s),
///         }
///     }
/// }
///
/// let mut src = Arena::new();
/// let one = src.alloc(Expr::Lit(1, Span::new(0, 1)));
/// let two = src.alloc(Expr::Lit(2, Span::new(4, 5)));
/// let add = src.alloc(Expr::Add(one, two, Span::new(0, 5)));
///
/// let mut dst = Arena::new();
/// let new_root = transform(&src, add, &mut dst, |node| match node {
///     Expr::Lit(v, s) => Expr::Lit(v * 2, s),
///     other => other,
/// })
/// .expect("root is live");
///
/// // The rebuilt tree sums to 2*(1) + 2*(2) = 6.
/// struct Sum(i64);
/// impl Visitor<Expr> for Sum {
///     fn enter(&mut self, _: &Arena<Expr>, _: Id<Expr>, node: &Expr) -> Flow {
///         if let Expr::Lit(v, _) = node {
///             self.0 += *v;
///         }
///         Flow::Continue
///     }
/// }
/// let mut sum = Sum(0);
/// walk(&dst, new_root, &mut sum);
/// assert_eq!(sum.0, 6);
/// ```
pub fn transform<N, F>(src: &Arena<N>, root: Id<N>, dst: &mut Arena<N>, mut f: F) -> Option<Id<N>>
where
    N: Node,
    F: FnMut(N) -> N,
{
    // Visit the source post-order so every child is rebuilt before its parent.
    let mut order = PostOrder { order: Vec::new() };
    walk(src, root, &mut order);

    // Map each source handle to its rebuilt handle in `dst`. Because the order is
    // post-order, a parent's children are always already in the map when it is
    // rebuilt, so `map_children` can substitute their new handles.
    let mut remap: BTreeMap<Id<N>, Id<N>> = BTreeMap::new();
    for id in order.order {
        let Some(node) = src.get(id) else {
            continue;
        };
        let rebuilt = node.map_children(&mut |child| remap.get(&child).copied().unwrap_or(child));
        let new_id = dst.try_alloc(f(rebuilt)).ok()?;
        let _ = remap.insert(id, new_id);
    }

    remap.get(&root).copied()
}

#[cfg(test)]
mod tests {
    use crate::Span;

    use super::*;

    #[derive(Clone, PartialEq, Eq, Debug)]
    enum E {
        Leaf(i64, Span),
        Neg(Id<E>, Span),
    }

    impl Node for E {
        fn span(&self) -> Span {
            match self {
                E::Leaf(_, s) | E::Neg(_, s) => *s,
            }
        }
        fn each_child(&self, f: &mut dyn FnMut(Id<Self>)) {
            if let E::Neg(a, _) = self {
                f(*a);
            }
        }
        fn map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self {
            match self {
                E::Leaf(v, s) => E::Leaf(*v, *s),
                E::Neg(a, s) => E::Neg(f(*a), *s),
            }
        }
    }

    fn neg_leaf() -> (Arena<E>, Id<E>) {
        let mut arena = Arena::new();
        let leaf = arena.alloc(E::Leaf(7, Span::new(0, 1)));
        let neg = arena.alloc(E::Neg(leaf, Span::new(0, 2)));
        (arena, neg)
    }

    #[test]
    fn test_identity_copies_into_destination() {
        let (src, root) = neg_leaf();
        let mut dst = Arena::new();
        let new_root = transform(&src, root, &mut dst, |n| n).expect("live root");
        assert_eq!(dst.len(), 2);
        match dst.get(new_root) {
            Some(E::Neg(child, s)) => {
                assert_eq!(*s, Span::new(0, 2));
                assert_eq!(dst.get(*child), Some(&E::Leaf(7, Span::new(0, 1))));
            }
            other => panic!("expected Neg, got {other:?}"),
        }
    }

    #[test]
    fn test_transform_applies_to_each_node() {
        let (src, root) = neg_leaf();
        let mut dst = Arena::new();
        let new_root = transform(&src, root, &mut dst, |n| match n {
            E::Leaf(v, s) => E::Leaf(v + 1, s),
            other => other,
        })
        .expect("live root");
        // The negated leaf's value was bumped 7 -> 8.
        match dst.get(new_root) {
            Some(E::Neg(child, _)) => {
                assert_eq!(dst.get(*child), Some(&E::Leaf(8, Span::new(0, 1))));
            }
            other => panic!("expected Neg, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_root_returns_none_and_leaves_dst_empty() {
        let (src, _root) = neg_leaf();
        // A handle from a larger arena names a slot `src` lacks.
        let mut big = Arena::new();
        let mut far = big.alloc(E::Leaf(0, Span::new(0, 1)));
        for _ in 0..10 {
            far = big.alloc(E::Leaf(0, Span::new(0, 1)));
        }
        let mut dst = Arena::new();
        assert_eq!(transform(&src, far, &mut dst, |n| n), None);
        assert!(dst.is_empty());
    }
}
