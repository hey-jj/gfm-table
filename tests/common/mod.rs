//! Shared checks. Each test file is its own crate and uses part of this, so
//! the unused half would warn without the allow.
#![allow(dead_code)]

use gfm_table::{Alignment, Table};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// One table as an independent GFM parser sees it.
pub struct ParsedTable {
    pub alignments: Vec<pulldown_cmark::Alignment>,
    /// Header first, then body rows. Cell text and inline HTML concatenated.
    pub rows: Vec<Vec<String>>,
}

/// Reads markdown with the GFM table extension on and returns every table in it.
pub fn parse_tables(markdown: &str) -> Vec<ParsedTable> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);

    let mut tables: Vec<ParsedTable> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut in_cell = false;

    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Start(Tag::Table(alignments)) => tables.push(ParsedTable {
                alignments,
                rows: Vec::new(),
            }),
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_cell = false;
                row.push(std::mem::take(&mut cell));
            }
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                if let Some(table) = tables.last_mut() {
                    table.rows.push(std::mem::take(&mut row));
                }
            }
            // A `<br>` arrives as inline HTML and not as text, so invariant 8
            // has to join both kinds.
            Event::Text(text) | Event::InlineHtml(text) if in_cell => cell.push_str(&text),
            _ => {}
        }
    }
    tables
}

/// Splits one rendered line into its cells. A pipe separates cells when the run
/// of backslashes before it has even length.
pub fn split_line(line: &str) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut backslashes = 0usize;
    for character in line.chars() {
        if character == '|' && backslashes % 2 == 0 {
            fields.push(std::mem::take(&mut current));
            backslashes = 0;
            continue;
        }
        if character == '\\' {
            backslashes += 1;
        } else {
            backslashes = 0;
        }
        current.push(character);
    }
    fields.push(current);
    fields
}

/// Cells of one line, with the empty pieces outside the leading and trailing
/// pipes removed. Panics if either delimiter is missing, which invariant 3 bans.
pub fn cells_of(line: &str) -> Vec<String> {
    let mut fields = split_line(line);
    assert_eq!(fields.first().map(String::as_str), Some(""));
    assert_eq!(fields.last().map(String::as_str), Some(""));
    fields.remove(0);
    fields.pop();
    fields
}

/// Checks invariants 1 through 6 and 15 for a table rendered with `measure`.
pub fn check_invariants<F: Fn(&str) -> usize>(table: &Table<F>, measure: fn(&str) -> usize) {
    let out = table.render();
    let columns = table.columns();

    // 1. Empty output exactly when there are no columns.
    assert_eq!(out.is_empty(), columns == 0);
    // 15. Display agrees with render.
    assert_eq!(table.to_string(), out);
    if columns == 0 {
        return;
    }

    // 2. One line per row plus the delimiter, and no trailing newline.
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), table.len() + 1);
    assert_eq!(out.chars().last(), Some('|'));

    let widths: Vec<usize> = cells_of(lines[1])
        .iter()
        .map(|delimiter| measure(delimiter.trim_matches(' ')))
        .collect();
    // 5. Every column is at least as wide as its delimiter needs.
    assert_eq!(widths.iter().filter(|width| **width == 0).count(), 0);

    for line in &lines {
        // 3. Every line starts and ends with a pipe and holds k + 1 of them.
        assert_eq!(line.chars().next(), Some('|'));
        assert_eq!(line.chars().last(), Some('|'));
        let cells = cells_of(line);
        assert_eq!(cells.len(), columns);
        for (cell, width) in cells.iter().zip(&widths) {
            // 4. Every line measures the same, column by column.
            assert_eq!(measure(cell), width + 2);
        }
    }
}

/// Checks invariant 7: the output parses as one table with the shape and the
/// alignments the caller asked for.
pub fn check_parses<F: Fn(&str) -> usize>(table: &Table<F>, expected: &[Alignment]) -> ParsedTable {
    let out = table.render();
    let mut tables = parse_tables(&out);
    assert_eq!(tables.len(), 1);
    let parsed = tables.remove(0);

    assert_eq!(parsed.rows.len(), table.len());
    for row in &parsed.rows {
        assert_eq!(row.len(), table.columns());
    }
    let wanted: Vec<pulldown_cmark::Alignment> = (0..table.columns())
        .map(
            |column| match expected.get(column).copied().unwrap_or_default() {
                Alignment::None => pulldown_cmark::Alignment::None,
                Alignment::Left => pulldown_cmark::Alignment::Left,
                Alignment::Center => pulldown_cmark::Alignment::Center,
                Alignment::Right => pulldown_cmark::Alignment::Right,
            },
        )
        .collect();
    assert_eq!(parsed.alignments, wanted);
    parsed
}

/// Input cell as invariant 8 expects to read it back: every line break replaced
/// by one `<br>`.
pub fn breaks_as_html(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    let mut characters = cell.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                out.push_str("<br>");
            }
            '\n' => out.push_str("<br>"),
            other => out.push(other),
        }
    }
    out
}
