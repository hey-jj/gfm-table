//! Render rows of strings as a GitHub Flavored Markdown table.
//!
//! The output is source text for a Markdown parser to read, not box drawing for
//! a terminal to display. Every cell is wrapped in pipes and one space of
//! padding, every column is padded to a common width, the header row is followed
//! by a delimiter row, and each column carries the alignment the caller asked
//! for.
//!
//! Cell content that would break the table is repaired instead of passed
//! through. A `|` becomes `\|` and a line break becomes `<br>`, because GFM
//! reads an unescaped pipe as a column separator and has no way to write a line
//! break inside a cell. [`escape_cell`] states the exact rule. Nothing else is
//! transformed: backticks, angle brackets, HTML, and leading hashes are the
//! caller's business, and a caller who needs general Markdown escaping wants a
//! Markdown serializer instead.
//!
//! Rendering cannot fail. There is no error type and no fallible entry point.
//!
//! # Examples
//!
//! ```
//! use gfm_table::{Alignment, Table};
//!
//! let mut table = Table::new();
//! table.push_row(["name", "count"]);
//! table.push_row(["a|b", "2"]);
//! table.set_alignments([Alignment::Left, Alignment::Right]);
//!
//! assert_eq!(
//!     table.render(),
//!     "| name | count |\n| :--- | ----: |\n| a\\|b |     2 |"
//! );
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

mod escape;
mod render;
mod table;

pub use crate::escape::escape_cell;
pub use crate::table::{Alignment, Table};
