<h1 align="center">
    <img width="90px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <br><b>CHANGELOG</b>
</h1>
<p>
  All notable changes to <code>ast-lang</code> will be documented in this file. The format is based on <a href="https://keepachangelog.com/en/1.1.0/">Keep a Changelog</a>,
  and this project adheres to <a href="https://semver.org/spec/v2.0.0.html/">Semantic Versioning</a>.
</p>

---

## [Unreleased]

### Added

### Changed

### Fixed

### Security

---

## [1.0.0] - 2026-06-28

API freeze. The node trait and traversal surface is stable and will not change in a
breaking way before `2.0`. No functional API changes from `0.2.0`.

### Changed

- The public API is declared stable under Semantic Versioning; `docs/API.md`
  catalogues the frozen surface and the SemVer promise.

---

## [0.2.0] - 2026-06-28

The core: the node trait and the visitor / transform machinery. `span-lang` and
`arena-lang` are wired and used.

### Added

- `Node` trait — `span`, `each_child`, and `map_children`: the three operations a language's node type implements.
- `Visitor` trait (`enter`/`leave`) and the `Flow` control enum (`Continue`/`SkipChildren`/`Stop`).
- `walk` — an iterative, depth-first read traversal that drives a `Visitor`; deep trees are walked without recursing on the call stack.
- `transform` — an iterative, post-order rebuild of a tree into a destination arena, passing each node through a closure; identity yields a faithful deep copy.
- `Arena` and `Id` (from `arena-lang`) and `Span` (from `span-lang`) re-exported from the crate root.
- Behavioural snapshot tests for traversal order, pruning, stopping, deep trees, and identity copy; property tests cross-checking the order invariants against the tree structure; benchmarks for `walk` and `transform`.

### Removed

- The reserved no-op `serde` feature, dropped rather than carried unused toward the frozen 1.0 surface.

---

## [0.1.0] - 2026-06-18

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.85.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).

[Unreleased]: https://github.com/jamesgober/ast-lang/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/jamesgober/ast-lang/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/jamesgober/ast-lang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/ast-lang/releases/tag/v0.1.0
