use crate::{ColumnInfo, Comparison, Formatter, TimeUnit};
use flexstr::{flex_fmt, FlexStr, IntoFlex, ToCase, ToFlexStr};

const CT_URL: &str = "https://github.com/nu11ptr/criterion-table";

// *** NOTE: These are in _bytes_, not _chars_ - since ASCII right now this is ok ***
// Width of making a single item bold
const FIRST_COL_EXTRA_WIDTH: usize = "**``**".len();
// Width of a single item in bold (italics is less) + one item in back ticks + one item in parens + one space
// NOTE: Added two more "X" because we added unicode check and x that won't be 1 byte each
const USED_EXTRA_WIDTH: usize = "() ``****XX".len();

// *** GFM Formatter ***

/// This formatter outputs Github Flavored Markdown
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
    fn start(&mut self, buffer: &mut String, comment: Option<&FlexStr>, tables: &[&FlexStr]) {
        buffer.push_str("# Benchmarks\n\n");

        if let Some(comments) = comment {
            buffer.push_str(comments);
            buffer.push('\n');
        }

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

    fn start_table(
        &mut self,
        buffer: &mut String,
        name: &FlexStr,
        comment: Option<&FlexStr>,
        columns: &[ColumnInfo],
    ) {
        // *** Title ***

        buffer.push_str("## ");
        buffer.push_str(name);
        buffer.push_str("\n\n");

        if let Some(comments) = comment {
            buffer.push_str(comments);
            buffer.push('\n');
        }

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
