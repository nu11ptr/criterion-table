use std::io;

use criterion_table::build_tables;
use criterion_table::formatter::GFMFormatter;

const TABLES_CONFIG: &str = "tables.toml";

fn main() {
    match build_tables(io::stdin(), GFMFormatter, TABLES_CONFIG) {
        Ok(data) => {
            println!("{data}");
        }
        Err(err) => {
            eprintln!("An error occurred processing Criterion data: {err}");
        }
    }
}
