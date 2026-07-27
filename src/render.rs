//! Measuring and writing. One pass folds the column widths, one pass writes the
//! text, and there is no third.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Write};

use crate::table::{Alignment, Table};

// Padding and delimiters are written in runs rather than one character at a
// time. A 64 KB column then costs a few writes instead of 64 thousand.
const SPACES: &str = "                                ";
const DASHES: &str = "--------------------------------";

/// Everything the writer needs that the table does not already hold.
pub(crate) struct Layout {
    columns: usize,
    /// Rendered width of each column.
    widths: Vec<usize>,
    /// Alignment of each column, already fitted to the column count.
    alignments: Vec<Alignment>,
    /// Measured width of every cell, row major, `columns` entries per row.
    cells: Vec<usize>,
    /// Exact length of the rendered table in bytes.
    output_len: usize,
}

impl Layout {
    pub(crate) fn of(table: &Table) -> Self {
        let columns = table.columns();
        if columns == 0 {
            return Layout {
                columns,
                widths: Vec::new(),
                alignments: Vec::new(),
                cells: Vec::new(),
                output_len: 0,
            };
        }

        let mut widths = Vec::with_capacity(columns);
        let mut alignments = Vec::with_capacity(columns);
        for column in 0..columns {
            let alignment = table.alignment_at(column);
            // A column starts at the narrowest delimiter its alignment can
            // write, so the subtractions in `write_delimiter` cannot underflow.
            widths.push(alignment.min_width());
            alignments.push(alignment);
        }

        let rows = table.rows.len();
        let mut cells = alloc::vec![0usize; rows.saturating_mul(columns)];
        let mut measured_total = 0usize;
        let mut byte_total = 0usize;
        for (row, slots) in table.rows.iter().zip(cells.chunks_mut(columns)) {
            for ((cell, slot), width) in row.iter().zip(slots.iter_mut()).zip(widths.iter_mut()) {
                // The measure is caller code. A value above the byte length
                // would let one small cell demand an unbounded run of spaces,
                // and no honest metric exceeds it.
                let cell_width = (table.measure)(cell).min(cell.len());
                *slot = cell_width;
                if cell_width > *width {
                    *width = cell_width;
                }
                measured_total = measured_total.saturating_add(cell_width);
                byte_total = byte_total.saturating_add(cell.len());
            }
        }

        // Each of the rows + 1 lines is a leading pipe plus, per column, a
        // space, the cell, a space, and a pipe. Cell bytes and padding are what
        // vary: padding across the whole table is (lines * total width) minus
        // everything measured, and the delimiter line pays a full width. The
        // last term is the newline between lines.
        let lines = rows.saturating_add(1);
        let fixed = columns.saturating_mul(3).saturating_add(1);
        let total_width = widths.iter().copied().fold(0usize, usize::saturating_add);
        let output_len = fixed
            .saturating_mul(lines)
            .saturating_add(total_width.saturating_mul(lines))
            .saturating_add(byte_total)
            .saturating_sub(measured_total)
            .saturating_add(rows);

        Layout {
            columns,
            widths,
            alignments,
            cells,
            output_len,
        }
    }

    pub(crate) fn output_len(&self) -> usize {
        self.output_len
    }
}

pub(crate) fn write_table<W: Write>(table: &Table, layout: &Layout, out: &mut W) -> fmt::Result {
    // No columns means no table. A run of empty lines is not something a caller
    // can paste into a document.
    if layout.columns == 0 {
        return Ok(());
    }

    for (index, (row, widths)) in table
        .rows
        .iter()
        .zip(layout.cells.chunks(layout.columns))
        .enumerate()
    {
        if index > 0 {
            out.write_char('\n')?;
        }
        write_row(layout, row, widths, out)?;
        if index == 0 {
            out.write_char('\n')?;
            write_delimiter(layout, out)?;
        }
    }
    Ok(())
}

fn write_row<W: Write>(
    layout: &Layout,
    row: &[String],
    measured: &[usize],
    out: &mut W,
) -> fmt::Result {
    out.write_char('|')?;
    for (column, ((&width, &alignment), &cell_width)) in layout
        .widths
        .iter()
        .zip(&layout.alignments)
        .zip(measured)
        .enumerate()
    {
        // A row shorter than the table is a row padded with empty cells.
        let cell = row.get(column).map_or("", |cell| cell.as_str());
        let fill = width.saturating_sub(cell_width);
        let left = match alignment {
            Alignment::Right => fill,
            // An odd space goes on the left of a centred cell.
            Alignment::Center => fill / 2 + fill % 2,
            Alignment::None | Alignment::Left => 0,
        };
        out.write_char(' ')?;
        write_run(out, SPACES, left)?;
        out.write_str(cell)?;
        write_run(out, SPACES, fill.saturating_sub(left))?;
        out.write_str(" |")?;
    }
    Ok(())
}

fn write_delimiter<W: Write>(layout: &Layout, out: &mut W) -> fmt::Result {
    out.write_char('|')?;
    for (&width, &alignment) in layout.widths.iter().zip(&layout.alignments) {
        out.write_char(' ')?;
        match alignment {
            Alignment::None => write_run(out, DASHES, width)?,
            Alignment::Left => {
                out.write_char(':')?;
                write_run(out, DASHES, width.saturating_sub(1))?;
            }
            Alignment::Right => {
                write_run(out, DASHES, width.saturating_sub(1))?;
                out.write_char(':')?;
            }
            Alignment::Center => {
                out.write_char(':')?;
                write_run(out, DASHES, width.saturating_sub(2))?;
                out.write_char(':')?;
            }
        }
        out.write_str(" |")?;
    }
    Ok(())
}

fn write_run<W: Write>(out: &mut W, unit: &str, mut count: usize) -> fmt::Result {
    while count >= unit.len() {
        out.write_str(unit)?;
        count -= unit.len();
    }
    // `unit` is ASCII, so any byte index inside it is a character boundary.
    out.write_str(&unit[..count])
}
