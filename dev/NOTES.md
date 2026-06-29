# ast-lang — Engineering Notes

Decisions taken while building ast-lang to 1.0, recorded for review. Lives
alongside `DIRECTIVES.md` and `ROADMAP.md` in `dev/`.

---

## Design decisions

### Homogeneous node model (one `enum N` in one `Arena<N>`)

ast-lang is generic over a *single* node type `N` stored in one
[`arena-lang`] `Arena<N>`, with children referenced by `Id<N>`. This is the pattern
arena-lang's own documentation establishes (`enum Expr { Int(i64), Add(Id<Expr>,
Id<Expr>) }`), and it is what makes the traversals language-agnostic: the machinery
needs only "a node yields child handles" and "a node rebuilds with remapped child
handles," never the variant set.

The alternative — heterogeneous nodes across several typed arenas (`Arena<Expr>`,
`Arena<Stmt>`, …) — was rejected: arena-lang is single-type-per-arena, and a
generic walk across multiple arenas of unknown types cannot be expressed without a
much heavier abstraction. A language with several node categories folds them into
one `enum` (the common compiler pattern). This is the one architectural fork; it is
not hard to reverse for a *consumer* (they can still use multiple arenas manually),
but the generic machinery commits to the single-`N` model.

### Deviation from the master plan's dependency list

The master plan lists ast-lang's deps as `span, arena, intern`. The generic
machinery uses **`span`** (every node reports a `Span`) and **`arena`** (storage and
handles), but **not `intern`**: interned symbols are *node data* a language carries,
not something the traversal machinery touches. Per the build directive's "wire a
dependency only when code actually uses it," `intern-lang` was not wired. A language
building an AST still uses `intern-lang` for its identifiers directly; ast-lang
simply does not need to name it. If a future generic helper needs `Symbol`, adding
the dependency is a non-breaking minor.

### No `serde` feature

The scaffold reserved a no-op `serde` feature. It was dropped rather than frozen
unused: the generic machinery cannot derive serialisation for a language's own node
type, so there is nothing for ast-lang itself to serialise. A language serialises
its own nodes. A `serde` feature can return post-1.0 as a non-breaking minor.

### Iterative traversal

Both `walk` and `transform` keep an explicit work stack on the heap rather than
recursing, so a deep tree is bounded by heap, not the call stack — a correctness and
robustness requirement, not an optimisation. The test suite walks and rebuilds a
200,000-node chain to hold this.

---

## STATUS — 1.0.0, frozen

ast-lang is built from scaffold through the v0.2.0 core to the v1.0.0 API freeze.
Engineering is complete and green; the commit, tag (`v1.0.0`), push, and
`cargo publish` are left for you, as requested.

**Done:**

- v0.2.0 core — `Node` trait (`span`/`each_child`/`map_children`), `Visitor` + `Flow`,
  iterative `walk`, iterative post-order `transform`. `span-lang` and `arena-lang`
  wired and used; `Arena`/`Id`/`Span` re-exported.
- v1.0.0 freeze — `docs/API.md` documents the frozen surface and the SemVer promise.

**Quality gates, all green** (Windows local via the rust-lld linker workaround; CI
runs the same on Linux/macOS/Windows, stable + 1.85): `fmt --check`, `clippy
-D warnings` on default / all-features / no-default-features, `test` on default and
all-features, `doc -D warnings`, no-default-features (`no_std`) build, `+1.85`
build, `publish --dry-run`, `deny check`. `#![forbid(unsafe_code)]`, no
`unwrap`/`expect`/`todo!` in shipping code. Counts: 7 unit + 9 behavioural + 5
property + 6 doctests.

**For your review before tagging:** the two deviations above (no `intern`
dependency; single-`N` homogeneous model). Both are defensible and documented; flag
either if it disagrees with the master plan's intent for ast-lang.
