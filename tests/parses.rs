//! The promise the crate exists for: an independent GFM parser reads the output
//! as the table the caller described, whatever the cells hold.

mod common;

use common::{breaks_as_html, check_parses, parse_tables};
use gfm_table::{Alignment, Table};

fn table_of(rows: &[&[&str]]) -> Table {
    let mut table = Table::new();
    for row in rows {
        table.push_row(*row);
    }
    table
}

#[test]
fn a_pipe_in_a_cell_does_not_break_the_table_apart() {
    // Passing the pipe through gives a header of three cells against a
    // delimiter row of two, and GFM then refuses to read the block as a table.
    let table = table_of(&[&["a|b", "c"], &["d", "e"]]);
    let parsed = check_parses(&table, &[]);
    assert_eq!(parsed.rows[0], vec!["a|b".to_string(), "c".to_string()]);
    assert_eq!(parsed.rows[1], vec!["d".to_string(), "e".to_string()]);
}

#[test]
fn a_cell_that_is_only_a_pipe_still_parses() {
    let table = table_of(&[&["|"]]);
    let parsed = check_parses(&table, &[]);
    assert_eq!(parsed.rows[0], vec!["|".to_string()]);
}

#[test]
fn a_newline_in_a_cell_does_not_split_the_table_in_two() {
    // Passing the newline through leaves a stray paragraph followed by a
    // different table whose header is the wrong data.
    let table = table_of(&[&["a\nb", "c"], &["d", "e"]]);
    let out = table.render();
    assert_eq!(parse_tables(&out).len(), 1);

    let parsed = check_parses(&table, &[]);
    assert_eq!(parsed.rows[0], vec!["a<br>b".to_string(), "c".to_string()]);
    assert_eq!(parsed.rows[1], vec!["d".to_string(), "e".to_string()]);
}

#[test]
fn every_line_break_arrives_as_one_br() {
    for cell in ["a\nb", "a\r\nb", "a\rb", "\n", "a\n\nb"] {
        let table = table_of(&[&[cell]]);
        let parsed = check_parses(&table, &[]);
        assert_eq!(parsed.rows[0][0], breaks_as_html(cell));
    }
}

#[test]
fn a_break_after_an_odd_run_of_backslashes_still_arrives_as_one_br() {
    // The parser reads a lone backslash before `<` as an escape, so an odd run
    // would swallow the break and hand back the text `<br>`. Left is the cell,
    // right is what the parser reads back, with each odd run resolving to half
    // its length in literal backslashes.
    for (cell, expected) in [
        ("a\\\nb", "a\\<br>b"),
        ("a\\\rb", "a\\<br>b"),
        ("a\\\r\nb", "a\\<br>b"),
        ("a\\\\\\\nb", "a\\\\<br>b"),
        ("\\\n", "\\<br>"),
        ("a\\\n\\\nb", "a\\<br>\\<br>b"),
    ] {
        let table = table_of(&[&[cell]]);
        let parsed = check_parses(&table, &[]);
        assert_eq!(parsed.rows[0][0], expected, "cell {cell:?}");
    }
}

#[test]
fn an_even_run_of_backslashes_before_a_break_reads_back_unchanged() {
    for (cell, expected) in [("a\\\\\nb", "a\\<br>b"), ("a\nb", "a<br>b")] {
        let table = table_of(&[&[cell]]);
        let parsed = check_parses(&table, &[]);
        assert_eq!(parsed.rows[0][0], expected, "cell {cell:?}");
    }
}

#[test]
fn the_parser_sees_the_alignments_that_were_asked_for() {
    let wanted = [
        Alignment::Right,
        Alignment::None,
        Alignment::Center,
        Alignment::Left,
    ];
    let mut table = table_of(&[&["a", "b", "c", "d"], &["e", "f", "g", "h"]]);
    table.set_alignments(wanted);
    check_parses(&table, &wanted);

    for alignment in wanted {
        let mut single = table_of(&[&["a", "b"]]);
        single.set_alignment(alignment);
        check_parses(&single, &[alignment, alignment]);
    }
}

#[test]
fn a_ragged_table_parses_with_the_full_column_count() {
    let table = table_of(&[&["a"], &["b", "c", "d"], &["e", "f"]]);
    let parsed = check_parses(&table, &[]);
    assert_eq!(parsed.rows.len(), 3);
    assert_eq!(
        parsed.rows[0],
        vec!["a".to_string(), String::new(), String::new()]
    );
    assert_eq!(
        parsed.rows[2],
        vec!["e".to_string(), "f".to_string(), String::new()]
    );
}

#[test]
fn no_cell_content_is_lost_moved_or_merged() {
    let cells = [
        "plain",
        "|",
        "a|b|c",
        "\u{4e2d}\u{6587}",
        "\u{1f469}\u{200d}\u{2764}\u{fe0f}\u{200d}\u{1f469}",
        "a\nb",
        "  spaced  ",
        "tab\there",
    ];
    let mut table = Table::new();
    table.push_row(cells);
    table.push_row(cells.iter().rev());

    let parsed = check_parses(&table, &[]);
    for (index, cell) in cells.iter().enumerate() {
        // GFM trims spaces and tabs at both ends of a cell before reading it.
        // The rendered source keeps them, and the parser is what drops them.
        let expected = breaks_as_html(cell);
        let expected = expected.trim_matches(|c| c == ' ' || c == '\t');
        assert_eq!(parsed.rows[0][index], expected);
    }
}

#[test]
fn a_header_only_table_parses_with_no_body_rows() {
    let table = table_of(&[&["a", "b"]]);
    let parsed = check_parses(&table, &[]);
    assert_eq!(parsed.rows.len(), 1);
}

#[test]
fn a_table_inside_a_document_still_parses() {
    let table = table_of(&[&["a|b"], &["c\nd"]]);
    let document = format!("Before.\n\n{}\n\nAfter.\n", table);
    let parsed = parse_tables(&document);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].rows[1], vec!["c<br>d".to_string()]);
}
