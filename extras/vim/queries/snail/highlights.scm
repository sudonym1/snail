; Snail Tree-sitter highlight queries

; Comments
(comment) @comment

; Strings
(double_string) @string
(single_string) @string
(triple_double_string) @string
(triple_single_string) @string
(string_interpolation) @embedded
(escape_sequence) @string.escape
(raw_prefix) @string.special

; Regex
(regex) @string.regex

; Numbers
(number) @number

; Booleans and None
(boolean) @boolean
(none) @constant.builtin

; Keywords
[
  "if"
  "elif"
  "else"
] @keyword.conditional

[
  "for"
  "while"
] @keyword.repeat

[
  "try"
  "except"
  "finally"
  "raise"
] @keyword.exception

[
  "def"
  "class"
] @keyword.function

[
  "import"
  "from"
  "as"
] @keyword.import

[
  "return"
  "break"
  "continue"
  "pass"
  "assert"
  "del"
  "with"
] @keyword

[
  "and"
  "or"
  "not"
  "in"
  "is"
] @keyword.operator

; AWK keywords
[
  "BEGIN"
  "END"
] @keyword.directive

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "//"
  "%"
  "**"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "="
] @operator

; Pipeline operator
"|" @operator.pipeline

; Try operator
"?" @operator.special

; Delimiters
[
  ";"
  ","
  ":"
  "."
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "%{"
  "#{"
] @punctuation.bracket

; Functions and classes
(def_stmt name: (identifier) @function)
(class_stmt name: (identifier) @type)

; Function calls
; Note: We match the func field of call which is a primary expression
(call) @function.call

; Parameters
(regular_param (identifier) @variable.parameter)
(star_param (identifier) @variable.parameter)
(kw_param (identifier) @variable.parameter)

; Variables
(identifier) @variable

; Special Snail variables
(exception_var) @variable.builtin
(injected_var) @variable.builtin
(field_index_var) @variable.builtin

; Subprocess
(subprocess_capture) @function.macro
(subprocess_status) @function.macro

; Structured accessor
(structured_accessor) @function.macro

; Attributes
(attribute (identifier) @property)
