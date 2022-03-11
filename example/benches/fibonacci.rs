use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

#[inline]
fn fib_recur(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fib_recur(n - 1) + fib_recur(n - 2),
    }
}

#[inline]
pub fn fib_iter(n: u64) -> u64 {
    if n == 1 {
        1
    } else {
        let mut sum = 0;
        let mut last = 0;
        let mut curr = 1;

        for _ in 1..n {
            sum = last + curr;
            last = curr;
            curr = sum;
        }

        sum
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fibonacci");

    for row in vec![10, 20] {
        let id = BenchmarkId::new("Recursive Fib", row);
        group.bench_with_input(id, &row, |b, row| b.iter(|| fib_recur(black_box(*row))));

        let id = BenchmarkId::new("Iterative Fib", row);
        group.bench_with_input(id, &row, |b, row| b.iter(|| fib_iter(black_box(*row))));
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
