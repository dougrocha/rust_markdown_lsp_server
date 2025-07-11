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

- [ ] Proper document syncing (consider trying incremental sync)
- [x] `textDocument/codeAction`
  - Shows a preview of linked documents on hover
  - [x] Wiki-links
  - [x] Regular links
- [x] `textDocument/definition`
- [x] `textDocument/references`
  - References for headings
  - Link notes Zettelkasten-style
- [ ] `textDocument/rename`
  - Rename files and headers
- [x] `textDocument/codeAction`
  - Extract sections
  - [ ] Bring in a section
- [ ] `textDocument/publishDiagnostics`
  - Show errors not detected by diagnostics
  - Currently half working, will depend on my parser
- [x] `textDocument/completion`
  - Also half working, wondering if I should sort them server side

---

## Inspired By

- [Marksman](https://github.com/artempyanykh/marksman)
- [zk-nvim](https://github.com/zk-org/zk-nvim)
