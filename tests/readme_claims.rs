//! Every behavioural sentence in the README, executed. A claim with no test
//! here is a claim to delete.

use gfm_table::{escape_cell, Alignment, Table};

const README: &str = include_str!("../README.md");
const MANIFEST: &str = include_str!("../Cargo.toml");

/// Contents of the first fenced block with the given tag.
fn fenced_block(tag: &str) -> String {
    let opening = format!("```{tag}\n");
    let start = README.find(&opening).expect("block is missing") + opening.len();
    let rest = &README[start..];
    let end = rest.find("```").expect("block is unterminated");
    rest[..end].trim_end().to_string()
}

fn manifest_value(key: &str) -> String {
    let line = MANIFEST
        .lines()
        .find(|line| line.starts_with(&format!("{key} = ")))
        .expect("key is missing");
    line.trim_start_matches(&format!("{key} = "))
        .trim_matches('"')
        .to_string()
}

#[test]
fn the_readme_example_renders_the_output_the_readme_shows() {
    let mut table = Table::new();
    table.push_row(["package", "downloads"]);
    table.push_row(["gfm-table", "1|2"]);
    table.set_alignments([Alignment::Left, Alignment::Right]);

    assert_eq!(table.render(), fenced_block("text"));
}

#[test]
fn the_readme_install_line_matches_the_manifest_version() {
    let version = manifest_value("version");
    let expected = format!("[dependencies]\ngfm-table = \"{version}\"");
    assert_eq!(fenced_block("toml"), expected);
}

#[test]
fn the_readme_names_the_manifest_rust_version() {
    // The claim is that the README states the manifest's MSRV, not that it
    // states it in any one sentence. So the version is what is pinned, in
    // either the full `1.56.0` form or the `1.56` one, and the prose around it
    // is free to change.
    let rust_version = manifest_value("rust-version");
    assert!(!rust_version.is_empty(), "the manifest names no MSRV");
    let mut parts = rust_version.split('.');
    let short = match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) => format!("{major}.{minor}"),
        _ => rust_version.clone(),
    };

    // A digit or a dot after the match would make it a different version, so
    // `1.56` is not found inside `1.560` or inside `1.56.1`.
    let names_it = |wanted: &str| {
        README.match_indices(wanted).any(|(at, _)| {
            let after = &README[at + wanted.len()..];
            !after.starts_with(|next: char| next == '.' || next.is_ascii_digit())
        })
    };
    assert!(
        names_it(&rust_version) || names_it(&short),
        "the README does not name the manifest rust-version {rust_version}"
    );
}

#[test]
fn the_crate_really_builds_without_the_standard_library() {
    // The attribute on its own is the claim. What backs it is that no module
    // names `std`, which is the thing that would break a bare-metal build.
    for source in [
        include_str!("../src/lib.rs"),
        include_str!("../src/escape.rs"),
        include_str!("../src/render.rs"),
        include_str!("../src/table.rs"),
    ] {
        assert_eq!(source.matches("std::").count(), 0);
        assert_eq!(source.matches("use std").count(), 0);
    }
    assert_eq!(
        include_str!("../src/lib.rs").matches("#![no_std]").count(),
        1
    );
}

#[test]
fn the_crate_really_has_zero_dependencies() {
    assert_eq!(README.matches("Zero dependencies").count(), 1);
    assert_eq!(MANIFEST.matches("\n[dependencies]").count(), 0);
}

#[test]
fn a_pipe_becomes_an_escaped_pipe_and_a_break_becomes_br() {
    assert_eq!(escape_cell("|"), "\\|");
    assert_eq!(escape_cell("\n"), "<br>");
    assert_eq!(escape_cell("\r\n"), "<br>");
    assert_eq!(escape_cell("\r"), "<br>");
}

#[test]
fn escaping_twice_is_the_same_as_escaping_once_and_only_a_break_adds_a_backslash() {
    let once = escape_cell("a|b\\|c").into_owned();
    assert_eq!(once, "a\\|b\\|c");
    assert_eq!(escape_cell(&once), once);
    assert_eq!(escape_cell("\\\\"), "\\\\");
    assert_eq!(escape_cell("a\\\nb"), "a\\\\<br>b");
}

#[test]
fn backticks_angle_brackets_html_and_leading_hashes_pass_through() {
    for cell in ["`code`", "<b>", "<b>bold</b>", "# heading"] {
        assert_eq!(escape_cell(cell), cell);
    }
}

#[test]
fn there_is_no_trailing_newline_to_strip() {
    let mut table = Table::new();
    table.push_row(["a"]);
    assert_eq!(table.render().chars().last(), Some('|'));
}

#[test]
fn the_default_measure_counts_char_values() {
    let mut table = Table::new();
    table.push_row(["\u{4e2d}\u{6587}"]);
    assert_eq!(table.render(), "| \u{4e2d}\u{6587} |\n| -- |");
}

#[test]
fn no_input_is_left_without_an_output() {
    // "no input that has no output": every shape renders, including the ones
    // that render to nothing.
    let empty: &[&str] = &[];
    let mut table = Table::new();
    assert_eq!(table.render(), "");
    table.push_row(empty);
    assert_eq!(table.render(), "");
    // The empty row pushed above is the header, so it widens to the body row.
    table.push_row(["\u{0}|\r\n"]);
    assert_eq!(table.render(), "|         |\n| ------- |\n| \u{0}\\|<br> |");
}
