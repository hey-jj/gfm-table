//! Every degenerate shape, pinned to the exact string it renders.

use gfm_table::{Alignment, Table};

fn table_of(rows: &[&[&str]]) -> Table {
    let mut table = Table::new();
    for row in rows {
        table.push_row(*row);
    }
    table
}

#[test]
fn no_rows_render_the_empty_string() {
    assert_eq!(Table::new().render(), "");
    assert_eq!(Table::new().columns(), 0);
}

#[test]
fn rows_that_are_all_empty_render_the_empty_string() {
    let empty: &[&str] = &[];
    for count in 1..4 {
        let mut table = Table::new();
        for _ in 0..count {
            table.push_row(empty);
        }
        assert_eq!(table.render(), "");
        assert_eq!(table.len(), count);
        assert_eq!(table.columns(), 0);
    }
}

#[test]
fn an_alignment_does_not_rescue_a_table_with_no_columns() {
    let empty: &[&str] = &[];
    let mut table = Table::new();
    table.push_row(empty);
    table.set_alignment(Alignment::Center);
    assert_eq!(table.render(), "");
}

#[test]
fn one_row_of_one_empty_cell_is_the_alignment_minimum_wide() {
    assert_eq!(table_of(&[&[""]]).render(), "|   |\n| - |");
}

#[test]
fn a_header_on_its_own_still_renders_a_delimiter_row() {
    let out = table_of(&[&["Alpha", "Bravo"]]).render();
    assert_eq!(out, "| Alpha | Bravo |\n| ----- | ----- |");
    assert_eq!(out.lines().count(), 2);
}

#[test]
fn a_header_shorter_than_a_body_row_is_padded_to_the_full_width() {
    let empty: &[&str] = &[];
    let mut table = Table::new();
    table.push_row(empty);
    table.push_row(["a", "b"]);
    assert_eq!(table.render(), "|   |   |\n| - | - |\n| a | b |");
}

#[test]
fn a_body_row_shorter_than_the_header_renders_empty_cells() {
    let out = table_of(&[&["a", "b"], &["c"]]).render();
    assert_eq!(out, "| a | b |\n| - | - |\n| c |   |");
}

#[test]
fn a_cell_of_only_spaces_is_kept_and_counted() {
    assert_eq!(table_of(&[&["  "]]).render(), "|    |\n| -- |");
}

#[test]
fn a_cell_of_only_a_pipe_renders_two_characters_wide() {
    assert_eq!(table_of(&[&["|"]]).render(), "| \\| |\n| -- |");
}

#[test]
fn a_cell_of_only_a_newline_renders_four_characters_wide() {
    assert_eq!(table_of(&[&["\n"]]).render(), "| <br> |\n| ---- |");
}

#[test]
fn a_cell_that_is_already_escaped_is_left_alone() {
    assert_eq!(table_of(&[&["a\\|b"]]).render(), "| a\\|b |\n| ---- |");
    assert_eq!(table_of(&[&["a<br>b"]]).render(), "| a<br>b |\n| ------ |");
}

#[test]
fn a_zero_length_alignment_slice_leaves_every_column_unaligned() {
    let mut table = table_of(&[&["a", "b"]]);
    table.set_alignments([]);
    assert_eq!(table.render(), "| a | b |\n| - | - |");
}

#[test]
fn alignments_beyond_the_last_column_are_ignored() {
    let mut table = table_of(&[&["a"]]);
    table.set_alignments(vec![Alignment::Center; 51]);
    assert_eq!(table.render(), "|  a  |\n| :-: |");
}

#[test]
fn a_short_alignment_slice_leaves_the_rest_unaligned() {
    let mut table = table_of(&[&["a", "b", "c"]]);
    table.set_alignments([Alignment::Right]);
    assert_eq!(table.render(), "|  a | b | c |\n| -: | - | - |");
}

#[test]
fn the_later_alignment_call_replaces_the_earlier_one() {
    let mut table = table_of(&[&["a", "b"]]);
    table.set_alignments([Alignment::Right, Alignment::Right]);
    table.set_alignment(Alignment::Left);
    assert_eq!(table.render(), "| a  | b  |\n| :- | :- |");

    table.set_alignments([Alignment::Center]);
    assert_eq!(table.render(), "|  a  | b |\n| :-: | - |");
}

#[test]
fn alignments_set_before_the_rows_still_reach_them() {
    let mut table = Table::new();
    table.set_alignment(Alignment::Right);
    table.push_row(["ab", "c"]);
    assert_eq!(table.render(), "| ab |  c |\n| -: | -: |");
}

#[test]
fn a_measure_of_zero_falls_back_to_the_alignment_minimum() {
    let mut table = table_of(&[&["wide", "wider"]]);
    table.set_cell_width(|_| 0);
    table.set_alignments([Alignment::None, Alignment::Center]);
    assert_eq!(table.render(), "| wide  |   wider  |\n| - | :-: |");
}

#[test]
fn a_measure_of_usize_max_is_clamped_to_the_byte_length() {
    let mut table = table_of(&[&["\u{e9}"]]);
    table.set_cell_width(|_| usize::MAX);
    assert_eq!(table.render(), "| \u{e9} |\n| -- |");
}

#[test]
fn control_bytes_and_bidi_controls_pass_through() {
    let cell = "a\u{0}b\u{feff}c\u{202e}d";
    let out = table_of(&[&[cell]]).render();
    assert_eq!(out.matches(cell).count(), 1);
    assert_eq!(out, "| a\u{0}b\u{feff}c\u{202e}d |\n| ------- |");
}

#[test]
fn a_64_kb_cell_renders_at_its_full_width() {
    let cell = "x".repeat(64 * 1024);
    let out = table_of(&[&[cell.as_str()], &["y"]]).render();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].len(), 64 * 1024 + 4);
    assert_eq!(lines[1].len(), 64 * 1024 + 4);
    assert_eq!(lines[2].len(), 64 * 1024 + 4);
}

#[test]
fn display_writes_the_same_bytes_as_render() {
    let mut table = table_of(&[&["a|b", "c"], &["d\ne"]]);
    table.set_alignments([Alignment::Center, Alignment::Right]);
    assert_eq!(table.to_string(), table.render());
    assert_eq!(format!("{:>80}", table), table.render());
}

#[test]
fn debug_prints_the_rows_and_the_alignments_and_repeats_itself() {
    let mut table = table_of(&[&["a|b"]]);
    table.set_alignments([Alignment::Right]);
    table.set_cell_width(|_| 1);
    let printed = format!("{:?}", table);
    assert_eq!(printed, format!("{:?}", table.clone()));
    assert_eq!(
        printed,
        "Table { rows: [[\"a\\\\|b\"]], alignments: PerColumn([Right]), .. }"
    );
}

#[test]
fn a_table_collected_from_rows_equals_one_built_by_pushing() {
    let rows = vec![vec!["a", "b"], vec!["c"]];
    let collected: Table = rows.clone().into_iter().collect();
    assert_eq!(
        collected.render(),
        table_of(&[&["a", "b"], &["c"]]).render()
    );

    let mut extended = Table::new();
    extended.extend(rows);
    assert_eq!(extended.render(), collected.render());
    assert_eq!(extended.len(), 2);
}

#[test]
fn a_default_table_is_an_empty_table() {
    assert_eq!(Table::default().render(), Table::new().render());
    assert_eq!(Alignment::default(), Alignment::None);
}
