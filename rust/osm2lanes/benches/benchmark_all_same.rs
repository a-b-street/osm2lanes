use std::hash::Hash;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn all_same_vec<T: Clone + PartialEq>(v: &[T]) -> bool {
    if v.is_empty() {
        return false;
    }
    let v: Vec<T> = v.to_vec();
    v.windows(2).all(|w| w[0] == w[1])
}

fn all_same_hashmap<T>(v: &[T]) -> bool
where
    T: Clone + PartialEq + Eq + Hash,
{
    if v.is_empty() {
        return false;
    }
    let v = std::collections::HashSet::<&T>::from_iter(v.iter());
    v.len() == 1
}

fn bench_fibs(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fibonacci");
    let small_vec = vec![20; 5];
    group.bench_with_input(
        BenchmarkId::new("all_same_vec", "small_vec"),
        &small_vec,
        |b, small_vec| b.iter(|| all_same_vec(black_box(small_vec))),
    );
    group.bench_with_input(
        BenchmarkId::new("all_same_hashmap", "small_vec"),
        &small_vec,
        |b, small_vec| b.iter(|| all_same_hashmap(black_box(small_vec))),
    );
    let big_vec = vec![20; 500];
    group.bench_with_input(
        BenchmarkId::new("all_same_vec", "big_vec"),
        &big_vec,
        |b, big_vec| b.iter(|| all_same_vec(black_box(big_vec))),
    );
    group.bench_with_input(
        BenchmarkId::new("all_same_hashmap", "big_vec"),
        &big_vec,
        |b, big_vec| b.iter(|| all_same_hashmap(black_box(big_vec))),
    );
    group.finish();
}

criterion_group!(benches, bench_fibs);
criterion_main!(benches);
