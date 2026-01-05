; Snail Tree-sitter highlight queries

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
(float) @number.float

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
] @punctuation.bracket

; Functions
(def_stmt
  name: (identifier) @function)

(class_stmt
  name: (identifier) @type)

(call
  function: (identifier) @function.call)

(call
  function: (attribute
    attribute: (identifier) @function.method))

; Parameters
(regular_param
  name: (identifier) @variable.parameter)
(star_param
  name: (identifier) @variable.parameter)
(kw_param
  name: (identifier) @variable.parameter)

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
(attribute
  attribute: (identifier) @property)

