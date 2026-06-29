//! Benchmarks for the two traversals: walking a tree read-only, and rebuilding it
//! into a fresh arena. Both are `O(nodes)` and allocation-light; these measure the
//! per-node cost and confirm it stays flat as the tree grows.

use ast_lang::{Arena, Flow, Id, Node, Span, Visitor, transform, walk};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

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

/// Builds a perfect binary `Add` tree with `2^depth` leaves, iteratively by level,
/// so the builder does not itself recurse.
fn build_tree(depth: u32) -> (Arena<Expr>, Id<Expr>) {
    let mut arena = Arena::with_capacity((1usize << (depth + 1)).saturating_sub(1));
    let mut level: Vec<Id<Expr>> = (0..(1u32 << depth))
        .map(|i| arena.alloc(Expr::Lit(i64::from(i), Span::new(i, i + 1))))
        .collect();
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|pair| arena.alloc(Expr::Add(pair[0], pair[1], Span::new(0, 1))))
            .collect();
    }
    let root = level[0];
    (arena, root)
}

struct Sum(i64);
impl Visitor<Expr> for Sum {
    fn enter(&mut self, _: &Arena<Expr>, _: Id<Expr>, node: &Expr) -> Flow {
        if let Expr::Lit(v, _) = node {
            self.0 = self.0.wrapping_add(*v);
        }
        Flow::Continue
    }
}

fn bench_walk(c: &mut Criterion) {
    let mut group = c.benchmark_group("walk");
    for &depth in &[6u32, 10, 14] {
        let (arena, root) = build_tree(depth);
        let nodes = arena.len();
        group.bench_with_input(BenchmarkId::from_parameter(nodes), &nodes, |b, _| {
            b.iter(|| {
                let mut sum = Sum(0);
                walk(black_box(&arena), black_box(root), &mut sum);
                sum.0
            });
        });
    }
    group.finish();
}

fn bench_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("transform");
    for &depth in &[6u32, 10, 14] {
        let (arena, root) = build_tree(depth);
        let nodes = arena.len();
        group.bench_with_input(BenchmarkId::from_parameter(nodes), &nodes, |b, _| {
            b.iter(|| {
                let mut dst = Arena::with_capacity(nodes);
                let id = transform(black_box(&arena), black_box(root), &mut dst, |n| n);
                black_box(id)
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_walk, bench_transform);
criterion_main!(benches);
