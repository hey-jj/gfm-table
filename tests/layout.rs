//! Padding, column widths, delimiter construction, and alignment fitting.

mod common;

use common::{cells_of, check_invariants};
use gfm_table::{Alignment, Table};

fn one_cell(cell: &str, alignment: Alignment) -> String {
    let mut table = Table::new();
    table.push_row([cell]);
    table.set_alignment(alignment);
    table.render()
}

#[test]
fn each_alignment_writes_its_own_delimiter() {
    assert_eq!(one_cell("abc", Alignment::None), "| abc |\n| --- |");
    assert_eq!(one_cell("abc", Alignment::Left), "| abc |\n| :-- |");
    assert_eq!(one_cell("abc", Alignment::Right), "| abc |\n| --: |");
    assert_eq!(one_cell("abc", Alignment::Center), "| abc |\n| :-: |");
}

#[test]
fn a_column_is_never_narrower_than_its_delimiter_needs() {
    assert_eq!(one_cell("a", Alignment::None), "| a |\n| - |");
    assert_eq!(one_cell("a", Alignment::Left), "| a  |\n| :- |");
    assert_eq!(one_cell("a", Alignment::Right), "|  a |\n| -: |");
    assert_eq!(one_cell("a", Alignment::Center), "|  a  |\n| :-: |");
}

#[test]
fn padding_goes_on_the_side_the_alignment_names() {
    let mut table = Table::new();
    table.push_row(["abcd", "abcd", "abcd", "abcd"]);
    table.push_row(["x", "x", "x", "x"]);
    table.set_alignments([
        Alignment::None,
        Alignment::Left,
        Alignment::Right,
        Alignment::Center,
    ]);
    let out = table.render();
    assert_eq!(
        out,
        "| abcd | abcd | abcd | abcd |\n\
         | ---- | :--- | ---: | :--: |\n\
         | x    | x    |    x |   x  |"
    );
}

#[test]
fn a_centred_cell_with_an_odd_fill_leans_left() {
    let mut table = Table::new();
    table.push_row(["abcd"]);
    table.push_row(["x"]);
    table.set_alignment(Alignment::Center);
    let out = table.render();
    let last = out.lines().last().unwrap();
    assert_eq!(last, "|   x  |");

    let cell = &cells_of(last)[0];
    let leading = cell.len() - cell.trim_start_matches(' ').len();
    let trailing = cell.len() - cell.trim_end_matches(' ').len();
    assert_eq!(leading, trailing + 1);
}

#[test]
fn an_even_fill_splits_evenly() {
    let mut table = Table::new();
    table.push_row(["abcde"]);
    table.push_row(["x"]);
    table.set_alignment(Alignment::Center);
    assert_eq!(table.render().lines().last(), Some("|   x   |"));
}

#[test]
fn the_column_width_is_the_widest_measured_cell() {
    let mut table = Table::new();
    table.push_row(["a", "bb"]);
    table.push_row(["ccc", "d"]);
    table.push_row(["e", "ffff"]);
    assert_eq!(
        table.render(),
        "| a   | bb   |\n| --- | ---- |\n| ccc | d    |\n| e   | ffff |"
    );
}

#[test]
fn the_measure_decides_the_width_not_the_byte_count() {
    let mut table = Table::new();
    table.push_row(["\u{4e2d}\u{6587}"]);
    assert_eq!(table.render(), "| \u{4e2d}\u{6587} |\n| -- |");

    let table = table.with_cell_width(str::len);
    assert_eq!(table.render(), "| \u{4e2d}\u{6587} |\n| ------ |");
}

#[test]
fn a_ragged_table_renders_the_same_as_one_padded_with_empty_cells() {
    let mut ragged = Table::new();
    ragged.push_row(["a"]);
    ragged.push_row(["b", "c", "d"]);
    ragged.push_row(["e", "f"]);

    let mut padded = Table::new();
    padded.push_row(["a", "", ""]);
    padded.push_row(["b", "c", "d"]);
    padded.push_row(["e", "f", ""]);

    assert_eq!(ragged.render(), padded.render());
}

#[test]
fn a_fitted_alignment_slice_renders_the_same_as_the_original() {
    let mut table = Table::new();
    table.push_row(["a", "b"]);

    let mut short = table.clone();
    short.set_alignments([Alignment::Center]);
    let mut fitted = table.clone();
    fitted.set_alignments([Alignment::Center, Alignment::None]);
    assert_eq!(short.render(), fitted.render());

    let mut long = table.clone();
    long.set_alignments([Alignment::Center, Alignment::None, Alignment::Right]);
    assert_eq!(long.render(), fitted.render());
}

#[test]
fn adding_a_row_never_shrinks_a_column() {
    let mut table = Table::new();
    table.push_row(["aaa", "b"]);
    let before: Vec<usize> = cells_of(table.render().lines().next().unwrap())
        .iter()
        .map(String::len)
        .collect();

    table.push_row(["c", "d"]);
    let after: Vec<usize> = cells_of(table.render().lines().next().unwrap())
        .iter()
        .map(String::len)
        .collect();

    assert_eq!(before, after);

    table.push_row(["e", "ffff"]);
    let wider: Vec<usize> = cells_of(table.render().lines().next().unwrap())
        .iter()
        .map(String::len)
        .collect();
    assert_eq!(wider, vec![5, 6]);
}

#[test]
fn there_is_no_trailing_newline() {
    let mut table = Table::new();
    table.push_row(["a"]);
    table.push_row(["b"]);
    let out = table.render();
    assert_eq!(out.chars().last(), Some('|'));
    assert_eq!(out.lines().count(), 3);
}

#[test]
fn rendering_twice_gives_the_same_bytes_and_does_not_escape_twice() {
    let mut table = Table::new();
    table.push_row(["a|b", "c\nd"]);
    let first = table.render();
    assert_eq!(table.render(), first);
    assert_eq!(first, "| a\\|b | c<br>d |\n| ---- | ------ |");
}

#[test]
fn the_invariants_hold_for_a_mixed_table() {
    let mut table = Table::new();
    table.push_row(["name", "value|unit", "note"]);
    table.push_row(["one", "1", "first\nline"]);
    table.push_row(["two"]);
    table.set_alignments([Alignment::Left, Alignment::Center, Alignment::Right]);
    check_invariants(&table, |cell| cell.chars().count());

    let table = table.with_cell_width(|cell| cell.encode_utf16().count());
    check_invariants(&table, |cell| cell.encode_utf16().count());
}

#[test]
fn the_output_string_is_allocated_once_at_the_exact_length() {
    // The capacity is computed from the column widths before anything is
    // written. If it were wrong in either direction the string would have grown
    // and its capacity would no longer equal its length.
    let mut table = Table::new();
    table.push_row(["name", "\u{4e2d}\u{6587}|x", "note"]);
    table.push_row(["one\ntwo", "", "\u{1f600}"]);
    table.push_row(["three"]);
    table.set_alignments([Alignment::Center, Alignment::Right, Alignment::Left]);

    for measure in [
        (|cell: &str| cell.chars().count()) as fn(&str) -> usize,
        |cell: &str| cell.encode_utf16().count(),
        str::len,
    ] {
        let out = table.clone().with_cell_width(measure).render();
        assert_eq!(out.capacity(), out.len());
    }
}
