//! Sizes and measures at the edge of what a caller can hand over.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};

use common::{cells_of, check_invariants, parse_tables};
use gfm_table::{Alignment, Table};

#[test]
fn five_thousand_rows_render_and_parse() {
    let mut table = Table::new();
    table.push_row(["index", "value"]);
    for row in 0..5000 {
        table.push_row([row.to_string(), "x".to_string()]);
    }
    let out = table.render();
    assert_eq!(out.lines().count(), 5002);
    assert_eq!(parse_tables(&out).len(), 1);
    assert_eq!(out.lines().last(), Some("| 4999  | x     |"));
}

#[test]
fn a_table_of_1024_columns_renders_every_one() {
    let cells: Vec<String> = (0..1024).map(|column| column.to_string()).collect();
    let mut table = Table::new();
    table.push_row(&cells);
    table.push_row(&cells);
    let out = table.render();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(cells_of(lines[0]).len(), 1024);
    assert_eq!(cells_of(lines[2]).len(), 1024);
    assert_eq!(parse_tables(&out)[0].rows[1].len(), 1024);
}

#[test]
fn a_cell_of_100000_pipes_renders_at_twice_the_width() {
    let mut table = Table::new();
    table.push_row(["|".repeat(100_000)]);
    let out = table.render();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0].len(), 200_000 + 4);
    assert_eq!(lines[1].len(), 200_000 + 4);
    assert_eq!(cells_of(lines[0]).len(), 1);
}

#[test]
fn a_cell_of_10000_newlines_renders_as_that_many_breaks() {
    let mut table = Table::new();
    table.push_row(["\n".repeat(10_000)]);
    let out = table.render();
    assert_eq!(out.matches("<br>").count(), 10_000);
    assert_eq!(out.lines().count(), 2);
}

#[test]
fn an_alignment_slice_far_longer_than_the_table_is_ignored() {
    let mut table = Table::new();
    table.push_row(["a", "b"]);
    table.set_alignments(vec![Alignment::Center; 2 + 50]);
    assert_eq!(table.render(), "|  a  |  b  |\n| :-: | :-: |");
}

#[test]
fn a_measure_that_changes_between_calls_still_renders() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    fn varying(_: &str) -> usize {
        CALLS.fetch_add(1, Ordering::Relaxed)
    }

    let mut table = Table::new();
    table.push_row(["alpha", "beta"]);
    table.push_row(["gamma", "delta"]);
    let table = table.with_cell_width(varying);

    let out = table.render();
    assert_eq!(out.lines().count(), 3);
    // One call per cell per render, and no more.
    assert_eq!(CALLS.load(Ordering::Relaxed), 4);
    assert_eq!(table.render().lines().count(), 3);
    assert_eq!(CALLS.load(Ordering::Relaxed), 8);
}

#[test]
fn a_measure_of_zero_or_usize_max_never_underflows_the_padding() {
    for measure in [(|_: &str| 0) as fn(&str) -> usize, |_: &str| usize::MAX] {
        let mut table = Table::new();
        table.push_row(["a", "\u{1f600}\u{1f600}"]);
        table.push_row(["", "b"]);
        table.set_alignments([Alignment::Center, Alignment::Right]);
        let out = table.with_cell_width(measure).render();
        assert_eq!(out.lines().count(), 3);
        assert_eq!(out.lines().filter(|line| !line.ends_with('|')).count(), 0);
    }
}

#[test]
fn a_wide_table_of_wide_text_holds_its_invariants() {
    let mut table = Table::new();
    table.push_row((0..40).map(|column| format!("col{column}")));
    for row in 0..200 {
        table.push_row((0..40).map(|column| format!("r{row}c{column}\u{4e2d}")));
    }
    table.set_alignments([Alignment::Center, Alignment::Right, Alignment::Left]);
    check_invariants(&table, |cell| cell.chars().count());
    assert_eq!(table.len(), 201);
    assert_eq!(table.columns(), 40);
}

#[test]
fn a_table_can_be_moved_between_threads_and_shared_across_them() {
    // The default width function is a fn pointer and not a closure, which is
    // what keeps these three impls automatic. A boxed trait object would drop
    // two of them.
    fn assert_bounds<T: Send + Sync + Clone + 'static>() {}
    assert_bounds::<Table>();
    assert_bounds::<Alignment>();
}
