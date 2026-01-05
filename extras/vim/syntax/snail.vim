" Vim syntax file
" Language: Snail
" Maintainer: Snail contributors
" Latest Revision: 2025
" Description: Comprehensive syntax highlighting for the Snail programming language
"              Generated from src/snail.pest grammar for consistency with parser
"
" Install: Copy the entire extras/vim/ directory to ~/.vim/ (or ~/.config/nvim/)
"          The ftdetect/snail.vim file will automatically detect .snail files

if exists("b:current_syntax")
  finish
endif

let s:save_cpo = &cpo
set cpo&vim

syn case match

" =============================================================================
" COMMENTS
" =============================================================================
syn match snailComment "#.*$" contains=snailTodo,@Spell
syn keyword snailTodo TODO FIXME XXX NOTE HACK BUG contained

" =============================================================================
" NUMBERS (from grammar: number = @{ ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? })
" =============================================================================
syn match snailNumber "\<\d\+\>"
syn match snailFloat "\<\d\+\.\d\+\>"

" =============================================================================
" STRINGS (from grammar: string with raw_prefix, triple/single/double variants)
" =============================================================================
" Triple-quoted strings (must come before single-quoted to match first)
syn region snailTripleString start=+"""+ end=+"""+ contains=snailInterpolation,snailEscape,@Spell
syn region snailTripleString start=+'''+ end=+'''+ contains=snailInterpolation,snailEscape,@Spell
syn region snailRawTripleString start=+r"""+ end=+"""+ contains=@Spell
syn region snailRawTripleString start=+r'''+ end=+'''+ contains=@Spell

" Single/double quoted strings
syn region snailString start=+"+ skip=+\\"+ end=+"+ contains=snailInterpolation,snailEscape,@Spell
syn region snailString start=+'+ skip=+\\'+ end=+'+ contains=snailInterpolation,snailEscape,@Spell
syn region snailRawString start=+r"+ skip=+\\"+ end=+"+ contains=@Spell
syn region snailRawString start=+r'+ skip=+\\'+ end=+'+ contains=@Spell

" String interpolation: {expr} inside strings
syn region snailInterpolation matchgroup=snailInterpolationDelim start=+{+ end=+}+ contained contains=TOP,snailInterpolationDelim
syn match snailEscapedBrace "{{" contained
syn match snailEscapedBrace "}}" contained

" Escape sequences in strings
syn match snailEscape +\\[abfnrtv'"\\]+ contained
syn match snailEscape +\\x\x\{2}+ contained
syn match snailEscape +\\u\x\{4}+ contained
syn match snailEscape +\\U\x\{8}+ contained
syn match snailEscape +\\N{[^}]*}+ contained
syn match snailEscape +\\\o\{1,3}+ contained

" =============================================================================
" REGEX (from grammar: regex = @{ "/" ~ ( "\\/" | !"/" ~ ANY )* ~ "/" })
" =============================================================================
syn region snailRegex start=+/+ skip=+\\/+ end=+/+ contains=snailRegexEscape,snailRegexClass
syn match snailRegexEscape +\\.+ contained
syn region snailRegexClass start=+\[+ end=+\]+ contained contains=snailRegexEscape

" =============================================================================
" KEYWORDS (from grammar: keyword rule)
" =============================================================================
" Control flow
syn keyword snailConditional if elif else
syn keyword snailRepeat while for
syn keyword snailStatement break continue pass return raise
syn keyword snailException try except finally

" Definitions
syn keyword snailDefine def class nextgroup=snailFunction,snailClass skipwhite

" Operators as keywords
syn keyword snailOperatorWord and or not in is

" Import
syn keyword snailImport import from as

" Context managers and assertions
syn keyword snailStatement with assert del

" AWK mode keywords
syn keyword snailAwkKeyword BEGIN END

" =============================================================================
" CONSTANTS (from grammar: boolean, none)
" =============================================================================
syn keyword snailBoolean True False
syn keyword snailNone None

" =============================================================================
" FUNCTIONS AND CLASSES
" =============================================================================
syn match snailFunction "\h\w*" contained
syn match snailClass "\h\w*" contained

" Function calls
syn match snailFunctionCall "\h\w*\ze\s*("

" Method calls after dot
syn match snailMethod "\.\h\w*\ze\s*(" contains=snailDot
syn match snailDot "\." contained

" =============================================================================
" SNAIL-SPECIFIC: SPECIAL VARIABLES
" (from grammar: exception_var, field_index_var, injected_var)
" =============================================================================
" Exception variable
syn match snailExceptionVar "\$e\>"

" AWK field index variables: $0, $1, $2, etc.
syn match snailFieldVar "\$\d\+"

" Injected AWK variables: $l, $f, $n, $fn, $p, $m
syn match snailInjectedVar "\$fn\>"
syn match snailInjectedVar "\$[lfnpm]\>"

" =============================================================================
" SNAIL-SPECIFIC: SUBPROCESS SYNTAX
" (from grammar: subprocess_capture, subprocess_status)
" =============================================================================
" Subprocess capture: $(command)
syn region snailSubprocessCapture matchgroup=snailSubprocessDelim start=+\$(+ end=+)+ contains=snailSubprocessInterpolation,snailSubprocessText
syn match snailSubprocessText "[^{}()]\+" contained
syn region snailSubprocessInterpolation matchgroup=snailInterpolationDelim start=+{+ end=+}+ contained contains=TOP

" Subprocess status: @(command)
syn region snailSubprocessStatus matchgroup=snailSubprocessDelim start=+@(+ end=+)+ contains=snailSubprocessInterpolation,snailSubprocessText

" =============================================================================
" SNAIL-SPECIFIC: STRUCTURED ACCESSOR
" (from grammar: structured_accessor = { "$[" ~ structured_query_body ~ "]" })
" =============================================================================
syn region snailStructuredAccessor matchgroup=snailStructuredDelim start=+\$\[+ end=+\]+ contains=snailStructuredNested
syn region snailStructuredNested start=+\[+ end=+\]+ contained contains=snailStructuredNested

" =============================================================================
" SNAIL-SPECIFIC: COMPACT TRY OPERATOR
" (from grammar: try_suffix = { "?" ~ (!add_op ~ try_fallback)? })
" =============================================================================
syn match snailTryOperator "?"

" =============================================================================
" OPERATORS
" =============================================================================
" Comparison operators
syn match snailOperator "=="
syn match snailOperator "!="
syn match snailOperator "<="
syn match snailOperator ">="
syn match snailOperator "<"
syn match snailOperator ">"

" Arithmetic operators
syn match snailOperator "+"
syn match snailOperator "-"
syn match snailOperator "\*\*"
syn match snailOperator "\*"
syn match snailOperator "//"
syn match snailOperator "/"
syn match snailOperator "%"

" Pipeline operator
syn match snailPipelineOp "|"

" Assignment
syn match snailOperator "="

" =============================================================================
" DELIMITERS AND BRACKETS
" =============================================================================
syn match snailDelimiter "[;,:]"
syn match snailBracket "[\[\](){}]"

" =============================================================================
" PYTHON BUILTINS (commonly used in Snail)
" =============================================================================
syn keyword snailBuiltin print len range str int float list dict set tuple
syn keyword snailBuiltin bool type isinstance issubclass hasattr getattr setattr
syn keyword snailBuiltin delattr callable repr hash id
syn keyword snailBuiltin open input abs min max sum sorted reversed enumerate
syn keyword snailBuiltin zip map filter any all next iter
syn keyword snailBuiltin super object staticmethod classmethod property
syn keyword snailBuiltin Exception ValueError TypeError RuntimeError KeyError
syn keyword snailBuiltin IndexError AttributeError ImportError OSError IOError
syn keyword snailBuiltin json re sys os math

" =============================================================================
" FOLDING
" =============================================================================
syn region snailFold start="{" end="}" transparent fold

" =============================================================================
" HIGHLIGHT LINKING
" =============================================================================
" Comments
hi def link snailComment Comment
hi def link snailTodo Todo

" Strings
hi def link snailString String
hi def link snailTripleString String
hi def link snailRawString String
hi def link snailRawTripleString String
hi def link snailEscape SpecialChar
hi def link snailEscapedBrace SpecialChar
hi def link snailInterpolationDelim Special

" Numbers
hi def link snailNumber Number
hi def link snailFloat Float

" Regex
hi def link snailRegex String
hi def link snailRegexEscape SpecialChar
hi def link snailRegexClass Special

" Keywords
hi def link snailConditional Conditional
hi def link snailRepeat Repeat
hi def link snailStatement Statement
hi def link snailException Exception
hi def link snailDefine Define
hi def link snailOperatorWord Operator
hi def link snailImport Include
hi def link snailAwkKeyword PreProc

" Constants
hi def link snailBoolean Boolean
hi def link snailNone Constant

" Functions and classes
hi def link snailFunction Function
hi def link snailClass Type
hi def link snailFunctionCall Function
hi def link snailMethod Function

" Snail-specific: special variables
hi def link snailExceptionVar Identifier
hi def link snailFieldVar Identifier
hi def link snailInjectedVar Identifier

" Snail-specific: subprocess
hi def link snailSubprocessCapture PreProc
hi def link snailSubprocessStatus PreProc
hi def link snailSubprocessDelim Special
hi def link snailSubprocessText String

" Snail-specific: structured accessor
hi def link snailStructuredAccessor PreProc
hi def link snailStructuredDelim Special

" Snail-specific: try operator
hi def link snailTryOperator Special

" Operators and delimiters
hi def link snailOperator Operator
hi def link snailPipelineOp Special
hi def link snailDelimiter Delimiter
hi def link snailBracket Delimiter
hi def link snailDot Delimiter

" Builtins
hi def link snailBuiltin Function

" =============================================================================
" FINAL
" =============================================================================
let b:current_syntax = "snail"

let &cpo = s:save_cpo
unlet s:save_cpo
