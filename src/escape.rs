//! The one transformation this crate applies to cell text.

use alloc::borrow::Cow;
use alloc::string::String;

/// Escapes one cell for use inside a GitHub Flavored Markdown table row.
///
/// Two changes, and no others.
///
/// An unescaped `|` gains a backslash. A pipe counts as unescaped when the run
/// of backslashes directly before it has even length, zero included. A pipe
/// preceded by an odd run is already escaped and is left alone, so a caller who
/// escaped by hand is not double escaped.
///
/// A line break becomes `<br>`. `\r\n`, `\n`, and a lone `\r` each produce one
/// `<br>`. GFM has no representation for a line break inside a cell, and a
/// verbatim newline ends the row and reattributes every cell after it. A break
/// that follows an odd run of backslashes takes one more backslash with it, so
/// the run closes and the `<br>` reaches the parser as inline HTML. Without it
/// the run-final backslash escapes the `<` and the break renders as the text
/// `<br>`. That one character is the only backslash this function writes.
/// Backslashes are otherwise untouched, because cells hold Markdown and
/// doubling them would change every deliberate escape in the input.
///
/// Nothing else changes. Backticks, angle brackets, HTML, leading hashes, and
/// control characters other than `\r` and `\n` pass through. U+2028 and U+2029
/// pass through, because GFM does not read them as line endings.
///
/// The result borrows the input when it holds no `|`, no `\r`, and no `\n`.
///
/// [`Table::push_row`](crate::Table::push_row) applies this to every cell, so
/// call it directly only when appending a row to table text you already own.
///
/// # Examples
///
/// ```
/// use gfm_table::escape_cell;
///
/// assert_eq!(escape_cell("a|b"), "a\\|b");
/// assert_eq!(escape_cell("a\\|b"), "a\\|b");
/// assert_eq!(escape_cell("line\r\nbreak"), "line<br>break");
/// assert_eq!(escape_cell("half\\\nopen"), "half\\\\<br>open");
/// assert_eq!(escape_cell("**bold**"), "**bold**");
/// ```
///
/// Escaping twice gives the same string as escaping once.
///
/// ```
/// # use gfm_table::escape_cell;
/// let once = escape_cell("a|b\nc");
/// assert_eq!(escape_cell(&once), once);
/// ```
#[must_use]
pub fn escape_cell(cell: &str) -> Cow<'_, str> {
    // One scan decides whether the input can be borrowed and, if not, how much
    // room the rewrite needs. A pipe grows by one byte, or by none when it is
    // already escaped, and counts either way because the borrow decision is the
    // presence of a pipe. A break grows by three, or by four when it also closes
    // an odd run of backslashes. The `\n` of a `\r\n` pair writes nothing of its
    // own, so it gives one byte back.
    let mut extra = 0usize;
    let mut escaped = false;
    let mut after_carriage_return = false;
    for &byte in cell.as_bytes() {
        match byte {
            b'|' => {
                extra = extra.saturating_add(1);
                escaped = false;
            }
            b'\r' => {
                extra = extra.saturating_add(if escaped { 4 } else { 3 });
                escaped = false;
            }
            b'\n' => {
                if after_carriage_return {
                    extra = extra.saturating_sub(1);
                } else {
                    extra = extra.saturating_add(if escaped { 4 } else { 3 });
                }
                escaped = false;
            }
            b'\\' => escaped = !escaped,
            _ => escaped = false,
        }
        after_carriage_return = byte == b'\r';
    }
    if extra == 0 {
        return Cow::Borrowed(cell);
    }

    let mut out = String::with_capacity(cell.len().saturating_add(extra));
    // Parity of the backslash run ending at the current character. A bool
    // cannot overflow the way a counter can.
    let mut escaped = false;
    // Set by `\r` so the `\n` of a `\r\n` pair does not emit a second `<br>`.
    let mut after_carriage_return = false;

    for character in cell.chars() {
        let carriage_return = character == '\r';
        match character {
            '\\' => {
                escaped = !escaped;
                out.push('\\');
            }
            '|' => {
                if !escaped {
                    out.push('\\');
                }
                escaped = false;
                out.push('|');
            }
            '\r' => {
                if escaped {
                    out.push('\\');
                }
                escaped = false;
                out.push_str("<br>");
            }
            '\n' => {
                if !after_carriage_return {
                    if escaped {
                        out.push('\\');
                    }
                    out.push_str("<br>");
                }
                escaped = false;
            }
            _ => {
                escaped = false;
                out.push(character);
            }
        }
        after_carriage_return = carriage_return;
    }

    Cow::Owned(out)
}
