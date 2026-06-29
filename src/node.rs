//! The trait every AST node implements.

use arena_lang::Id;
use span_lang::Span;

/// The contract a language's syntax-tree node satisfies so the generic traversals
/// in this crate can walk and rebuild it.
///
/// ast-lang owns no grammar: a language defines its own node type — almost always
/// a single `enum` with a variant per syntactic form — stores its nodes in an
/// [`Arena`](arena_lang::Arena), and wires the tree with [`Id`] handles. Implementing
/// `Node` for that type is what lets [`walk`](crate::walk) and
/// [`transform`](crate::transform) operate on it without knowing what the variants
/// are.
///
/// The contract is three operations, and nothing more:
///
/// - [`span`](Node::span) — the byte range of source this node covers.
/// - [`each_child`](Node::each_child) — report the node's direct child handles, in
///   source order. Read traversal is built on this.
/// - [`map_children`](Node::map_children) — rebuild the node with each child handle
///   replaced by another, preserving everything else. Rewriting is built on this.
///
/// The two traversal operations are kept separate so a leaf node — one with no
/// children — is an empty `each_child` and a plain clone in `map_children`.
///
/// # Examples
///
/// A minimal expression tree. The node carries its span and its child handles; the
/// three methods just thread those handles through.
///
/// ```
/// use ast_lang::{Id, Node, Span};
///
/// enum Expr {
///     Lit(i64, Span),
///     Neg(Id<Expr>, Span),
///     Add(Id<Expr>, Id<Expr>, Span),
/// }
///
/// impl Node for Expr {
///     fn span(&self) -> Span {
///         match self {
///             Expr::Lit(_, s) | Expr::Neg(_, s) | Expr::Add(_, _, s) => *s,
///         }
///     }
///
///     fn each_child(&self, f: &mut dyn FnMut(Id<Self>)) {
///         match self {
///             Expr::Lit(..) => {}
///             Expr::Neg(a, _) => f(*a),
///             Expr::Add(a, b, _) => {
///                 f(*a);
///                 f(*b);
///             }
///         }
///     }
///
///     fn map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self {
///         match self {
///             Expr::Lit(v, s) => Expr::Lit(*v, *s),
///             Expr::Neg(a, s) => Expr::Neg(f(*a), *s),
///             Expr::Add(a, b, s) => Expr::Add(f(*a), f(*b), *s),
///         }
///     }
/// }
/// ```
pub trait Node: Sized {
    /// Returns the byte range of source this node covers.
    ///
    /// A synthetic node with no source of its own returns an empty span (for
    /// example [`Span::empty`](span_lang::Span::empty) at the insertion point);
    /// every node has a span so a later pass can always point a diagnostic at it.
    fn span(&self) -> Span;

    /// Calls `f` once with each direct child handle, in source order.
    ///
    /// "Direct" means one level down — a child reports its own children when it is
    /// visited in turn. A leaf node calls `f` zero times. The order `f` is called
    /// in is the order the children appear in the source, which is the order
    /// [`walk`](crate::walk) visits them.
    fn each_child(&self, f: &mut dyn FnMut(Id<Self>));

    /// Rebuilds this node with each child handle `c` replaced by `f(c)`,
    /// preserving the span and every non-child field.
    ///
    /// This is the one language-specific step of a [`transform`](crate::transform):
    /// the machinery supplies an `f` that maps each old child handle to its rebuilt
    /// counterpart, and this method threads those new handles back into a fresh
    /// node. A leaf node ignores `f` and returns a copy of itself.
    fn map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self;
}
