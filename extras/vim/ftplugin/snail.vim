" Snail filetype plugin
" Language-specific settings for Snail files

if exists("b:did_ftplugin")
  finish
endif
let b:did_ftplugin = 1

" Save local options
let s:save_cpo = &cpo
set cpo&vim

" Set comment format
setlocal commentstring=#\ %s
setlocal comments=:#

" Braces are paired
setlocal matchpairs+=<:>

" Indentation settings
setlocal shiftwidth=4
setlocal tabstop=4
setlocal softtabstop=4
setlocal expandtab
setlocal autoindent
setlocal smartindent

" Define keywords for include searches
setlocal include=^\\s*\\(from\\\|import\\)
setlocal define=^\\s*\\(def\\\|class\\)

" Format program for snail files
if executable('snail')
  setlocal formatprg=snail\ --format
endif

" Completion function
setlocal omnifunc=snail#complete

" Folding based on braces
setlocal foldmethod=syntax

" Key mappings for formatting
nnoremap <buffer> <silent> gq :call snail#format()<CR>
vnoremap <buffer> <silent> gq :call snail#format_range()<CR>

" Commands
command! -buffer SnailFormat call snail#format()
command! -buffer SnailRun call snail#run()
command! -buffer SnailShowPython call snail#show_python()

" Undo ftplugin settings when switching filetypes
let b:undo_ftplugin = "setlocal commentstring< comments< matchpairs<"
      \ . " shiftwidth< tabstop< softtabstop< expandtab<"
      \ . " autoindent< smartindent< include< define< formatprg<"
      \ . " omnifunc< foldmethod<"
      \ . " | nunmap <buffer> gq"
      \ . " | vunmap <buffer> gq"
      \ . " | delcommand SnailFormat"
      \ . " | delcommand SnailRun"
      \ . " | delcommand SnailShowPython"

" Restore options
let &cpo = s:save_cpo
unlet s:save_cpo

