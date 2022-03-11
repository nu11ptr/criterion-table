use std::cmp::max;
use std::io::{BufReader, Read};
use std::ops::Div;

use anyhow::anyhow;
use flexstr::{flex_fmt, FlexStr, IntoFlex, ToCase, ToFlex, ToFlexStr};
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
pub struct ColumnInfo {
    pub name: FlexStr,
    pub max_width: usize,
}

impl ColumnInfo {
    #[inline]
    pub fn new(name: FlexStr, width: usize) -> Self {
        Self {
            name,
            max_width: width,
        }
    }

    #[inline]
    fn update_info(&mut self, width: usize) {
        self.max_width = max(self.max_width, width);
    }
}

// ### Time Unit ###

#[derive(Clone, Copy, Debug)]
pub enum TimeUnit {
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
    pub fn width(&self) -> usize {
        self.to_flex_str().chars().count()
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
pub struct Comparison(f64);

impl Comparison {
    #[inline]
    pub fn width(self) -> usize {
        self.to_flex_str().chars().count()
    }
}

impl ToFlexStr for Comparison {
    fn to_flex_str(&self) -> FlexStr {
        if self.0 > 1.0 {
            flex_fmt!("{:.2}x faster", self.0)
        } else if self.0 < 1.0 {
            flex_fmt!("{:.2}x slower", 1.0 / self.0)
        } else {
            flex_fmt!("{:.2}x", self.0)
        }
    }
}

// #### Column ###

#[derive(Clone, Debug)]
struct Column {
    #[allow(dead_code)]
    name: FlexStr,
    time_unit: TimeUnit,
    pct: Comparison,
}

impl Column {
    pub fn new(name: FlexStr, time_unit: TimeUnit, first_col_time: Option<TimeUnit>) -> Self {
        let pct = match first_col_time {
            Some(first_col_time) => Comparison(first_col_time / time_unit),
            None => Comparison(1.0),
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
    pub fn width(&self) -> usize {
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
struct ColumnInfoVec(Vec<ColumnInfo>);

impl ColumnInfoVec {
    pub fn update_column_info(&mut self, idx: usize, name: FlexStr, width: usize) {
        match self.0.iter_mut().find(|col| col.name == name) {
            Some(col_info) => col_info.update_info(width),
            None => self.0.insert(idx, ColumnInfo::new(name, width)),
        }
    }
}

// ### Table ###

#[derive(Clone, Debug)]
struct Table {
    #[allow(dead_code)]
    name: FlexStr,
    columns: ColumnInfoVec,
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
        idx: usize,
        column_name: FlexStr,
        row_name: FlexStr,
        time: TimeUnit,
    ) -> anyhow::Result<()> {
        // Assume we have a blank named first column just for holding the row name
        self.columns
            .update_column_info(0, Default::default(), row_name.chars().count());

        let row = self.get_row(row_name);
        let col = row.add_column(column_name.clone(), time)?;

        // Use either the width of the data or the name, whichever is larger
        let width = max(col.width(), column_name.chars().count());
        self.columns.update_column_info(idx, column_name, width);
        Ok(())
    }

    fn get_row(&mut self, name: FlexStr) -> &mut Row {
        match self.rows.entry(name.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Row::new(name)),
        }
    }
}

// ### Column Position ###

#[derive(Default)]
struct ColumnPosition(IndexMap<FlexStr, usize>);

impl ColumnPosition {
    pub fn next_idx(&mut self, row_name: FlexStr) -> usize {
        match self.0.entry(row_name) {
            Entry::Occupied(mut entry) => {
                *entry.get_mut() += 1;
                *entry.get()
            }
            Entry::Vacant(entry) => *entry.insert(1),
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
        let mut col_pos = ColumnPosition::default();

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

                let idx = col_pos.next_idx(row_name.clone());
                table.add_column_data(idx, column_name, row_name, time_unit)?;
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

    pub fn make_tables(&self, mut f: impl Formatter) -> String {
        // We have no idea how big this will be, but might as well not go tiny
        let mut buffer = String::with_capacity(65535);

        // Start of doc
        let table_names: Vec<_> = self.tables.keys().collect();
        f.start(&mut buffer, &table_names);

        for table in self.tables.values() {
            let col_info = &table.columns.0;

            if let Some(first_col) = col_info.first() {
                // Start of table
                f.start_table(&mut buffer, &table.name, col_info);

                for row in table.rows.values() {
                    // Start of row
                    f.start_row(&mut buffer, &row.name, first_col.max_width);

                    for col in &col_info[1..] {
                        match row.column_data.get(&col.name) {
                            // Used column
                            Some(col_data) => f.used_column(
                                &mut buffer,
                                col_data.time_unit,
                                col_data.pct,
                                col.max_width,
                            ),
                            // Unused column
                            None => f.unused_column(&mut buffer, col.max_width),
                        }
                    }

                    // End of row
                    f.end_row(&mut buffer);
                }

                // End of table
                f.end_table(&mut buffer);
            }
        }

        // End of doc
        f.end(&mut buffer);

        buffer
    }
}

pub trait Formatter {
    fn start(&mut self, buffer: &mut String, tables: &[&FlexStr]);

    fn end(&mut self, buffer: &mut String);

    fn start_table(&mut self, buffer: &mut String, name: &FlexStr, columns: &[ColumnInfo]);

    fn end_table(&mut self, buffer: &mut String);

    fn start_row(&mut self, buffer: &mut String, name: &FlexStr, max_width: usize);

    fn end_row(&mut self, buffer: &mut String);

    fn used_column(
        &mut self,
        buffer: &mut String,
        time: TimeUnit,
        pct: Comparison,
        max_width: usize,
    );

    fn unused_column(&mut self, buffer: &mut String, max_width: usize);
}

const CT_URL: &str = "https://github.com/nu11ptr/criterion_compare";

// *** NOTE: These are in _bytes_, not _chars_ - since ASCII right now this is ok ***
// Width of making a single item bold
const FIRST_COL_EXTRA_WIDTH: usize = "**``**".len();
// Width of a single item in bold (italics is less) + one item in back ticks + one item in parens + one space
// NOTE: Added two more "X" because we added unicode check and x that won't be 1 byte each
const USED_EXTRA_WIDTH: usize = "() ``****XX".len();

pub struct GFMFormatter;

impl GFMFormatter {
    fn pad(buffer: &mut String, ch: char, max_width: usize, written: usize) {
        // Pad the rest of the column (inclusive to handle trailing space)
        let remaining = max_width - written;

        for _ in 0..=remaining {
            buffer.push(ch);
        }
    }

    #[inline]
    fn encode_link(s: &FlexStr) -> FlexStr {
        s.replace(' ', "-").into_flex().to_lower()
    }
}

impl Formatter for GFMFormatter {
    fn start(&mut self, buffer: &mut String, tables: &[&FlexStr]) {
        buffer.push_str("# Benchmarks\n\n");

        for &table in tables {
            buffer.push_str("- [");
            buffer.push_str(table);
            buffer.push_str("](#");
            buffer.push_str(&Self::encode_link(table));
            buffer.push_str(")\n");
        }

        buffer.push('\n');
    }

    fn end(&mut self, buffer: &mut String) {
        buffer.push_str("Made with [criterion-table](");
        buffer.push_str(CT_URL);
        buffer.push_str(")\n");
    }

    fn start_table(&mut self, buffer: &mut String, name: &FlexStr, columns: &[ColumnInfo]) {
        // *** Title ***

        buffer.push_str("## ");
        buffer.push_str(name);
        buffer.push_str("\n\n");

        // *** Header Row ***

        buffer.push_str("| ");
        // Safety: Any slicing up to index 1 is always safe - guaranteed to have at least one column
        let first_col_max_width = columns[0].max_width + FIRST_COL_EXTRA_WIDTH;
        Self::pad(buffer, ' ', first_col_max_width, 0);

        // Safety: Any slicing up to index 1 is always safe - guaranteed to have at least one column
        for column in &columns[1..] {
            let max_width = column.max_width + USED_EXTRA_WIDTH;

            buffer.push_str("| `");
            buffer.push_str(&column.name);
            buffer.push('`');
            Self::pad(buffer, ' ', max_width, column.name.chars().count() + 2);
        }

        buffer.push_str(" |\n");

        // *** Deliminator Row ***

        // Right now, everything is left justified
        buffer.push_str("|:");
        Self::pad(buffer, '-', first_col_max_width, 0);

        // Safety: Any slicing up to index 1 is always safe - guaranteed to have at least one column
        for column in &columns[1..] {
            let max_width = column.max_width + USED_EXTRA_WIDTH;

            buffer.push_str("|:");
            Self::pad(buffer, '-', max_width, 0);
        }

        buffer.push_str(" |\n");
    }

    fn end_table(&mut self, buffer: &mut String) {
        buffer.push('\n');
    }

    fn start_row(&mut self, buffer: &mut String, name: &FlexStr, max_width: usize) {
        // Regular row name
        let written = if !name.is_empty() {
            buffer.push_str("| **`");
            buffer.push_str(name);
            buffer.push_str("`**");
            name.chars().count() + FIRST_COL_EXTRA_WIDTH
        // Empty row name
        } else {
            buffer.push_str("| ");
            0
        };

        Self::pad(buffer, ' ', max_width + FIRST_COL_EXTRA_WIDTH, written);
    }

    fn end_row(&mut self, buffer: &mut String) {
        buffer.push_str(" |\n");
    }

    fn used_column(
        &mut self,
        buffer: &mut String,
        time: TimeUnit,
        compare: Comparison,
        max_width: usize,
    ) {
        let (time_str, speedup_str) = (time.to_flex_str(), compare.to_flex_str());

        // Positive = bold
        let data = if speedup_str.contains("faster") {
            flex_fmt!("`{time_str}` (✅ **{speedup_str}**)")
        // Negative = italics
        } else if speedup_str.contains("slower") {
            flex_fmt!("`{time_str}` (❌ *{speedup_str}*)")
        // Even = no special formatting
        } else {
            flex_fmt!("`{time_str}` ({speedup_str})")
        };

        buffer.push_str("| ");
        buffer.push_str(&data);

        let max_width = max_width + USED_EXTRA_WIDTH;
        Self::pad(buffer, ' ', max_width, data.chars().count());
    }

    fn unused_column(&mut self, buffer: &mut String, max_width: usize) {
        buffer.push_str("| ");
        let data = "`N/A`";
        buffer.push_str(data);

        Self::pad(
            buffer,
            ' ',
            max_width + USED_EXTRA_WIDTH,
            data.chars().count(),
        );
    }
}
