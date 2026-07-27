//! Invariants 1 through 15 over generated tables. The alphabet is weighted
//! toward the characters that break a table: pipes, backslashes, line breaks,
//! and text whose width is not its byte count.

mod common;

use common::{breaks_as_html, cells_of, check_invariants, check_parses, parse_tables};
use gfm_table::{escape_cell, Alignment, Table};
use proptest::prelude::*;

/// Cell content that reads back as itself once escaped. Markdown syntax is left
/// out here and covered by the structural properties below, because a backtick
/// changes what the parser reports without changing the table.
///
/// Backslashes are in, because a backslash before a line break is the one place
/// they decide whether the break survives. The filter drops the two sequences a
/// parser resolves rather than reports: a backslash escaping another backslash,
/// and a backslash escaping a pipe. Every other backslash reads back as itself.
fn inert_cell() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            30 => prop::char::range('a', 'z'),
            10 => Just('|'),
            8 => Just('\\'),
            6 => Just('\n'),
            3 => Just('\r'),
            6 => Just(' '),
            4 => Just('\u{4e2d}'),
            3 => Just('\u{1f600}'),
            2 => Just('\u{301}'),
        ],
        0..8usize,
    )
    .prop_map(|characters| characters.into_iter().collect::<String>())
    .prop_filter(
        "backslash escapes a backslash or a pipe",
        |cell: &String| !cell.contains("\\\\") && !cell.contains("\\|"),
    )
}

/// Adds the characters that carry Markdown meaning.
fn hostile_cell() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            20 => prop::char::range('a', 'z'),
            10 => Just('|'),
            10 => Just('\\'),
            6 => Just('\n'),
            3 => Just('\r'),
            4 => Just('`'),
            4 => Just(' '),
            3 => Just('*'),
            3 => Just('<'),
            2 => Just('#'),
            3 => Just('\u{1f600}'),
        ],
        0..8usize,
    )
    .prop_map(|characters| characters.into_iter().collect())
}

fn rows_of(cell: impl Strategy<Value = String>) -> impl Strategy<Value = Vec<Vec<String>>> {
    proptest::collection::vec(proptest::collection::vec(cell, 0..5usize), 0..5usize)
}

fn alignments() -> impl Strategy<Value = Vec<Alignment>> {
    proptest::collection::vec(
        prop_oneof![
            Just(Alignment::None),
            Just(Alignment::Left),
            Just(Alignment::Center),
            Just(Alignment::Right),
        ],
        0..7usize,
    )
}

fn build(rows: &[Vec<String>], alignments: &[Alignment]) -> Table {
    let mut table = Table::new();
    for row in rows {
        table.push_row(row);
    }
    table.set_alignments(alignments.to_vec());
    table
}

fn character_count(cell: &str) -> usize {
    cell.chars().count()
}

proptest! {
    #[test]
    fn invariants_one_to_six_and_fifteen_hold(
        rows in rows_of(hostile_cell()),
        aligns in alignments(),
    ) {
        check_invariants(&build(&rows, &aligns), character_count);
    }

    #[test]
    fn the_output_parses_as_the_table_that_was_asked_for(
        rows in rows_of(hostile_cell()),
        aligns in alignments(),
    ) {
        let table = build(&rows, &aligns);
        if table.columns() > 0 {
            check_parses(&table, &aligns);
        } else {
            prop_assert_eq!(parse_tables(&table.render()).len(), 0);
        }
    }

    #[test]
    fn every_cell_reads_back_unchanged_apart_from_its_line_breaks(
        rows in rows_of(inert_cell()),
        aligns in alignments(),
    ) {
        let table = build(&rows, &aligns);
        if table.columns() == 0 {
            return Ok(());
        }
        let parsed = check_parses(&table, &aligns);
        for (row, read_back) in rows.iter().zip(&parsed.rows) {
            for (column, cell) in row.iter().enumerate() {
                // GFM trims spaces and tabs at both ends of a cell.
                let expected = breaks_as_html(cell);
                let expected = expected.trim_matches(|c| c == ' ' || c == '\t');
                prop_assert_eq!(&read_back[column], expected);
            }
        }
    }

    #[test]
    fn escaping_is_idempotent_and_leaves_no_unescaped_pipe(cell in hostile_cell()) {
        let once = escape_cell(&cell).into_owned();
        prop_assert_eq!(escape_cell(&once).into_owned(), once.clone());
        prop_assert_eq!(once.contains('\n'), false);
        prop_assert_eq!(once.contains('\r'), false);
        // Splitting on unescaped pipes gives one field when there are none.
        prop_assert_eq!(common::split_line(&once).len(), 1);
    }

    #[test]
    fn borrowing_happens_exactly_when_nothing_needs_escaping(cell in hostile_cell()) {
        let borrowed = matches!(escape_cell(&cell), std::borrow::Cow::Borrowed(_));
        let clean = !cell.contains('|') && !cell.contains('\n') && !cell.contains('\r');
        prop_assert_eq!(borrowed, clean);
    }

    #[test]
    fn a_ragged_table_renders_like_one_padded_with_empty_cells(
        rows in rows_of(hostile_cell()),
        aligns in alignments(),
    ) {
        let table = build(&rows, &aligns);
        let columns = table.columns();
        let padded: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                let mut row = row.clone();
                row.resize(columns, String::new());
                row
            })
            .collect();
        prop_assert_eq!(build(&padded, &aligns).render(), table.render());
    }

    #[test]
    fn fitting_the_alignment_slice_changes_nothing(
        rows in rows_of(hostile_cell()),
        aligns in alignments(),
    ) {
        let table = build(&rows, &aligns);
        let columns = table.columns();
        let mut fitted = aligns.clone();
        fitted.resize(columns, Alignment::None);
        prop_assert_eq!(build(&rows, &fitted).render(), table.render());
    }

    #[test]
    fn rendering_is_deterministic_and_display_agrees(
        rows in rows_of(hostile_cell()),
        aligns in alignments(),
    ) {
        let table = build(&rows, &aligns);
        let out = table.render();
        prop_assert_eq!(table.render(), out.clone());
        prop_assert_eq!(table.clone().render(), out.clone());
        prop_assert_eq!(table.to_string(), out);
    }

    #[test]
    fn adding_a_row_never_shrinks_a_column(
        rows in rows_of(hostile_cell()),
        extra in proptest::collection::vec(hostile_cell(), 0..5usize),
        aligns in alignments(),
    ) {
        let before = build(&rows, &aligns);
        let mut after = before.clone();
        after.push_row(&extra);
        if before.columns() == 0 || after.columns() == 0 {
            return Ok(());
        }

        let widths = |table: &Table| -> Vec<usize> {
            let out = table.render();
            cells_of(out.lines().next().unwrap())
                .iter()
                .map(|cell| cell.chars().count())
                .collect()
        };
        let (old, new) = (widths(&before), widths(&after));
        for (column, width) in old.iter().enumerate() {
            prop_assert_eq!(new[column] >= *width, true);
            // A column the new row does not reach keeps its width exactly.
            if column >= extra.len() {
                prop_assert_eq!(new[column], *width);
            }
        }
    }

    #[test]
    fn all_three_metrics_agree_on_ascii(
        rows in rows_of(proptest::string::string_regex("[a-z |]{0,6}").unwrap()),
        aligns in alignments(),
    ) {
        let characters = build(&rows, &aligns).render();
        let mut table = build(&rows, &aligns);
        table.set_cell_width(|cell| cell.encode_utf16().count());
        prop_assert_eq!(table.render(), characters.clone());
        table.set_cell_width(str::len);
        prop_assert_eq!(table.render(), characters);
    }

    #[test]
    fn a_prefix_renders_the_same_only_when_the_rest_widens_nothing(
        rows in rows_of(hostile_cell()),
        aligns in alignments(),
        cut in 0..5usize,
    ) {
        let keep = cut.min(rows.len());
        let prefix = build(&rows[..keep], &aligns);
        let whole = build(&rows, &aligns);
        if prefix.columns() == 0 || prefix.columns() != whole.columns() {
            return Ok(());
        }

        let widths = |table: &Table| -> Vec<usize> {
            let out = table.render();
            cells_of(out.lines().nth(1).unwrap())
                .iter()
                .map(|cell| cell.chars().count())
                .collect()
        };
        let same_widths = widths(&prefix) == widths(&whole);
        let prefix_lines: Vec<String> = prefix.render().lines().map(String::from).collect();
        let whole_lines: Vec<String> = whole.render().lines().map(String::from).collect();
        let shares_prefix = whole_lines.starts_with(&prefix_lines);
        prop_assert_eq!(shares_prefix, same_widths);
    }

    #[test]
    fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let text = String::from_utf8_lossy(&bytes);
        let rows: Vec<&str> = text.split(',').collect();
        let mut table = Table::new();
        for row in rows.chunks(3) {
            table.push_row(row);
        }
        table.set_alignments(bytes.iter().map(|byte| match byte % 4 {
            0 => Alignment::None,
            1 => Alignment::Left,
            2 => Alignment::Center,
            _ => Alignment::Right,
        }));
        check_invariants(&table, character_count);
    }
}
