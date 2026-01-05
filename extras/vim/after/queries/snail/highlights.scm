; Snail Tree-sitter highlight queries for Neovim

; Comments
(comment) @comment

; Strings
(string) @string
(triple_string) @string
(raw_string) @string
(raw_triple_string) @string
(interpolation) @embedded
(escape_sequence) @string.escape

; Regex
(regex) @string.regex

; Numbers
(number) @number

; Booleans and None
(boolean) @boolean
(none) @constant.builtin

; Keywords
["if" "elif" "else"] @conditional
["for" "while"] @repeat
["try" "except" "finally" "raise"] @exception
["def" "class"] @keyword.function
["import" "from" "as"] @include
["return" "break" "continue" "pass" "assert" "del" "with"] @keyword
["and" "or" "not" "in" "is"] @keyword.operator
["BEGIN" "END"] @preproc

; Operators
["+" "-" "*" "/" "//" "%" "**" "==" "!=" "<" ">" "<=" ">=" "="] @operator
"|" @operator
"?" @operator

; Delimiters
[";" "," ":" "."] @punctuation.delimiter
["(" ")" "[" "]" "{" "}"] @punctuation.bracket

; Functions and classes
(def_stmt name: (identifier) @function)
(class_stmt name: (identifier) @type)
(call function: (identifier) @function.call)

; Parameters
(regular_param name: (identifier) @parameter)
(star_param name: (identifier) @parameter)
(kw_param name: (identifier) @parameter)

; Variables
(identifier) @variable
(exception_var) @variable.builtin
(injected_var) @variable.builtin
(field_index_var) @variable.builtin

; Subprocess and structured accessor
(subprocess_capture) @function.macro
(subprocess_status) @function.macro
(structured_accessor) @function.macro

; Attributes
(attribute attribute: (identifier) @field)

