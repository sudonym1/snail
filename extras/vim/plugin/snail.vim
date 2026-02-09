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
  if !exists('g:snail_treesitter_auto_install')
    let g:snail_treesitter_auto_install = 1
  endif
  if !exists('g:snail_treesitter_auto_install_attempted')
    let g:snail_treesitter_auto_install_attempted = 0
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
local snail_config = configs.snail or parsers.snail
if not snail_config then
  snail_config = {
    install_info = {
      path = vim.g.snail_treesitter_dir,
      url = vim.g.snail_treesitter_dir,
      files = { "src/parser.c" },
      generate_requires_npm = false,
      requires_generate_from_grammar = false,
    },
    filetype = "snail",
  }
end
configs.snail = snail_config
parsers.snail = snail_config
vim.g.snail_treesitter_registered = 1
EOF
  endfunction

  function! s:snail_treesitter_parser_installed() abort
    lua << EOF
local installed = false
local ok_parsers, parsers = pcall(require, "nvim-treesitter.parsers")
if ok_parsers and type(parsers.has_parser) == "function" then
  local ok_has, has = pcall(parsers.has_parser, "snail")
  if ok_has and has == true then
    installed = true
  end
end
if not installed and vim.treesitter and vim.treesitter.language
    and type(vim.treesitter.language.inspect) == "function" then
  installed = pcall(vim.treesitter.language.inspect, "snail")
end
vim.g.snail_treesitter_parser_installed = installed and 1 or 0
EOF
    return get(g:, 'snail_treesitter_parser_installed', 0)
  endfunction

  function! s:snail_treesitter_auto_install() abort
    if !get(g:, 'snail_treesitter_auto_install', 1)
      return
    endif
    if !g:snail_treesitter_registered
      return
    endif
    if get(g:, 'snail_treesitter_auto_install_attempted', 0)
      return
    endif
    if s:snail_treesitter_parser_installed()
      let g:snail_treesitter_auto_install_attempted = 1
      return
    endif

    lua << EOF
local ok_install, install = pcall(require, "nvim-treesitter.install")
if not ok_install or install.install == nil then
  return
end
vim.g.snail_treesitter_auto_install_attempted = 1
vim.schedule(function()
  local ok, err = pcall(install.install, { "snail" }, { summary = false })
  if not ok then
    vim.notify(
      "snail: failed to auto-install tree-sitter parser: " .. tostring(err),
      vim.log.levels.WARN
    )
  end
end)
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
  call s:snail_treesitter_auto_install()

  " nvim-treesitter reloads parser definitions during :TSInstall/:TSUpdate.
  " Re-register Snail after that reload so TSInstall snail stays available.
  augroup snail_treesitter_refresh
    autocmd!
    autocmd User TSUpdate call <SID>snail_register_treesitter()
    autocmd User TSUpdate call <SID>snail_treesitter_auto_install()
  augroup END

  if !g:snail_treesitter_registered
    augroup snail_treesitter_register
      autocmd!
      autocmd VimEnter,BufEnter,FileType * call <SID>snail_treesitter_try()
      autocmd User LazyLoad,LazyDone,PackerLoad,PackerComplete call <SID>snail_treesitter_try()
      autocmd VimEnter,BufEnter,FileType * call <SID>snail_treesitter_auto_install()
      autocmd User LazyLoad,LazyDone,PackerLoad,PackerComplete call <SID>snail_treesitter_auto_install()
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
