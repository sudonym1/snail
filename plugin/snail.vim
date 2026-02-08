" Snail runtime bootstrap for plugin-manager installs from repository root.
" This keeps user config minimal by loading extras/vim automatically.

if exists('g:loaded_snail_repo_bootstrap')
  finish
endif
let g:loaded_snail_repo_bootstrap = 1

let s:repo_root = fnamemodify(expand('<sfile>:p'), ':h:h')
let s:extras_vim_dir = s:repo_root . '/extras/vim'
let s:snail_plugin = s:extras_vim_dir . '/plugin/snail.vim'

if isdirectory(s:extras_vim_dir)
  execute 'set runtimepath+=' . fnameescape(s:extras_vim_dir)
endif

if filereadable(s:snail_plugin) && !exists('g:loaded_snail')
  execute 'source ' . fnameescape(s:snail_plugin)
endif
