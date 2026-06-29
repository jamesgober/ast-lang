//! Shared test fixture: a tiny expression AST implementing [`Node`], plus helpers
//! for building trees and recording traversals.

#![allow(clippy::unwrap_used, dead_code)]

use ast_lang::{Arena, Id, Node, Span};

/// A minimal three-form expression node: an integer leaf, a unary negation, and a
/// binary addition. Enough to exercise leaves, single children, and ordered pairs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Lit(i64, Span),
    Neg(Id<Expr>, Span),
    Add(Id<Expr>, Id<Expr>, Span),
}

impl Node for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Lit(_, s) | Expr::Neg(_, s) | Expr::Add(_, _, s) => *s,
        }
    }

    fn each_child(&self, f: &mut dyn FnMut(Id<Self>)) {
        match self {
            Expr::Lit(..) => {}
            Expr::Neg(a, _) => f(*a),
            Expr::Add(a, b, _) => {
                f(*a);
                f(*b);
            }
        }
    }

    fn map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self {
        match self {
            Expr::Lit(v, s) => Expr::Lit(*v, *s),
            Expr::Neg(a, s) => Expr::Neg(f(*a), *s),
            Expr::Add(a, b, s) => Expr::Add(f(*a), f(*b), *s),
        }
    }
}

/// A structural recipe for an expression tree, independent of any arena. Built by
/// [`build`] into an arena; used by the property tests to generate random trees.
#[derive(Clone, Debug)]
pub enum Shape {
    Lit,
    Neg(Box<Shape>),
    Add(Box<Shape>, Box<Shape>),
}

impl Shape {
    /// The number of nodes this shape expands to.
    pub fn count(&self) -> usize {
        match self {
            Shape::Lit => 1,
            Shape::Neg(a) => 1 + a.count(),
            Shape::Add(a, b) => 1 + a.count() + b.count(),
        }
    }
}

/// Builds `shape` into `arena`, giving each node a distinct span derived from its
/// allocation order, and returns the root handle. Spans are distinct so a test can
/// confirm a transform preserved them.
pub fn build(shape: &Shape, arena: &mut Arena<Expr>) -> Id<Expr> {
    match shape {
        Shape::Lit => {
            let at = arena.len() as u32;
            arena.alloc(Expr::Lit(at as i64, Span::new(at, at + 1)))
        }
        Shape::Neg(a) => {
            let child = build(a, arena);
            let at = arena.len() as u32;
            arena.alloc(Expr::Neg(child, Span::new(at, at + 1)))
        }
        Shape::Add(a, b) => {
            let left = build(a, arena);
            let right = build(b, arena);
            let at = arena.len() as u32;
            arena.alloc(Expr::Add(left, right, Span::new(at, at + 1)))
        }
    }
}

/// Builds a left-leaning negation chain `depth` deep over a single literal,
/// iteratively, so the builder itself does not recurse. Returns the arena and the
/// root; the tree has `depth + 1` nodes.
pub fn deep_chain(depth: usize) -> (Arena<Expr>, Id<Expr>) {
    let mut arena = Arena::new();
    let mut id = arena.alloc(Expr::Lit(0, Span::new(0, 1)));
    for _ in 0..depth {
        let at = arena.len() as u32;
        id = arena.alloc(Expr::Neg(id, Span::new(at, at + 1)));
    }
    (arena, id)
}
