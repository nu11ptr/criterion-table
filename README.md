# criterion-table

[![Crate](https://img.shields.io/crates/v/criterion-table?style=for-the-badge)](https://crates.io/crates/criterion-table)
[![Docs](https://img.shields.io/docsrs/criterion-table?style=for-the-badge)](https://docs.rs/criterion-table)

Generate markdown comparison tables from 
[Cargo Criterion](https://github.com/bheisler/cargo-criterion) benchmark JSON 
output. 

Currently, the tool is limited to Github Flavored Markdown (GFM), but adding 
new output types is relatively simple.

## Generated Markdown Examples

[Very Basic Report](example/README.md)

[FlexStr Benchmark Report](https://github.com/nu11ptr/flexstr/blob/master/benchmarks/README.md)

## Installation

```bash
# If you don't have it already
cargo install cargo-criterion

# This project
cargo install criterion-table
```

## Usage

1. Ensure your benchmarks meet these basic criteria:

* Benchmark IDs are formatted in two to three sections separated by forward 
  slashes (`/`)
    * The sections are used like this: `<table_name>/<column_name>/[row_name]`
    * Case is not currently altered, so set appropriately for display
    * Row name is the only optional field, and if left blank, all results 
      will be a single blank row
    * If using a very basic `benchmark_function` you would only get a column 
      name by default, which isn't sufficient
    * If using benchmark groups you will get two sections automatically
    * If using benchmark groups and `BenchmarkId` you will get all three 
      sections automatically
* Benchmark data is not reordered, so ensure they execute in the order desired
    * Tables are ordered based on the order they are seen in the data 
      (execution order)
    * The first column seen in each row will be the baseline everything else 
      in that row is compared to, so benchmark execution order matters

### Benchmark Example 1 - Manual ID

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[inline]
fn fibonacci(n: u64) -> u64 {
  match n {
    0 => 1,
    1 => 1,
    n => fibonacci(n-1) + fibonacci(n-2),
  }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let id = "Fibonacci/Recursive Fib/20";
    c.bench_function(id, |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
```

### Benchmark Example 2 - Benchmark Group with Parameters

```rust
use criterion::{black_box, BenchmarkId, criterion_group, criterion_main, 
                Criterion};

#[inline]
fn fibonacci(n: u64) -> u64 {
  match n {
    0 => 1,
    1 => 1,
    n => fibonacci(n-1) + fibonacci(n-2),
  }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fibonacci");
    
    for row in vec![10, 20] {
        let id = BenchmarkId::new("Recursive Fib", row);
        group.bench_with_input(id, &row, |b, row| {
            b.iter(|| fibonacci(black_box(*row)))
        });
    }
    
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
```

2. Create a `tables.toml` configuration file (*Optional*)

This allows you to add commentary to integrate with the tables in the markdown. 
Table names are in lowercase and spaces replaced with dashes. The file must 
be in the local directory. Here is an example:

```toml
[top_comments]
Overview = """
This is a benchmark comparison report.
"""

[table_comments]
fibonacci = """
Since `fibonacci` is not tail recursive or iterative, all these function calls 
are not inlined which makes this version very slow.
"""
```

3. Run Benchmarks and Generate Markdown

This can be done in a couple of different ways:

### Single Step

This method ensures all benchmarks are included in one step

```bash
# Run all benchmarks and convert into the markdown all in one step
cargo criterion --message-format=json | criterion-table > BENCHMARKS.md
```

### Multiple Steps

This method allows better control of order and which benchmarks are included

```bash
# Execute only the desired benchmarks
cargo criterion --bench recursive_fib --message-format=json > recursive_fib.json
cargo criterion --bench iterative_fib --message-format=json > iterative_fib.json

# Reorder before converting into markdown
cat iterative_fib.json recursive_fib.json | criterion-table > BENCHMARKS.md
```

## Adding New Output File Types

Currently, the tool is hardcoded to GFM, but it is easy to add a new output 
type via the `Formatter` trait by creating your own new binary project

1. Add this crate, [FlexStr](https://github.com/nu11ptr/flexstr), and 
   IndexMap to your binary project

```toml
[dependencies]
criterion-table = "0.4"
flexstr = "0.8"
indexmap = "1"
```

2. Create a new type and implement 
[Formatter](https://docs.rs/criterion-table/latest/criterion_table/trait.Formatter.html)

3. Create a `main` function and call 
[build_tables](https://docs.rs/criterion-table/latest/criterion_table/fn.build_tables.html)

NOTE: Replace `GFMFormatter` with your new formatter below 

```rust
use std::io;

use criterion_table::build_tables;
// Replace with your formatter
use criterion_table::formatter::GFMFormatter;

const TABLES_CONFIG: &str = "tables.toml";

fn main() {
    // Replace `GFMFormatter` with your formatter
    match build_tables(io::stdin(), GFMFormatter, TABLES_CONFIG) {
        Ok(data) => {
            println!("{data}");
        }
        Err(err) => {
            eprintln!("An error occurred processing Criterion data: {err}");
        }
    }
}
```

4. Save the returned `String` to the file type of your formatter or write to 
   stdout

## License

This project is licensed optionally under either:

* Apache License, Version 2.0, (LICENSE-APACHE
  or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)
