local root = vim.fs.root(0, "Cargo.toml") or vim.fn.getcwd()

return {
  cmd = { root .. "/target/debug/rust_markdown_lsp" },
  cmd_env = { RUST_LOG = "rust_markdown_lsp=trace,lib_lsp=trace" },
  filetypes = { "markdown" },
  root_markers = { "Cargo.toml", ".git" },
}
