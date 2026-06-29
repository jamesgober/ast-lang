//! Read-only traversal: the [`Visitor`] trait and the [`walk`] driver.

use alloc::vec::Vec;

use arena_lang::{Arena, Id};

use crate::Node;

/// What a [`Visitor`] tells [`walk`] to do after entering a node.
///
/// Returned from [`Visitor::enter`] to steer the traversal:
///
/// - [`Continue`](Flow::Continue) — descend into this node's children, then leave it.
/// - [`SkipChildren`](Flow::SkipChildren) — do not descend, but still leave this node.
/// - [`Stop`](Flow::Stop) — abandon the whole walk now; no further node is entered
///   or left.
///
/// # Examples
///
/// ```
/// use ast_lang::Flow;
///
/// // The default for a visitor that wants the full tree.
/// assert_eq!(Flow::default(), Flow::Continue);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Flow {
    /// Descend into the node's children, then call [`Visitor::leave`] for it.
    #[default]
    Continue,
    /// Skip the node's children but still call [`Visitor::leave`] for it.
    SkipChildren,
    /// Stop the entire walk immediately; no further `enter` or `leave` runs.
    Stop,
}

/// A read-only traversal over a tree of [`Node`]s.
///
/// Implement `Visitor` to observe a tree without changing it — collecting spans,
/// counting forms, building a symbol table, accumulating diagnostics. The visitor
/// holds whatever state it accumulates (`&mut self` is threaded through every
/// callback), and [`walk`] drives it over the tree.
///
/// Both methods have defaults — [`enter`](Visitor::enter) continues, [`leave`](Visitor::leave)
/// does nothing — so a visitor overrides only the one it needs. `enter` runs on the
/// way down and returns a [`Flow`] to steer the walk; `leave` runs on the way back
/// up, after the node's children have been fully visited.
///
/// # Examples
///
/// A visitor that sums the integer literals in an expression tree.
///
/// ```
/// use ast_lang::{walk, Arena, Flow, Id, Node, Span, Visitor};
///
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
/// #[derive(Default)]
/// struct Sum(i64);
/// impl Visitor<Expr> for Sum {
///     fn enter(&mut self, _arena: &Arena<Expr>, _id: Id<Expr>, node: &Expr) -> Flow {
///         if let Expr::Lit(v, _) = node {
///             self.0 += *v;
///         }
///         Flow::Continue
///     }
/// }
///
/// let mut arena = Arena::new();
/// let one = arena.alloc(Expr::Lit(1, Span::new(0, 1)));
/// let two = arena.alloc(Expr::Lit(2, Span::new(4, 5)));
/// let add = arena.alloc(Expr::Add(one, two, Span::new(0, 5)));
///
/// let mut sum = Sum::default();
/// walk(&arena, add, &mut sum);
/// assert_eq!(sum.0, 3);
/// ```
pub trait Visitor<N: Node> {
    /// Called when the walk reaches a node, before its children.
    ///
    /// Return a [`Flow`] to steer the traversal: [`Continue`](Flow::Continue) to
    /// descend, [`SkipChildren`](Flow::SkipChildren) to prune the subtree,
    /// [`Stop`](Flow::Stop) to end the walk. The default descends.
    fn enter(&mut self, arena: &Arena<N>, id: Id<N>, node: &N) -> Flow {
        let _ = (arena, id, node);
        Flow::Continue
    }

    /// Called after a node's children have been fully visited, on the way back up.
    ///
    /// Runs for every node that was entered and not stopped at, including one whose
    /// children were skipped. The default does nothing.
    fn leave(&mut self, arena: &Arena<N>, id: Id<N>, node: &N) {
        let _ = (arena, id, node);
    }
}

/// One pending unit of work for the iterative walk: a node still to enter, or a
/// node whose children are done and which is ready to leave.
enum Step<N> {
    Enter(Id<N>),
    Leave(Id<N>),
}

/// Walks the tree rooted at `root`, driving `visitor` over every node.
///
/// The traversal is depth-first and **iterative** — it keeps its own work stack on
/// the heap rather than recursing — so a tree of any depth is walked without
/// risking a call-stack overflow. Each node is entered before its children
/// ([`Visitor::enter`]) and left after them ([`Visitor::leave`]); children are
/// visited in the order [`Node::each_child`] yields them. The visitor can prune or
/// stop the walk through the [`Flow`] it returns from `enter`.
///
/// A handle that names no live node in `arena` — including a `root` that is out of
/// range — is silently skipped, so a stale handle is a no-op, never a panic.
///
/// # Examples
///
/// ```
/// use ast_lang::{walk, Arena, Flow, Id, Node, Span, Visitor};
///
/// # enum Expr { Lit(i64, Span), Neg(Id<Expr>, Span) }
/// # impl Node for Expr {
/// #     fn span(&self) -> Span { match self { Expr::Lit(_, s) | Expr::Neg(_, s) => *s } }
/// #     fn each_child(&self, f: &mut dyn FnMut(Id<Self>)) { if let Expr::Neg(a, _) = self { f(*a) } }
/// #     fn map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self {
/// #         match self { Expr::Lit(v, s) => Expr::Lit(*v, *s), Expr::Neg(a, s) => Expr::Neg(f(*a), *s) }
/// #     }
/// # }
/// // Count how many nodes the walk visits.
/// struct Count(usize);
/// impl Visitor<Expr> for Count {
///     fn enter(&mut self, _: &Arena<Expr>, _: Id<Expr>, _: &Expr) -> Flow {
///         self.0 += 1;
///         Flow::Continue
///     }
/// }
///
/// let mut arena = Arena::new();
/// let lit = arena.alloc(Expr::Lit(7, Span::new(1, 2)));
/// let neg = arena.alloc(Expr::Neg(lit, Span::new(0, 2)));
///
/// let mut count = Count(0);
/// walk(&arena, neg, &mut count);
/// assert_eq!(count.0, 2); // Neg and its Lit child
/// ```
pub fn walk<N, V>(arena: &Arena<N>, root: Id<N>, visitor: &mut V)
where
    N: Node,
    V: Visitor<N> + ?Sized,
{
    let mut stack: Vec<Step<N>> = Vec::new();
    stack.push(Step::Enter(root));
    // One reusable buffer for a node's children, refilled per node instead of
    // allocating a fresh vector each time.
    let mut scratch: Vec<Id<N>> = Vec::new();

    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(id) => {
                let Some(node) = arena.get(id) else {
                    continue;
                };
                match visitor.enter(arena, id, node) {
                    Flow::Stop => return,
                    Flow::SkipChildren => visitor.leave(arena, id, node),
                    Flow::Continue => {
                        stack.push(Step::Leave(id));
                        scratch.clear();
                        node.each_child(&mut |child| scratch.push(child));
                        // Pop the children back off in reverse so the main stack
                        // yields them in source order.
                        while let Some(child) = scratch.pop() {
                            stack.push(Step::Enter(child));
                        }
                    }
                }
            }
            Step::Leave(id) => {
                if let Some(node) = arena.get(id) {
                    visitor.leave(arena, id, node);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Span;

    use super::*;

    /// A leaf or a pair, enough to test ordering and pruning.
    enum E {
        Leaf(u32, Span),
        Pair(Id<E>, Id<E>, Span),
    }

    impl Node for E {
        fn span(&self) -> Span {
            match self {
                E::Leaf(_, s) | E::Pair(_, _, s) => *s,
            }
        }
        fn each_child(&self, f: &mut dyn FnMut(Id<Self>)) {
            if let E::Pair(a, b, _) = self {
                f(*a);
                f(*b);
            }
        }
        fn map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self {
            match self {
                E::Leaf(v, s) => E::Leaf(*v, *s),
                E::Pair(a, b, s) => E::Pair(f(*a), f(*b), *s),
            }
        }
    }

    #[derive(Default)]
    struct Tags(Vec<u32>);
    impl Visitor<E> for Tags {
        fn enter(&mut self, _: &Arena<E>, _: Id<E>, node: &E) -> Flow {
            if let E::Leaf(v, _) = node {
                self.0.push(*v);
            }
            Flow::Continue
        }
    }

    fn pair_tree() -> (Arena<E>, Id<E>) {
        let mut arena = Arena::new();
        let a = arena.alloc(E::Leaf(1, Span::new(0, 1)));
        let b = arena.alloc(E::Leaf(2, Span::new(1, 2)));
        let root = arena.alloc(E::Pair(a, b, Span::new(0, 2)));
        (arena, root)
    }

    #[test]
    fn test_flow_default_is_continue() {
        assert_eq!(Flow::default(), Flow::Continue);
    }

    #[test]
    fn test_walk_enters_leaves_left_to_right() {
        let (arena, root) = pair_tree();
        let mut tags = Tags::default();
        walk(&arena, root, &mut tags);
        assert_eq!(tags.0, [1, 2]);
    }

    #[test]
    fn test_walk_on_missing_root_is_a_noop() {
        let arena: Arena<E> = Arena::new();
        let (other, root) = pair_tree();
        let _ = &other;
        let mut tags = Tags::default();
        walk(&arena, root, &mut tags);
        assert!(tags.0.is_empty());
    }

    #[test]
    fn test_skip_children_prunes_descendants() {
        struct SkipRoot {
            leaves: u32,
        }
        impl Visitor<E> for SkipRoot {
            fn enter(&mut self, _: &Arena<E>, _: Id<E>, node: &E) -> Flow {
                match node {
                    E::Pair(..) => Flow::SkipChildren,
                    E::Leaf(..) => {
                        self.leaves += 1;
                        Flow::Continue
                    }
                }
            }
        }
        let (arena, root) = pair_tree();
        let mut v = SkipRoot { leaves: 0 };
        walk(&arena, root, &mut v);
        assert_eq!(v.leaves, 0); // the pair's leaves were pruned
    }
}
