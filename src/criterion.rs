use std::cmp::max;
use std::io::{BufReader, Read};
use std::ops::Div;

use anyhow::anyhow;
use flexstr::{flex_fmt, FlexStr, ToFlex, ToFlexStr};
use indexmap::map::Entry;
use indexmap::IndexMap;
use serde::Deserialize;

// *** Raw JSON Data Structs ***

// NOTE: These were shamelessly copied (with translation) from:
// https://github.com/bheisler/cargo-criterion/blob/main/src/message_formats/json.rs

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ConfidenceInterval {
    estimate: f64,
    lower_bound: f64,
    upper_bound: f64,
    unit: FlexStr,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Throughput {
    per_iteration: u64,
    unit: FlexStr,
}

#[derive(Debug, Deserialize)]
enum ChangeType {
    NoChange,
    Improved,
    Regressed,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ChangeDetails {
    mean: ConfidenceInterval,
    median: ConfidenceInterval,

    change: ChangeType,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct BenchmarkComplete {
    id: FlexStr,
    report_directory: FlexStr,
    iteration_count: Vec<u64>,
    measured_values: Vec<f64>,
    unit: FlexStr,

    throughput: Vec<Throughput>,

    typical: ConfidenceInterval,
    mean: ConfidenceInterval,
    median: ConfidenceInterval,
    median_abs_dev: ConfidenceInterval,
    slope: Option<ConfidenceInterval>,

    change: Option<ChangeDetails>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct BenchmarkGroupComplete {
    group_name: FlexStr,
    benchmarks: Vec<FlexStr>,
    report_directory: FlexStr,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawCriterionData {
    Benchmark(Box<BenchmarkComplete>),
    BenchmarkGroup(Box<BenchmarkGroupComplete>),
}

impl RawCriterionData {
    pub fn from_reader(r: impl Read) -> serde_json::error::Result<Vec<Self>> {
        let reader = BufReader::new(r);
        let mut de = serde_json::Deserializer::from_reader(reader);
        let mut data_vec = Vec::new();

        loop {
            match RawCriterionData::deserialize(&mut de) {
                Ok(data) => data_vec.push(data),
                Err(err) if err.is_eof() => break,
                Err(err) => return Err(err),
            }
        }

        Ok(data_vec)
    }
}

// *** Criterion Data ***

// ### Column Info ###

#[derive(Clone, Debug)]
struct ColumnInfo {
    #[allow(dead_code)]
    name: FlexStr,
    max_width: u32,
}

impl ColumnInfo {
    #[inline]
    pub fn new(name: FlexStr, width: u32) -> Self {
        Self {
            name,
            max_width: width,
        }
    }

    #[inline]
    pub fn update_info(&mut self, width: u32) {
        self.max_width = max(self.max_width, width);
    }
}

// ### Time Unit ###

#[derive(Clone, Copy, Debug)]
enum TimeUnit {
    Second(f64),
    Millisecond(f64),
    Microsecond(f64),
    Nanosecond(f64),
    Picosecond(f64),
}

impl TimeUnit {
    pub fn try_new(time: f64, unit: &str) -> anyhow::Result<Self> {
        match unit {
            "s" => Ok(TimeUnit::Second(time)),
            "ms" => Ok(TimeUnit::Millisecond(time)),
            "us" => Ok(TimeUnit::Microsecond(time)),
            "ns" => Ok(TimeUnit::Nanosecond(time)),
            "ps" => Ok(TimeUnit::Picosecond(time)),
            _ => Err(anyhow!("Unrecognized time unit: {unit}")),
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.to_flex_str().len() as u32
    }

    fn as_picoseconds(&self) -> f64 {
        match *self {
            TimeUnit::Second(s) => s * 1_000_000_000_000.0,
            TimeUnit::Millisecond(ms) => ms * 1_000_000_000.0,
            TimeUnit::Microsecond(us) => us * 1_000_000.0,
            TimeUnit::Nanosecond(ns) => ns * 1_000.0,
            TimeUnit::Picosecond(ps) => ps,
        }
    }
}

impl Div for TimeUnit {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        let unit1 = self.as_picoseconds();
        let unit2 = rhs.as_picoseconds();
        unit1 / unit2
    }
}

impl ToFlexStr for TimeUnit {
    fn to_flex_str(&self) -> FlexStr {
        match self {
            TimeUnit::Second(time) => flex_fmt!("{time:.2} s"),
            TimeUnit::Millisecond(time) => flex_fmt!("{time:.2} ms"),
            TimeUnit::Microsecond(time) => flex_fmt!("{time:.2} us"),
            TimeUnit::Nanosecond(time) => flex_fmt!("{time:.2} ns"),
            TimeUnit::Picosecond(time) => flex_fmt!("{time:.2} ps"),
        }
    }
}

// ### Percent ###

#[derive(Clone, Copy, Debug, Default)]
struct Percent(f64);

impl Percent {
    #[inline]
    pub fn width(self) -> u32 {
        self.to_flex_str().len() as u32
    }
}

impl ToFlexStr for Percent {
    #[inline]
    fn to_flex_str(&self) -> FlexStr {
        flex_fmt!("{:.2}%", self.0)
    }
}

// #### Column ###

#[derive(Clone, Debug)]
struct Column {
    #[allow(dead_code)]
    name: FlexStr,
    time_unit: TimeUnit,
    pct: Percent,
}

impl Column {
    pub fn new(name: FlexStr, time_unit: TimeUnit, first_col_time: Option<TimeUnit>) -> Self {
        let pct = match first_col_time {
            Some(first_col_time) => Percent(first_col_time / time_unit - 1.0),
            None => Default::default(),
        };

        Self {
            name,
            time_unit,
            pct,
        }
    }

    // This returns the "width" of the resulting text in chars. Since we don't know how it will be
    // formatted we return width of: TimeUnit + Percent. Any additional spaces or formatting chars
    // are not considered and must be added by the formatter
    #[inline]
    pub fn width(&self) -> u32 {
        self.time_unit.width() + self.pct.width()
    }
}

// ### Row ###

#[derive(Clone, Debug)]
struct Row {
    #[allow(dead_code)]
    name: FlexStr,
    column_data: IndexMap<FlexStr, Column>,
}

impl Row {
    #[inline]
    pub fn new(name: FlexStr) -> Self {
        Self {
            name,
            column_data: Default::default(),
        }
    }

    // NOTE: The 'first' column here reflects the first column seen for THIS row NOT for the whole table
    // This means our timings COULD be based off different columns in different rows
    fn first_column_time(&self) -> Option<TimeUnit> {
        self.column_data
            .first()
            .map(|(_, Column { time_unit, .. })| *time_unit)
    }

    fn add_column(&mut self, name: FlexStr, time_unit: TimeUnit) -> anyhow::Result<&Column> {
        let first_time = self.first_column_time();

        match self.column_data.entry(name.clone()) {
            Entry::Occupied(_) => Err(anyhow!("Duplicate column: {name}")),
            Entry::Vacant(entry) => {
                let col = Column::new(name, time_unit, first_time);
                Ok(entry.insert(col))
            }
        }
    }
}

// ### Column Info Map ###

#[derive(Clone, Debug, Default)]
struct ColumnInfoMap(IndexMap<FlexStr, ColumnInfo>);

impl ColumnInfoMap {
    pub fn update_column_info(&mut self, name: FlexStr, width: u32) {
        match self.0.entry(name.clone()) {
            // If already exists, then just update with our width data
            Entry::Occupied(entry) => {
                entry.into_mut().update_info(width);
            }
            // If new column, we append to the end
            Entry::Vacant(entry) => {
                entry.insert(ColumnInfo::new(name, width));
            }
        }
    }
}

// ### Table ###

#[derive(Clone, Debug)]
struct Table {
    #[allow(dead_code)]
    name: FlexStr,
    columns: ColumnInfoMap,
    rows: IndexMap<FlexStr, Row>,
}

impl Table {
    #[inline]
    pub fn new(name: FlexStr) -> Self {
        Self {
            name,
            columns: Default::default(),
            rows: Default::default(),
        }
    }

    pub fn add_column_data(
        &mut self,
        column_name: FlexStr,
        row_name: FlexStr,
        time: TimeUnit,
    ) -> anyhow::Result<()> {
        let row = self.get_row(row_name);
        let col = row.add_column(column_name.clone(), time)?;
        let width = col.width();
        self.columns.update_column_info(column_name, width);
        Ok(())
    }

    fn get_row(&mut self, name: FlexStr) -> &mut Row {
        match self.rows.entry(name.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Row::new(name)),
        }
    }
}

// ### Criterion Table Data ###

/// Fully processed Criterion benchmark data ready for formatting
#[derive(Clone, Debug)]
pub struct CriterionTableData {
    tables: IndexMap<FlexStr, Table>,
}

impl CriterionTableData {
    /// Build table data from the input raw Criterion data
    pub fn from_raw(raw_data: &[RawCriterionData]) -> anyhow::Result<Self> {
        let mut data = Self {
            tables: Default::default(),
        };

        data.build_from_raw_data(raw_data)?;
        Ok(data)
    }

    fn build_from_raw_data(&mut self, raw_data: &[RawCriterionData]) -> anyhow::Result<()> {
        for item in raw_data {
            // We only process benchmark data - skip anything else
            if let RawCriterionData::Benchmark(bm) = item {
                // Break the id into table, column, and row respectively
                let mut parts: Vec<FlexStr> = bm.id.split('/').map(|s| s.to_flex()).collect();
                if parts.len() < 2 {
                    return Err(anyhow::anyhow!("Malformed id: {}", &bm.id));
                }

                let (table_name, column_name) = (parts.remove(0), parts.remove(0));
                // If we don't have a row name then we will work with a blank row name
                let row_name = if !parts.is_empty() {
                    parts.remove(0)
                } else {
                    "".into()
                };

                // Find our table, calculate our timing, and add data to our column
                let table = self.get_table(table_name);
                let time_unit = TimeUnit::try_new(bm.typical.estimate, &bm.typical.unit)?;
                table.add_column_data(column_name, row_name, time_unit)?;
            }
        }

        Ok(())
    }

    fn get_table(&mut self, name: FlexStr) -> &mut Table {
        match self.tables.entry(name.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Table::new(name)),
        }
    }
}
