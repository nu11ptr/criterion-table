# Benchmarks

## Table of Contents

- [Overview](#overview)
- [Benchmark Results](#benchmark-results)
    - [Fibonacci](#fibonacci)

## Overview

This benchmark comparison report shows the difference in performance between iterative (`fib_iter`) and recursive
(`fib_recur`) fibonacci functions.

## Benchmark Results

### Fibonacci

Since `fib_recur` is not tail recursive, Rust is forced to make a function call making the recursive version MUCH slower.

|          | `Recursive Fib`           | `Iterative Fib`                      |
|:---------|:--------------------------|:------------------------------------ |
| **`10`** | `111.67 ns` (1.00x)       | `1.38 ns` (✅ **81.00x faster**)      |
| **`20`** | `14.01 us` (1.00x)        | `2.12 ns` (✅ **6600.43x faster**)    |
| **`30`** | `1.73 ms` (1.00x)         | `3.28 ns` (✅ **526537.98x faster**)  |

---
Made with [criterion-table](https://github.com/nu11ptr/criterion-table)

