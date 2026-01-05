-- Snail filetype detection for Neovim
-- This file is automatically loaded by Neovim when in the runtimepath

vim.filetype.add({
  extension = {
    snail = "snail",
  },
  pattern = {
    -- Match shebang lines containing 'snail'
    [".*"] = {
      priority = -math.huge,
      function(path, bufnr)
        local first_line = vim.api.nvim_buf_get_lines(bufnr, 0, 1, false)[1] or ""
        if first_line:match("^#!.*snail") then
          return "snail"
        end
      end,
    },
  },
})

