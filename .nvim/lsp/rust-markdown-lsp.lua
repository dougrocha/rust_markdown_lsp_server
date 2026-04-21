local root = vim.fs.root(0, "Cargo.toml") or vim.fn.getcwd()

return {
  cmd = { root .. "/target/debug/rust_markdown_lsp" },
  filetypes = { "markdown" },
  root_markers = { "Cargo.toml", ".git" },
}
