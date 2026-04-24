vim.cmd([[set runtimepath+=.nvim]])
vim.lsp.enable("rust-markdown-lsp")

vim.api.nvim_create_user_command("TestRename", function()
	local current_buf = vim.api.nvim_get_current_buf()
	local bufname = vim.api.nvim_buf_get_name(current_buf)
	local current_uri = vim.uri_from_fname(bufname)

	local fname = vim.fn.fnamemodify(bufname, ":t")
	local dir = vim.fn.fnamemodify(bufname, ":h")
	local new_uri = vim.uri_from_fname(dir .. "/test/" .. fname)

	local params = {
		files = {
			{
				oldUri = current_uri,
				newUri = new_uri,
			},
		},
	}

	local will_rename_method = "workspace/willRenameFiles"
	local did_rename_method = "workspace/didRenameFiles"

	local lsp_clients = vim.lsp.get_clients()
	for _, client in ipairs(lsp_clients) do
		if client:supports_method(will_rename_method) then
			local res = client:request_sync(will_rename_method, params, 1000, 0)
			if res and res.result then
				vim.lsp.util.apply_workspace_edit(res.result, client.offset_encoding)
			end
		end
	end

	for _, client in ipairs(lsp_clients) do
		if client:supports_method(did_rename_method) then
			client:notify(did_rename_method, params)
		end
	end
end, {})
