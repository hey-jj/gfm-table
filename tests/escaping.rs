//! The escaping rule, character by character.

use std::borrow::Cow;

use gfm_table::escape_cell;

/// Which half of the `Cow` came back.
fn kind(cell: &str) -> &'static str {
    match escape_cell(cell) {
        Cow::Borrowed(_) => "borrowed",
        Cow::Owned(_) => "owned",
    }
}

#[test]
fn an_unescaped_pipe_gains_a_backslash() {
    assert_eq!(escape_cell("a|b"), "a\\|b");
    assert_eq!(escape_cell("|"), "\\|");
    assert_eq!(escape_cell("||"), "\\|\\|");
    assert_eq!(escape_cell("ab|"), "ab\\|");
    assert_eq!(escape_cell("|ab"), "\\|ab");
}

#[test]
fn a_pipe_after_an_odd_run_of_backslashes_is_already_escaped() {
    assert_eq!(escape_cell("a\\|b"), "a\\|b");
    assert_eq!(escape_cell("a\\\\\\|b"), "a\\\\\\|b");
}

#[test]
fn a_pipe_after_an_even_run_of_backslashes_is_not() {
    assert_eq!(escape_cell("a\\\\|b"), "a\\\\\\|b");
    assert_eq!(escape_cell("a\\\\\\\\|b"), "a\\\\\\\\\\|b");
}

#[test]
fn backslashes_on_their_own_are_never_doubled() {
    assert_eq!(escape_cell("\\"), "\\");
    assert_eq!(escape_cell("\\\\"), "\\\\");
    assert_eq!(escape_cell("\\(x\\)"), "\\(x\\)");
    assert_eq!(escape_cell("C:\\path\\to"), "C:\\path\\to");
}

#[test]
fn each_kind_of_line_break_becomes_one_br() {
    assert_eq!(escape_cell("a\nb"), "a<br>b");
    assert_eq!(escape_cell("a\r\nb"), "a<br>b");
    assert_eq!(escape_cell("a\rb"), "a<br>b");
    assert_eq!(escape_cell("a\n\nb"), "a<br><br>b");
    assert_eq!(escape_cell("a\r\n\r\nb"), "a<br><br>b");
    assert_eq!(escape_cell("a\n\rb"), "a<br><br>b");
    assert_eq!(escape_cell("\n"), "<br>");
}

#[test]
fn nothing_else_is_transformed() {
    for cell in [
        "**bold**",
        "`code`",
        "<b>html</b>",
        "# heading",
        "a\tb",
        "\u{0}\u{7}\u{1b}",
        "\u{2028}\u{2029}",
        "e\u{301}\u{200d}\u{1f600}",
        "",
    ] {
        assert_eq!(escape_cell(cell), cell);
    }
}

#[test]
fn escaping_twice_changes_nothing_the_second_time() {
    for cell in [
        "a|b", "a\\|b", "a\nb", "|\r\n|", "\\\\|", "<br>", "\\", "a\\\nb", "\\\r\n\\",
    ] {
        let once = escape_cell(cell).into_owned();
        assert_eq!(escape_cell(&once), once);
        assert_eq!(once.matches('\n').count(), 0);
        assert_eq!(once.matches('\r').count(), 0);
    }
}

#[test]
fn the_result_borrows_exactly_when_nothing_needs_changing() {
    for cell in ["plain", "", "\\", "<br>", "\u{1f600}"] {
        assert_eq!(kind(cell), "borrowed");
    }
    for cell in ["a|b", "a\nb", "a\rb", "a\r\nb", "a\\|b"] {
        assert_eq!(kind(cell), "owned");
    }
}

#[test]
fn an_already_escaped_pipe_is_rewritten_into_the_same_string() {
    // The borrow test is the presence of a pipe, not the need for a change, so
    // a pre-escaped pipe reaches the rewrite and has to come back unchanged.
    let cell = "a\\|b";
    assert_eq!(escape_cell(cell), cell);
    assert_eq!(kind(cell), "owned");
}

#[test]
fn a_break_after_an_odd_run_of_backslashes_gains_one_more() {
    // Without the extra backslash the run is odd, GFM reads it as escaping the
    // `<`, and the break renders as the text `<br>` instead of a break.
    assert_eq!(escape_cell("a\\\nb"), "a\\\\<br>b");
    assert_eq!(escape_cell("a\\\rb"), "a\\\\<br>b");
    assert_eq!(escape_cell("a\\\r\nb"), "a\\\\<br>b");
    assert_eq!(escape_cell("a\\\\\\\nb"), "a\\\\\\\\<br>b");
    assert_eq!(escape_cell("\\\n"), "\\\\<br>");
}

#[test]
fn a_break_after_an_even_run_of_backslashes_is_left_alone() {
    assert_eq!(escape_cell("a\\\\\nb"), "a\\\\<br>b");
    assert_eq!(escape_cell("a\\\\\\\\\rb"), "a\\\\\\\\<br>b");
    assert_eq!(escape_cell("a\nb"), "a<br>b");
}

#[test]
fn only_the_run_before_the_break_grows() {
    // The extra backslash is one character wide and lands on one break. Every
    // other escape in the cell is untouched.
    assert_eq!(escape_cell("\\(x\\)\\\n\\|"), "\\(x\\)\\\\<br>\\|");
}

#[test]
fn the_rewrite_buffer_is_sized_exactly() {
    // The sizing scan and the rewrite have to agree, or a cell of breaks buys a
    // buffer twice the size it fills. The one deliberate slack is an already
    // escaped pipe, counted because the borrow decision needs the count.
    for cell in [
        "a|b".to_string(),
        "a\\\nb".to_string(),
        "\r\n".repeat(1000),
        "\n".repeat(1000),
        "\r".repeat(1000),
        "\\\r\n".repeat(1000),
        "|".repeat(1000),
    ] {
        match escape_cell(&cell) {
            Cow::Owned(out) => assert_eq!(out.capacity(), out.len(), "cell {cell:?}"),
            Cow::Borrowed(_) => panic!("cell {cell:?} needed a rewrite"),
        }
    }
}

#[test]
fn a_long_run_of_pipes_is_escaped_in_full() {
    let cell = "|".repeat(100_000);
    let escaped = escape_cell(&cell);
    assert_eq!(escaped.len(), 200_000);
    assert_eq!(escaped.matches("\\|").count(), 100_000);
}

#[test]
fn a_long_run_of_newlines_becomes_one_br_each() {
    let cell = "\n".repeat(10_000);
    let escaped = escape_cell(&cell);
    assert_eq!(escaped.len(), 40_000);
    assert_eq!(escaped.matches("<br>").count(), 10_000);
}

#[test]
fn a_crlf_run_produces_half_as_many_breaks() {
    let cell = "\r\n".repeat(10_000);
    assert_eq!(escape_cell(&cell).matches("<br>").count(), 10_000);
}
