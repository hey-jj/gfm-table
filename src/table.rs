//! The table itself: rows, alignments, and the two entry points that render.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::escape::escape_cell;
use crate::render;

/// Column alignment.
///
/// The variant decides the delimiter row for the column and the side the
/// padding goes on. [`Alignment::None`] omits the colon, which leaves the choice
/// to whatever reads the table.
///
/// # Examples
///
/// ```
/// use gfm_table::{Alignment, Table};
///
/// let mut table = Table::new();
/// table.push_row(["ab"]);
/// table.set_alignment(Alignment::Center);
/// assert_eq!(table.render(), "|  ab |\n| :-: |");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// No colon. The column is padded on the right, like [`Alignment::Left`],
    /// and the reader applies its own default.
    None,
    /// A colon on the left of the delimiter. Padding goes on the right.
    Left,
    /// A colon on each end of the delimiter. Padding is split, and an odd space
    /// goes on the left of the cell.
    Center,
    /// A colon on the right of the delimiter. Padding goes on the left.
    Right,
}

impl Alignment {
    /// Narrowest delimiter that still carries this alignment.
    pub(crate) fn min_width(self) -> usize {
        match self {
            Alignment::None => 1,
            Alignment::Left | Alignment::Right => 2,
            Alignment::Center => 3,
        }
    }
}

/// [`Alignment::None`], which is the delimiter row with no colon in it.
///
/// Written by hand rather than derived, because `#[derive(Default)]` on an enum
/// needs a newer compiler than this crate asks for.
impl Default for Alignment {
    fn default() -> Self {
        Alignment::None
    }
}

/// How the caller set alignments. Fitting to the column count happens at render
/// time, so rows added later still get the alignment the caller asked for.
#[derive(Clone, Debug)]
enum Alignments {
    Every(Alignment),
    PerColumn(Vec<Alignment>),
}

/// Rows of cells that render as one GitHub Flavored Markdown table.
///
/// The first row pushed is the header. Cells are escaped once, when their row is
/// added, so rendering twice does not escape twice.
///
/// # Examples
///
/// ```
/// use gfm_table::Table;
///
/// let mut table = Table::new();
/// table.push_row(["fruit", "count"]);
/// table.push_row(["apple", "3"]);
///
/// assert_eq!(
///     table.render(),
///     "| fruit | count |\n| ----- | ----- |\n| apple | 3     |"
/// );
/// ```
#[derive(Clone)]
pub struct Table {
    pub(crate) rows: Vec<Vec<String>>,
    alignments: Alignments,
    pub(crate) measure: fn(&str) -> usize,
}

fn count_characters(cell: &str) -> usize {
    cell.chars().count()
}

impl Table {
    /// Creates a table with no rows, no alignment, and character counting as
    /// the width measure.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfm_table::Table;
    ///
    /// assert_eq!(Table::new().render(), "");
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Table {
            rows: Vec::new(),
            alignments: Alignments::Every(Alignment::None),
            measure: count_characters,
        }
    }

    /// Adds one row. The first row added is the header.
    ///
    /// Rows may differ in length. The table is as wide as its widest row and
    /// short rows render with empty cells at the end.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfm_table::Table;
    ///
    /// let mut table = Table::new();
    /// table.push_row(["a", "b"]);
    /// table.push_row(["c"]);
    ///
    /// assert_eq!(table.render(), "| a | b |\n| - | - |\n| c |   |");
    /// ```
    pub fn push_row<I, S>(&mut self, cells: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let row = cells
            .into_iter()
            .map(|cell| escape_cell(cell.as_ref()).into_owned())
            .collect();
        self.rows.push(row);
        self
    }

    /// Sets one alignment per column, replacing whatever was set before.
    ///
    /// A slice shorter than the table is filled out with [`Alignment::None`].
    /// A slice longer than the table has its extra entries ignored. Neither is
    /// an error, because the column count is not known until the last row is in.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfm_table::{Alignment, Table};
    ///
    /// let mut table = Table::new();
    /// table.push_row(["a", "b", "c"]);
    /// table.set_alignments([Alignment::Right]);
    ///
    /// assert_eq!(table.render(), "|  a | b | c |\n| -: | - | - |");
    /// ```
    pub fn set_alignments<I>(&mut self, alignments: I) -> &mut Self
    where
        I: IntoIterator<Item = Alignment>,
    {
        self.alignments = Alignments::PerColumn(alignments.into_iter().collect());
        self
    }

    /// Sets one alignment for every column, replacing whatever was set before.
    ///
    /// The value applies however wide the table turns out to be.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfm_table::{Alignment, Table};
    ///
    /// let mut table = Table::new();
    /// table.set_alignment(Alignment::Left);
    /// table.push_row(["a", "b"]);
    ///
    /// assert_eq!(table.render(), "| a  | b  |\n| :- | :- |");
    /// ```
    pub fn set_alignment(&mut self, alignment: Alignment) -> &mut Self {
        self.alignments = Alignments::Every(alignment);
        self
    }

    /// Sets the function that measures a cell for padding.
    ///
    /// The default counts `char` values. Pass `|s| s.encode_utf16().count()` for
    /// UTF-16 code units, or a display width function from a crate of your
    /// choice. The function runs once per cell per render.
    ///
    /// A measurement larger than the cell's length in bytes is clamped to that
    /// length. Every plausible metric returns at most the byte count, so the
    /// clamp cannot change a correct result. It is there so one small cell
    /// cannot demand an unbounded run of spaces.
    ///
    /// # Panics
    ///
    /// A panic inside the supplied function propagates out of [`Table::render`]
    /// or [`core::fmt::Display`]. The table is not modified while rendering, so
    /// nothing is left half written.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfm_table::Table;
    ///
    /// let mut table = Table::new();
    /// table.push_row(["\u{1f600}"]);
    /// assert_eq!(table.render(), "| \u{1f600} |\n| - |");
    ///
    /// table.set_cell_width(|cell| cell.encode_utf16().count());
    /// assert_eq!(table.render(), "| \u{1f600} |\n| -- |");
    /// ```
    pub fn set_cell_width(&mut self, measure: fn(&str) -> usize) -> &mut Self {
        self.measure = measure;
        self
    }

    /// Number of rows, header included.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfm_table::Table;
    ///
    /// let mut table = Table::new();
    /// table.push_row(["a"]);
    /// assert_eq!(table.len(), 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Reports whether any row has been added.
    ///
    /// A table of rows that are all empty is not empty by this test, and it
    /// still renders as the empty string. [`Table::columns`] is the test for
    /// empty output.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfm_table::Table;
    ///
    /// let mut table = Table::new();
    /// assert_eq!(table.is_empty(), true);
    ///
    /// let empty_row: [&str; 0] = [];
    /// table.push_row(empty_row);
    /// assert_eq!(table.is_empty(), false);
    /// assert_eq!(table.columns(), 0);
    /// assert_eq!(table.render(), "");
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Cell count of the widest row.
    ///
    /// This is the column count of the rendered table, and the length an
    /// alignment slice should have to be used in full. [`Table::render`] returns
    /// the empty string when it is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfm_table::Table;
    ///
    /// let mut table = Table::new();
    /// table.push_row(["a"]);
    /// table.push_row(["b", "c", "d"]);
    /// assert_eq!(table.columns(), 3);
    /// ```
    #[must_use]
    pub fn columns(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }

    /// Renders the table as Markdown.
    ///
    /// Lines are joined with `\n` and there is no trailing newline. Code
    /// appending this to a document adds the blank line the surrounding
    /// Markdown needs.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfm_table::Table;
    ///
    /// let mut table = Table::new();
    /// table.push_row(["a"]);
    ///
    /// let mut document = String::from("Results:\n\n");
    /// document.push_str(&table.render());
    /// document.push('\n');
    ///
    /// assert_eq!(document, "Results:\n\n| a |\n| - |\n");
    /// ```
    #[must_use]
    pub fn render(&self) -> String {
        let layout = render::Layout::of(self);
        let mut out = String::with_capacity(layout.output_len());
        // `core::fmt::Write` for `String` never fails, so there is no error to
        // handle and nothing for a caller to do about one.
        let _ = render::write_table(self, &layout, &mut out);
        out
    }

    /// Alignment for one column, after fitting to the column count.
    pub(crate) fn alignment_at(&self, column: usize) -> Alignment {
        match &self.alignments {
            Alignments::Every(alignment) => *alignment,
            Alignments::PerColumn(list) => list.get(column).copied().unwrap_or(Alignment::None),
        }
    }
}

/// An empty table, the same as [`Table::new`].
impl Default for Table {
    fn default() -> Self {
        Table::new()
    }
}

/// Prints the stored rows and the alignments. The width function is left out
/// because a function pointer's `Debug` is an address, which would make the
/// output differ between runs.
impl fmt::Debug for Table {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Table")
            .field("rows", &self.rows)
            .field("alignments", &self.alignments)
            .finish_non_exhaustive()
    }
}

/// Writes the same bytes as [`Table::render`], with no trailing newline.
///
/// Formatting flags such as width, fill, and precision are ignored. A value that
/// spans several lines has no sensible response to them.
impl fmt::Display for Table {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let layout = render::Layout::of(self);
        render::write_table(self, &layout, formatter)
    }
}

/// Collects rows straight into a table, header first.
///
/// # Examples
///
/// ```
/// use gfm_table::Table;
///
/// let records = vec![vec!["id", "name"], vec!["1", "alpha"]];
/// let table: Table = records.into_iter().collect();
///
/// assert_eq!(table.render(), "| id | name  |\n| -- | ----- |\n| 1  | alpha |");
/// ```
impl<R, S> FromIterator<R> for Table
where
    R: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fn from_iter<I: IntoIterator<Item = R>>(rows: I) -> Self {
        let mut table = Table::new();
        table.extend(rows);
        table
    }
}

/// Appends rows to a table that already has some.
///
/// # Examples
///
/// ```
/// use gfm_table::Table;
///
/// let mut table = Table::new();
/// table.push_row(["id"]);
/// table.extend(vec![vec!["1"], vec!["2"]]);
///
/// assert_eq!(table.len(), 3);
/// ```
impl<R, S> Extend<R> for Table
where
    R: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fn extend<I: IntoIterator<Item = R>>(&mut self, rows: I) {
        let iterator = rows.into_iter();
        self.rows.reserve(iterator.size_hint().0);
        for row in iterator {
            self.push_row(row);
        }
    }
}
