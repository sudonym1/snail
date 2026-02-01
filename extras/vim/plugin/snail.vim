" Snail plugin
" A comprehensive Vim plugin for the Snail programming language

if exists('g:loaded_snail')
  finish
endif
let g:loaded_snail = 1

let s:save_cpo = &cpo
set cpo&vim

" Configuration options with defaults
if !exists('g:snail_format_on_save')
  let g:snail_format_on_save = 0
endif

" Ensure filetype detection is enabled
if has('autocmd')
  filetype plugin indent on
endif

" Register snail filetype (fallback if ftdetect wasn't loaded)
augroup snail_filetype
  autocmd!
  autocmd BufRead,BufNewFile *.snail setfiletype snail
augroup END

" Neovim: register Tree-sitter parser config automatically
if has('nvim')
  let s:extras_dir = fnamemodify(expand('<sfile>:p'), ':h:h:h')
  let s:default_treesitter_dir = s:extras_dir . '/tree-sitter-snail'
  let s:treesitter_dir = exists('g:snail_treesitter_dir')
        \ ? g:snail_treesitter_dir
        \ : s:default_treesitter_dir
  if isdirectory(s:treesitter_dir)
    let g:snail_treesitter_dir = s:treesitter_dir
    lua << EOF
local ok, parsers = pcall(require, "nvim-treesitter.parsers")
if ok then
  local configs = parsers
  if type(parsers.get_parser_configs) == "function" then
    configs = parsers.get_parser_configs()
  end
  if not configs.snail then
    configs.snail = {
      install_info = {
        url = vim.g.snail_treesitter_dir,
        files = { "src/parser.c" },
        generate_requires_npm = false,
        requires_generate_from_grammar = false,
      },
      filetype = "snail",
    }
  end
end
EOF
  endif
endif

" Global commands
command! -nargs=0 SnailVersion echo "Snail Vim plugin v1.0.0"

" Auto-format on save if enabled
augroup snail_autoformat
  autocmd!
  if g:snail_format_on_save
    autocmd BufWritePre *.snail call snail#format()
  endif
augroup END

let &cpo = s:save_cpo
unlet s:save_cpo
