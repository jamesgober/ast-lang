//! # ast_lang
//!
//! The syntax-tree substrate for the `-lang` family: the trait an AST node
//! implements and the generic machinery that walks and rewrites a tree of those
//! nodes. It owns no grammar — a language brings its own node type — so the same
//! traversal code serves every language built on it.
//!
//! ## Model
//!
//! ast-lang builds on the pattern [`arena_lang`] establishes: a language defines a
//! single node type `N` (almost always an `enum`), stores its nodes in an
//! [`Arena<N>`](Arena), and wires the tree with [`Id<N>`](Id) handles rather than
//! references — so a parent can hold its children without tangling the borrow
//! checker. A node implements [`Node`] to report its [`span`](Node::span), enumerate
//! its child handles ([`each_child`](Node::each_child)), and rebuild itself with
//! remapped children ([`map_children`](Node::map_children)). Everything else is
//! generic:
//!
//! - [`walk`] drives a [`Visitor`] over the tree, depth-first and iteratively, so
//!   even a very deep tree never overflows the call stack. The visitor steers the
//!   traversal with a [`Flow`] and accumulates whatever it needs.
//! - [`transform`] rebuilds a tree into a destination arena, passing each node
//!   through a closure — the rewrite (fold) operation, also iterative.
//!
//! ## Quickstart
//!
//! ```
//! use ast_lang::{transform, walk, Arena, Flow, Id, Node, Span, Visitor};
//!
//! // A language defines its node type; ast-lang carries the rest.
//! #[derive(Clone)]
//! enum Expr {
//!     Lit(i64, Span),
//!     Add(Id<Expr>, Id<Expr>, Span),
//! }
//!
//! impl Node for Expr {
//!     fn span(&self) -> Span {
//!         match self {
//!             Expr::Lit(_, s) | Expr::Add(_, _, s) => *s,
//!         }
//!     }
//!     fn each_child(&self, f: &mut dyn FnMut(Id<Self>)) {
//!         if let Expr::Add(a, b, _) = self {
//!             f(*a);
//!             f(*b);
//!         }
//!     }
//!     fn map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self {
//!         match self {
//!             Expr::Lit(v, s) => Expr::Lit(*v, *s),
//!             Expr::Add(a, b, s) => Expr::Add(f(*a), f(*b), *s),
//!         }
//!     }
//! }
//!
//! let mut arena = Arena::new();
//! let one = arena.alloc(Expr::Lit(1, Span::new(0, 1)));
//! let two = arena.alloc(Expr::Lit(2, Span::new(4, 5)));
//! let add = arena.alloc(Expr::Add(one, two, Span::new(0, 5)));
//!
//! // Walk: sum the literals.
//! struct Sum(i64);
//! impl Visitor<Expr> for Sum {
//!     fn enter(&mut self, _: &Arena<Expr>, _: Id<Expr>, node: &Expr) -> Flow {
//!         if let Expr::Lit(v, _) = node {
//!             self.0 += *v;
//!         }
//!         Flow::Continue
//!     }
//! }
//! let mut sum = Sum(0);
//! walk(&arena, add, &mut sum);
//! assert_eq!(sum.0, 3);
//!
//! // Transform: deep-copy into a fresh arena, doubling each literal.
//! let mut doubled = Arena::new();
//! let _ = transform(&arena, add, &mut doubled, |node| match node {
//!     Expr::Lit(v, s) => Expr::Lit(v * 2, s),
//!     other => other,
//! });
//! ```
//!
//! ## Features
//!
//! - `std` (default) — the standard library; without it the crate is `no_std`
//!   (it always needs `alloc`). Forwards to `span-lang/std` and `arena-lang/std`.
//!
//! ## Stability
//!
//! The public surface is being designed across the 0.x series and freezes at
//! `1.0.0`, after which it follows Semantic Versioning: no breaking changes before
//! `2.0`, additions arrive in minor releases, and the MSRV (Rust 1.85) only rises
//! in a minor. The frozen surface is catalogued in
//! [`docs/API.md`](https://github.com/jamesgober/ast-lang/blob/main/docs/API.md).

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

extern crate alloc;

mod node;
mod transform;
mod visit;

pub use node::Node;
pub use transform::transform;
pub use visit::{Flow, Visitor, walk};

// Re-exported so a downstream defining and traversing an AST can name the storage
// and position types this crate's API is built on without also having to depend on
// `arena-lang` and `span-lang` directly.
pub use arena_lang::{Arena, Id};
pub use span_lang::Span;
