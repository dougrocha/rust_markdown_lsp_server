# Another Markdown LSP

A project primarily to learn how an LSP works — and because I like both
Marksman and zk-nvim, but I can't use them at the same time.

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
- [-] `textDocument/codeAction`
  - Shows a preview of linked documents on hover
  - [x] Wiki-links
  - [ ] Regular links
- [x] `textDocument/definition`
- [ ] `textDocument/references`
  - Link notes Zettelkasten-style
- [ ] `textDocument/rename`
  - Rename files and headers
- [ ] `textDocument/codeAction`
  - Refactor sections
- [ ] `textDocument/publishDiagnostics`
  - Show errors not detected by diagnostics
- [ ] `textDocument/completion`

---

## Inspired By

- [Marksman](https://github.com/artempyanykh/marksman)
- [zk-nvim](https://github.com/zk-org/zk-nvim)
