# gfm-table

Render rows of strings as a GitHub Flavored Markdown table.

The output is source text for a Markdown parser, not box drawing for a terminal.
Cells are padded, columns share a width, the header is followed by a delimiter
row, and each column carries the alignment you ask for.

```toml
[dependencies]
gfm-table = "0.2.0"
```

## Example

```rust
use gfm_table::{Alignment, Table};

let mut table = Table::new();
table.push_row(["package", "downloads"]);
table.push_row(["gfm-table", "1|2"]);
table.set_alignments([Alignment::Left, Alignment::Right]);

println!("{}", table.render());
```

```text
| package   | downloads |
| :-------- | --------: |
| gfm-table |      1\|2 |
```

## What it escapes

Programs that assemble a table feed the renderer values they did not choose, so
cell content that breaks the format is the interesting case. Two characters get
changed and no others.

- A pipe becomes `\|`. GFM reads an unescaped pipe as a column separator, so one
  pipe in one cell changes that row's cell count and the block stops being a
  table.
- A line break becomes `<br>`. `\n`, `\r\n`, and `\r` each produce one. GFM has
  no line break inside a cell, and a verbatim newline ends the row and
  reattributes every cell after it.

A pipe that already carries an odd run of backslashes is left alone, so escaping
twice is the same as escaping once. The one backslash added anywhere else closes
an odd run that a line break would otherwise leave open, which is what keeps the
`<br>` a break instead of text. Backticks, angle brackets, HTML, and leading
hashes pass through, because cells hold Markdown rather than literal text.

`escape_cell` is public for callers who append a row to table text they already
own.

## What it does not do

- It does not fail. There is no error type, no `Result`, and no input that has
  no output.
- It does not add a trailing newline. Code appending to a document adds the
  blank line the surrounding Markdown needs.
- It does not bundle a display width table. `with_cell_width` takes your
  function, and the default counts `char` values.
- It does not draw tables for a terminal. `tabled` does that, with different
  delimiters and different rules.
- It does not parse Markdown or read a table back into rows.

## Requirements

Zero dependencies. `#![no_std]` with `alloc`, so an allocator is the only thing
needed.

MSRV is 1.56.0, which is what edition 2021 needs. A CI job builds the library and
runs the doctests on that exact release, so the floor is tested and not asserted.
The full test suite runs on stable.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT) at your option.

Unless you state otherwise, any contribution you intentionally submit for
inclusion in this crate, as defined in the Apache-2.0 license, is dual licensed
as above, with no additional terms or conditions.
