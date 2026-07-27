# Changelog

## 0.1.0

First release.

- `Table` renders rows of strings as a GitHub Flavored Markdown table, with
  per-column alignment, padded cells, and a delimiter row under the header.
- `escape_cell` turns an unescaped `|` into `\|` and every line break into
  `<br>`, so a cell holding either still parses as one cell.
- `set_cell_width` takes the caller's width function. The default counts `char`
  values.
- No error type. No dependencies. `#![no_std]` with `alloc`.
