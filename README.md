<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>ast-lang</b>
    <br>
    <sub><sup>AST & TRAVERSAL</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/ast-lang"><img alt="Crates.io" src="https://img.shields.io/crates/v/ast-lang"></a>
    <a href="https://crates.io/crates/ast-lang"><img alt="Downloads" src="https://img.shields.io/crates/d/ast-lang?color=%230099ff"></a>
    <a href="https://docs.rs/ast-lang"><img alt="docs.rs" src="https://img.shields.io/docsrs/ast-lang"></a>
    <a href="https://github.com/jamesgober/ast-lang/actions"><img alt="CI" src="https://github.com/jamesgober/ast-lang/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        ast-lang is the MAIN-tier crate that follows: Node traits and visitor/fold/transform machinery; arena-backed for stable nodes. Part of the -lang language-construction family; see _strategy/LANG_COLLECTION.md for the master plan.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition).
    </p>
    <blockquote>
        <strong>Status: stable.</strong> The public API is frozen as of <code>1.0.0</code> and follows Semantic Versioning &mdash; no breaking changes before <code>2.0</code>. See <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

## Installation

```toml
[dependencies]
ast-lang = "1"
```

<br>

## Example

A language brings its own node type; ast-lang carries the traversal.

```rust
use ast_lang::{transform, walk, Arena, Flow, Id, Node, Span, Visitor};

#[derive(Clone)]
enum Expr {
    Lit(i64, Span),
    Add(Id<Expr>, Id<Expr>, Span),
}

impl Node for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Lit(_, s) | Expr::Add(_, _, s) => *s,
        }
    }
    fn each_child(&self, f: &mut dyn FnMut(Id<Self>)) {
        if let Expr::Add(a, b, _) = self {
            f(*a);
            f(*b);
        }
    }
    fn map_children(&self, f: &mut dyn FnMut(Id<Self>) -> Id<Self>) -> Self {
        match self {
            Expr::Lit(v, s) => Expr::Lit(*v, *s),
            Expr::Add(a, b, s) => Expr::Add(f(*a), f(*b), *s),
        }
    }
}

let mut arena = Arena::new();
let one = arena.alloc(Expr::Lit(1, Span::new(0, 1)));
let two = arena.alloc(Expr::Lit(2, Span::new(4, 5)));
let add = arena.alloc(Expr::Add(one, two, Span::new(0, 5)));

// Walk read-only; transform rebuilds into a fresh arena.
let mut doubled = Arena::new();
let _ = transform(&arena, add, &mut doubled, |node| match node {
    Expr::Lit(v, s) => Expr::Lit(v * 2, s),
    other => other,
});
```

<br>

## Status

This is <code>v1.0.0</code>: the public API is stable and frozen under SemVer. The node trait and the iterative (stack-safe) <code>walk</code> / <code>transform</code> machinery are complete and catalogued in <a href="./docs/API.md"><code>docs/API.md</code></a>.

<hr>
<br>

## Contributing

See <a href="./dev/DIRECTIVES.md"><code>dev/DIRECTIVES.md</code></a> for engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>James Gober <me@jamesgober.com>.</strong></sup>
</div>
