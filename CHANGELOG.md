# Changelog

## 0.2.0

Breaking.

- `set_cell_width` becomes `with_cell_width`. It consumes the table and returns
  it, and it takes any `Fn(&str) -> usize` rather than a bare function pointer,
  so a closure carrying a width table or a locale can be passed. `Table` gained
  a type parameter for the measure, defaulted to `fn(&str) -> usize`, which
  leaves `Table::new` and every other method as they were.

Other changes.

- `Alignment` derives `PartialOrd` and `Ord`, so it can sit in a sorted
  collection.
- `no-std` joins the keywords, so a keyword search finds the crate the way the
  `no-std` category already did. `table` makes way for it, since the crate name
  and the description both carry that word already.
- Dual licensed under MIT OR Apache-2.0. `LICENSE` is now `LICENSE-MIT`
  and `LICENSE-APACHE` sits beside it. 0.1.0 stays MIT only.
- README states the MSRV in a `## Requirements` section rather than in a line
  under what the crate does not do.

## 0.1.0

First release.

- `Table` renders rows of strings as a GitHub Flavored Markdown table, with
  per-column alignment, padded cells, and a delimiter row under the header.
- `escape_cell` turns an unescaped `|` into `\|` and every line break into
  `<br>`, so a cell holding either still parses as one cell.
- `set_cell_width` takes the caller's width function. The default counts `char`
  values.
- No error type. No dependencies. `#![no_std]` with `alloc`.
