" Snail plugin
" A comprehensive Vim plugin for the Snail programming language

if exists('g:loaded_snail')
  finish
endif
let g:loaded_snail = 1

let s:save_cpo = &cpo
set cpo&vim

let s:plugin_path = expand('<sfile>:p')
let s:extras_dir = fnamemodify(s:plugin_path, ':h:h:h')
let s:default_treesitter_dir = s:extras_dir . '/tree-sitter-snail'

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
  if !exists('g:snail_treesitter_registered')
    let g:snail_treesitter_registered = 0
  endif

  function! s:snail_register_treesitter() abort
    let s:treesitter_dir = exists('g:snail_treesitter_dir')
          \ ? g:snail_treesitter_dir
          \ : s:default_treesitter_dir
    if !isdirectory(s:treesitter_dir)
      return
    endif
    let g:snail_treesitter_dir = s:treesitter_dir
    lua << EOF
local ok, parsers = pcall(require, "nvim-treesitter.parsers")
if not ok then
  return
end
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
vim.g.snail_treesitter_registered = 1
EOF
  endfunction

  function! s:snail_treesitter_try() abort
    if g:snail_treesitter_registered
      return
    endif
    call s:snail_register_treesitter()
    if g:snail_treesitter_registered
      augroup snail_treesitter_register
        autocmd!
      augroup END
    endif
  endfunction

  call s:snail_treesitter_try()
  if !g:snail_treesitter_registered
    augroup snail_treesitter_register
      autocmd!
      autocmd VimEnter,BufEnter,FileType * call <SID>snail_treesitter_try()
      autocmd User LazyLoad,LazyDone,PackerLoad,PackerComplete call <SID>snail_treesitter_try()
    augroup END
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
