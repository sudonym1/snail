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

if !exists('g:snail_highlight_awk_vars')
  let g:snail_highlight_awk_vars = 1
endif

if !exists('g:snail_highlight_interpolation')
  let g:snail_highlight_interpolation = 1
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