" Snail autoload functions
" Provides formatting, execution, and completion for Snail files

let s:save_cpo = &cpo
set cpo&vim

" Format the entire buffer
function! snail#format() abort
  let l:view = winsaveview()
  let l:content = join(getline(1, '$'), "\n")
  
  " Use the internal formatter
  let l:formatted = s:format_snail(l:content)
  
  if l:formatted !=# l:content
    " Replace buffer content
    silent! undojoin
    call setline(1, split(l:formatted, "\n", 1))
    " Remove extra lines if formatted is shorter
    let l:old_lines = line('$')
    let l:new_lines = len(split(l:formatted, "\n", 1))
    if l:old_lines > l:new_lines
      execute (l:new_lines + 1) . ',$delete _'
    endif
  endif
  
  call winrestview(l:view)
endfunction

" Format a visual selection
function! snail#format_range() range abort
  let l:content = join(getline(a:firstline, a:lastline), "\n")
  let l:formatted = s:format_snail(l:content)
  
  if l:formatted !=# l:content
    silent! undojoin
    execute a:firstline . ',' . a:lastline . 'delete _'
    call append(a:firstline - 1, split(l:formatted, "\n", 1))
  endif
endfunction

" Internal formatting function
function! s:format_snail(code) abort
  let l:lines = split(a:code, "\n", 1)
  let l:result = []
  let l:indent = 0
  let l:shiftwidth = exists('*shiftwidth') ? shiftwidth() : &shiftwidth
  let l:in_string = 0
  let l:in_triple_string = 0
  
  for l:line in l:lines
    let l:trimmed = substitute(l:line, '^\s*', '', '')
    let l:trimmed = substitute(l:trimmed, '\s*$', '', '')
    
    " Skip empty lines but preserve them
    if l:trimmed ==# ''
      call add(l:result, '')
      continue
    endif
    
    " Handle triple-quoted strings
    if l:in_triple_string
      call add(l:result, l:line)
      if l:trimmed =~# '"""' || l:trimmed =~# "'''"
        let l:in_triple_string = 0
      endif
      continue
    endif
    
    " Check for triple-quoted string start
    if l:trimmed =~# '^\(r\)\?"""' || l:trimmed =~# "^\\(r\\)\\?'''"
      if !(l:trimmed =~# '""".*"""' || l:trimmed =~# "'''.*'''")
        let l:in_triple_string = 1
      endif
    endif
    
    " Decrease indent before closing braces
    let l:open_count = len(substitute(l:trimmed, '[^{]', '', 'g'))
    let l:close_count = len(substitute(l:trimmed, '[^}]', '', 'g'))
    
    " If line starts with closing brace, decrease indent first
    if l:trimmed =~# '^}'
      let l:indent = max([0, l:indent - 1])
    endif
    
    " Handle elif, else, except, finally - decrease indent then increase
    if l:trimmed =~# '^\(elif\|else\|except\|finally\)\>'
      let l:indent = max([0, l:indent - 1])
    endif
    
    " Add the line with proper indentation
    let l:indented = repeat(' ', l:indent * l:shiftwidth) . l:trimmed
    call add(l:result, l:indented)
    
    " Calculate indent change for next line
    if l:trimmed =~# '{\s*$' || l:trimmed =~# '{\s*#.*$'
      let l:indent = l:indent + 1
    elseif l:trimmed =~# '}\s*$'
      " Only decrease if we didn't already for opening line
      if l:trimmed !~# '^}'
        let l:indent = max([0, l:indent - 1])
      endif
    endif
    
    " Handle elif, else, except, finally - they start a new block
    if l:trimmed =~# '^\(elif\|else\|except\|finally\)\>.*{\s*$'
      let l:indent = l:indent + 1
    endif
  endfor
  
  return join(l:result, "\n")
endfunction

" Run the current buffer
function! snail#run() abort
  if !executable('snail')
    echoerr "snail executable not found in PATH"
    return
  endif
  
  let l:file = expand('%:p')
  if empty(l:file)
    " Run from buffer content
    let l:content = join(getline(1, '$'), "\n")
    let l:output = system('snail', l:content)
  else
    write
    let l:output = system('snail -f ' . shellescape(l:file))
  endif
  
  " Display output
  echo l:output
endfunction

" Show generated Python code
function! snail#show_python() abort
  if !executable('snail')
    echoerr "snail executable not found in PATH"
    return
  endif
  
  let l:content = join(getline(1, '$'), "\n")
  let l:python = system('snail --python', l:content)
  
  " Open in a new split
  new
  setlocal buftype=nofile
  setlocal bufhidden=wipe
  setlocal noswapfile
  setlocal filetype=python
  call setline(1, split(l:python, "\n"))
  setlocal nomodifiable
  file [Snail->Python]
endfunction

" Omni completion function
function! snail#complete(findstart, base) abort
  if a:findstart
    " Find start of word
    let l:line = getline('.')
    let l:start = col('.') - 1
    while l:start > 0 && l:line[l:start - 1] =~# '\w'
      let l:start -= 1
    endwhile
    return l:start
  else
    " Complete keywords and builtins
    let l:keywords = [
          \ 'and', 'as', 'assert', 'break', 'class', 'continue',
          \ 'def', 'del', 'elif', 'else', 'except', 'finally',
          \ 'for', 'from', 'if', 'import', 'in', 'is', 'not',
          \ 'or', 'pass', 'raise', 'return', 'try', 'while', 'with',
          \ 'True', 'False', 'None',
          \ 'BEGIN', 'END',
          \ ]
    let l:builtins = [
          \ 'print', 'len', 'range', 'str', 'int', 'float', 'list',
          \ 'dict', 'set', 'tuple', 'bool', 'type', 'isinstance',
          \ 'hasattr', 'getattr', 'setattr', 'open', 'input', 'abs',
          \ 'min', 'max', 'sum', 'sorted', 'reversed', 'enumerate',
          \ 'zip', 'map', 'filter', 'any', 'all', 'json',
          \ ]
    let l:snail_vars = ['$e', '$l', '$f', '$n', '$fn', '$p', '$m']
    
    let l:matches = []
    for l:word in l:keywords + l:builtins + l:snail_vars
      if l:word =~# '^' . a:base
        call add(l:matches, l:word)
      endif
    endfor
    return l:matches
  endif
endfunction

let &cpo = s:save_cpo
unlet s:save_cpo

