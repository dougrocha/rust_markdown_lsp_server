# Another Markdown LSP

A project primarily to learn how an LSP works — and because I like features
from both Marksman and zk-nvim, but I can't use them at the same time.

### Features

Currently, this server supports:

- **Wiki-links preview**
  Example:
  ```md
  Example: [[example_link]]
  To a Header: [[example_link#With Header]]
  ```

---

## Implemented Features

- [x] `textDocument/hover` - Preview linked documents on hover (wiki-links, regular links, and footnotes)
- [x] `textDocument/definition` - Navigate to target files, headers, and footnote definitions
- [x] `textDocument/references` - Find all references to files, headers, and footnotes
- [x] `textDocument/completion` - Autocomplete for `[[`, `](`, `#` (headers/tags), and `[^` (footnotes)
- [x] `textDocument/codeAction` - Extract header sections to new files
- [x] `textDocument/diagnostic` - Report parser errors and malformed tags (partial)
- [x] `textDocument/rename` - Rename files/headers and update all references
- [x] `textDocument/documentSymbol` - Document outline with headers and links
- [x] `textDocument/workspaceSymbol` - Search symbols across workspace
- [-] `textDocument/formatting` - Format markdown documents
- [ ] Proper document syncing (incremental sync instead of full sync)
- [ ] Broken link validation (configured but not active)
- [ ] Missing frontmatter validation (configured but not active)

---

## Inspired By

- [Marksman](https://github.com/artempyanykh/marksman)
- [zk-nvim](https://github.com/zk-org/zk-nvim)
