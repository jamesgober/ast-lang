# ast-lang &mdash; API Reference

> Complete reference for every public item in `ast-lang`, with examples.
> **Status: stable as of `1.0.0`.** The surface below is frozen under Semantic Versioning — no breaking changes before `2.0`.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [`Node`](#node)
- [`Visitor`](#visitor) and [`Flow`](#flow)
- [`walk`](#walk)
- [`transform`](#transform)
- [Re-exports](#re-exports)
- [Feature flags](#feature-flags)
- [Frozen surface](#frozen-surface)

---

## Overview

ast-lang is the syntax-tree substrate of the `-lang` family: the trait an AST node
implements and the generic machinery that walks and rewrites a tree of those
nodes. It owns no grammar — a language brings its own node type — so one set of
traversal code serves every language built on it.

The model follows [`arena-lang`](https://docs.rs/arena-lang): a language defines a
single node type `N` (almost always an `enum`), stores its nodes in an `Arena<N>`,
and wires the tree with `Id<N>` handles instead of references. A node implements
[`Node`](#node); everything else — read traversal, rewriting — is built generically
on that trait and is iterative, so even a very deep tree never overflows the call
stack.

---

## Installation

```toml
[dependencies]
ast-lang = "1"
```

---

## `Node`

The contract a language's node type satisfies so the generic traversals can
operate on it. Three operations, nothing more.

```rust
use ast_lang::{Id, Node, Span};

enum Expr {
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
            Expr::Add(a, b, _) => { f(*a); f(*b); }
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
```

| Method | Description |
|--------|-------------|
| `span(&self) -> Span` | The byte range of source the node covers (empty for a synthetic node). |
| `each_child(&self, f: &mut dyn FnMut(Id<Self>))` | Calls `f` with each direct child handle, in source order. Read traversal is built on this. |
| `map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self` | Rebuilds the node with each child handle `c` replaced by `f(c)`, preserving the span and other fields. Rewriting is built on this. |

A leaf node has an empty `each_child` and a plain copy in `map_children`.

---

## `Visitor`

A read-only traversal over a tree of [`Node`](#node)s. Implement it to observe a
tree without changing it; the visitor holds whatever state it accumulates.

| Method | Description |
|--------|-------------|
| `enter(&mut self, arena: &Arena<N>, id: Id<N>, node: &N) -> Flow` | Runs on the way down, before children. Returns a [`Flow`](#flow). Default: `Flow::Continue`. |
| `leave(&mut self, arena: &Arena<N>, id: Id<N>, node: &N)` | Runs on the way back up, after children. Default: no-op. |

`Visitor<N>` is object-safe, so `&mut dyn Visitor<N>` can be passed to [`walk`](#walk).

## `Flow`

What `Visitor::enter` returns to steer the walk.

| Variant | Effect |
|---------|--------|
| `Continue` (default) | Descend into the node's children, then leave it. |
| `SkipChildren` | Do not descend, but still leave the node. |
| `Stop` | Abandon the whole walk immediately; no further `enter` or `leave` runs. |

`Flow` is `Copy` and implements `Default` (= `Continue`).

---

## `walk`

```rust,ignore
pub fn walk<N, V>(arena: &Arena<N>, root: Id<N>, visitor: &mut V)
where
    N: Node,
    V: Visitor<N> + ?Sized;
```

Drives `visitor` over the tree rooted at `root`, depth-first. The traversal is
**iterative** — it keeps its work stack on the heap — so a tree of any depth is
walked without recursing on the call stack. Each node is entered before its
children and left after them; children are visited in `each_child` order. A handle
that names no live node (including an out-of-range `root`) is skipped, never a
panic.

---

## `transform`

```rust,ignore
pub fn transform<N, F>(src: &Arena<N>, root: Id<N>, dst: &mut Arena<N>, f: F) -> Option<Id<N>>
where
    N: Node,
    F: FnMut(N) -> N;
```

Rebuilds the tree rooted at `root` from `src` into `dst`, passing every node
through `f`, and returns the new root handle. Nodes are processed post-order, so
when `f` sees a node its children have already been rebuilt into `dst` and its
child handles remapped. With `f = |node| node` the result is a faithful deep copy.
Also iterative. Returns `None` if `root` names no live node, or if `dst` runs out
of capacity. It assumes a tree (each node reachable once from `root`).

---

## Re-exports

The storage and position types this crate's API is built on are re-exported, so a
downstream need not also name `arena-lang` and `span-lang`:

- `Arena`, `Id` — from [`arena-lang`](https://docs.rs/arena-lang); the node storage and handles.
- `Span` — from [`span-lang`](https://docs.rs/span-lang); the range a node covers.

---

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | yes | Standard library. With it off, the crate is `no_std` (it always needs `alloc`). Forwards to `span-lang/std` and `arena-lang/std`. |

---

## Frozen surface

The items above are the complete public API, frozen as of `1.0.0`. It follows
Semantic Versioning: no breaking change before `2.0`, additions in minors, MSRV
(Rust 1.85) rising only in a minor. Not part of the contract: internal modules and
the exact heap-allocation behaviour of a traversal.

Deliberately left out of 1.0, each addable later without a breaking change:

- **`serde`.** The generic machinery cannot derive serialisation for a language's
  own node type, so the crate carries no `serde` feature; a language serialises its
  own nodes.
- **A bundled tree type.** Walking takes `(&Arena<N>, root)` directly rather than a
  wrapper; a convenience `Ast<N>` newtype can be added later.
- **Mutating / fallible visitors.** `walk` is read-only and `transform` is
  infallible per node; a mutating visit or a `Result`-returning transform can be
  added as new functions.

---

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
