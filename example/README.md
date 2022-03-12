# Benchmarks

This benchmark comparison report shows the difference in performance between iterative (`fib_iter`) and recursive
(`fib_recur`) fibonacci functions.

- [Fibonacci](#fibonacci)

## Fibonacci

Since `fib_recur` is not tail recursive, Rust is forced to make a function call making the recursive version MUCH slower.

|          | `Recursive Fib`           | `Iterative Fib`                      |
|:---------|:--------------------------|:------------------------------------ |
| **`10`** | `112.04 ns` (1.00x)       | `1.39 ns` (✅ **80.35x faster**)      |
| **`20`** | `14.12 us` (1.00x)        | `2.15 ns` (✅ **6573.15x faster**)    |
| **`30`** | `1.73 ms` (1.00x)         | `3.29 ns` (✅ **526813.08x faster**)  |

Made with [criterion-table](https://github.com/nu11ptr/criterion-table)
