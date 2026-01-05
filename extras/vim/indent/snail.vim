" Snail indent file
" Language: Snail
" Maintainer: Snail contributors

if exists("b:did_indent")
  finish
endif
let b:did_indent = 1

setlocal indentexpr=GetSnailIndent()
setlocal indentkeys=0{,0},0),0],!^F,o,O,e,<:>

" Only define the function once
if exists("*GetSnailIndent")
  finish
endif

let s:save_cpo = &cpo
set cpo&vim

function! GetSnailIndent()
  let lnum = prevnonblank(v:lnum - 1)
  
  " Start of file
  if lnum == 0
    return 0
  endif

  let prevline = getline(lnum)
  let curline = getline(v:lnum)
  let ind = indent(lnum)

  " Increase indent after opening brace or block-starting keywords
  if prevline =~ '{\s*$' || prevline =~ '{\s*#.*$'
    let ind = ind + shiftwidth()
  endif

  " Increase indent after keywords that start blocks
  if prevline =~ '^\s*\(if\|elif\|else\|while\|for\|def\|class\|try\|except\|finally\|with\|BEGIN\|END\)\>'
        \ && prevline !~ '}\s*$' && prevline !~ '{\s*$'
    " Check if next line has the opening brace
    let nextline = getline(v:lnum)
    if nextline =~ '^\s*{'
      " Keep same indent for the opening brace
      return ind
    endif
  endif

  " Decrease indent for closing brace
  if curline =~ '^\s*}'
    let ind = ind - shiftwidth()
  endif

  " Handle elif, else, except, finally at same level as if/try
  if curline =~ '^\s*\(elif\|else\|except\|finally\)\>'
    " Find matching if/try
    let [matchlnum, matchcol] = searchpairpos('{', '', '}', 'bnW')
    if matchlnum > 0
      let ind = indent(matchlnum)
    endif
  endif

  " Handle END at same level as BEGIN
  if curline =~ '^\s*END\>'
    let ind = 0
  endif

  " Handle BEGIN at column 0
  if curline =~ '^\s*BEGIN\>'
    let ind = 0
  endif

  return ind
endfunction

let &cpo = s:save_cpo
unlet s:save_cpo

