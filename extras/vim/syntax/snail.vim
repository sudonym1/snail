" Vim syntax file
" Language: Snail
" Install: copy to ~/.vim/syntax/snail.vim and add an ftdetect rule that sets
"          `setfiletype snail` for *.snail files.

if exists("b:current_syntax")
  finish
endif

syn case match

" Basic tokens
syn match snailComment "#.*$"
syn match snailShebang "^#!snail.*$"

" Strings (simple quoting; use Python syntax highlighting for deeper support)
syn region snailString start=+'+ skip=+\\'+ end=+'+
\ contains=snailFormat
syn region snailString start=+"+ skip=+\\"+ end=+"+
\ contains=snailFormat
syn region snailFormat start=+{+ end=+}+ contained

" Keywords roughly mirror Python's set
syn keyword snailKeyword and as assert async await break class continue def del
syn keyword snailKeyword elif else except False finally for from global if import
syn keyword snailKeyword in is lambda match nonlocal None or pass raise return
syn keyword snailKeyword True try while with yield

" Snail-specific syntax helpers
syn match snailSubprocess "\$([^)]*)"
syn match snailSubprocessBg "@([^)]*)"
syn match snailFallbackVar "\$e"
syn match snailSwallow "?" containedin=snailOperator

syn match snailOperator "[][{}():.,]"
syn match snailOperator "==\|!=\|<=\|>=\|[-+*/%<>]"

hi def link snailComment        Comment
hi def link snailShebang        PreProc
hi def link snailString         String
hi def link snailFormat         Special
hi def link snailKeyword        Keyword
hi def link snailSubprocess     Function
hi def link snailSubprocessBg   Function
hi def link snailFallbackVar    Identifier
hi def link snailSwallow        Operator
hi def link snailOperator       Operator

let b:current_syntax = "snail"
