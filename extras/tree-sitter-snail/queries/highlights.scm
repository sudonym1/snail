; Snail Tree-sitter highlight queries

; Comments
(comment) @comment

; Line continuation
(line_continuation) @punctuation.special

; Strings
(double_string) @string
(single_string) @string
(triple_double_string) @string
(triple_single_string) @string
(raw_double_string) @string
(raw_single_string) @string
(raw_triple_double_string) @string
(raw_triple_single_string) @string
(string_interpolation) @embedded
(escape_sequence) @string.escape
(raw_prefix) @string.special
(byte_prefix) @string.special

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

(let_cond "let" @keyword.special)
(let_guard) @keyword.special

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
  "$("
  "@("
  "["
  "]"
  "$["
  "{"
  "}"
  "%{"
  "#{"
] @punctuation.bracket

; Functions and classes
(def_stmt name: (identifier) @function.definition (#set! "priority" 110))
(class_stmt name: (identifier) @type.definition (#set! "priority" 110))

; Methods (definitions inside classes)
(class_stmt
  body: (block
    (stmt_list
      (def_stmt name: (identifier) @function.method.definition (#set! "priority" 120))
    )
  )
)


; Parameters
(regular_param (identifier) @variable.parameter (#set! "priority" 120))
(star_param (identifier) @variable.parameter (#set! "priority" 120))
(kw_param (identifier) @variable.parameter (#set! "priority" 120))

; Method self parameter
(class_stmt
  body: (block
    (stmt_list
      (def_stmt
        parameters: (parameters
          (param_list
            (regular_param (identifier) @variable.builtin (#eq? @variable.builtin "self") (#set! "priority" 130))
          )
        )
      )
    )
  )
)

; Variables
(identifier) @variable

; Imports
(import_from (dotted_name) @module (#set! "priority" 105))
(import_item (dotted_name) @module (#set! "priority" 105))

; Function calls
; Call target is the primary identifier or attribute before (call)
(primary (identifier) @function.call (call) (#set! "priority" 110))
(primary (identifier) (attribute (identifier) @function.method.call) (call) (#set! "priority" 110))
(primary (identifier) (call) (attribute (identifier) @function.method.call) (call) (#set! "priority" 110))

; Special Snail variables
(exception_var) @constant.builtin (#set! "priority" 120)
(injected_var) @constant.builtin (#set! "priority" 120)
(field_index_var) @constant.builtin (#set! "priority" 120)

; Subprocess
(subprocess_capture) @function.macro
(subprocess_status) @function.macro

; Structured accessor
(structured_accessor) @function.macro

; Attributes
(attribute (identifier) @property)
[ (identifier) @keyword.directive (#eq? @keyword.directive "BEGIN") ]
[ (identifier) @keyword.directive (#eq? @keyword.directive "END") ]
