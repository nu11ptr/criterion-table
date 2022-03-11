use std::io;
use std::io::Read;

use criterion_table::formatter::GFMFormatter;
use criterion_table::{CriterionTableData, RawCriterionData};

fn main() {
    match process(io::stdin()) {
        Ok(data) => {
            println!("{data}");
        }
        Err(err) => {
            eprintln!("An error occurred processing Criterion data: {err}");
        }
    }
}

fn process(r: impl Read) -> anyhow::Result<String> {
    let raw_data = RawCriterionData::from_reader(r)?;
    let data = CriterionTableData::from_raw(&raw_data)?;
    Ok(data.make_tables(GFMFormatter))
}
