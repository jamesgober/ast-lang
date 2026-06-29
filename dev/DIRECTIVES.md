# ast-lang &mdash; Engineering Directives

> Engineering standards and the definition of done for this project. Read alongside `REPS.md` (root, authoritative) and `dev/ROADMAP.md` (current phase). If anything here conflicts with `REPS.md`, `REPS.md` wins.

---

## 0. Philosophy

This library is built and maintained to a production standard and treated as a flagship piece of work. Plan the full path, then build one verified step at a time. "Good enough" is treated as a defect. ast-lang is a foundation crate of the `-lang` family: a parser builds through it and every later pass — name resolution, type checking, lowering, lints — walks the trees it defines, so its traversal must be correct, total, and cheap on the first try.

---

## 1. What this is

ast-lang is the syntax-tree substrate: the trait every AST node implements and the visitor / transform machinery that walks and rebuilds a tree of those nodes. It is generic over the language — it owns no concrete grammar. A language defines its own node type (typically one `enum`), stores nodes in an [`arena-lang`] `Arena`, and wires the tree with `Id` handles; ast-lang supplies the [`Node`] contract those nodes satisfy and the language-agnostic traversals over them. It owns node traversal and transformation only — no parsing, no allocation policy of its own (that is arena-lang), no diagnostics.

The model is the one [`arena-lang`] documents: a homogeneous node type `N` in an `Arena<N>`, children referenced by `Id<N>`. A node reports its span, enumerates its child handles, and rebuilds itself with remapped child handles; everything else (read traversal, rewriting) is built generically on those three operations.

---

## 2. Engineering law (non-negotiable)

- **Performance** — traversal is allocation-light: a walk reuses one child buffer rather than allocating per node, and resolving a handle is a direct arena slot lookup. No "faster" claim without `criterion` numbers.
- **Correctness** — a walk visits every node of a tree exactly once; pre-order enters a parent before its children, post-order leaves children before their parent; an identity transform reproduces the tree's shape and spans exactly. The invariants in section 4 are property-tested against a naive reference.
- **Robustness** — traversal is iterative, not recursive, so a pathologically deep tree is bounded by heap, not the call stack: deep input is a defined outcome, never a stack overflow. A handle that names no live node resolves to nothing rather than panicking.
- **Architecture** — SOLID, KISS, YAGNI; one responsibility; the node contract is the only thing a language implements, and the machinery depends on that abstraction, not on any concrete grammar.
- **Cross-platform** — Linux/macOS/Windows first-class; no platform-specific code.
- **Error handling** — building into a full arena surfaces as an `Option`/`Result` from the arena, never a panic in library code.
- **Production-ready** — `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` from the first commit; no stray `println!`/`dbg!`; every public item has rustdoc with a runnable example.

---

## 3. Definition of done

1. Compiles clean on Linux/macOS/Windows, stable and MSRV 1.85.
2. `fmt`, `clippy -D warnings`, `test --all-features`, `cargo doc -D warnings` clean.
3. `cargo audit` + `cargo deny check` pass.
4. No `unwrap`/`expect`/`todo!`/`dbg!` in shipping code.
5. A Tier-1 API exists and headlines the docs.
6. Property tests cover the section-4 traversal invariants.
7. Traversal/transform changes carry benchmarks; no regression over 5%.
8. Docs and `CHANGELOG.md` updated; the matching `docs/release/vX.Y.Z.md` written before the tag.

---

## 4. Project-specific invariants

- A walk over a tree visits every reachable node exactly once.
- Pre-order: a node is entered before any of its children. Post-order: every child is left before its parent is left. Children are visited in the source order the node yields them.
- The walk is iterative; a tree of any depth is traversed without recursion on the call stack.
- A visitor may skip a node's children or stop the whole walk; once it stops, no further node is entered or left.
- An identity transform produces a new tree, in the destination arena, with the same shape and the same span on every node as the source.
- Resolving a handle that names no live node, or transforming from an empty/out-of-range root, is a defined `Option` outcome — never a panic.
