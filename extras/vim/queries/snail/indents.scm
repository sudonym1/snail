; Snail Tree-sitter indent queries

[
  (block)
  (if_stmt)
  (while_stmt)
  (for_stmt)
  (def_stmt)
  (class_stmt)
  (try_stmt)
  (with_stmt)
  (list_literal)
  (set_literal)
  (dict_literal)
  (tuple_literal)
] @indent.begin

[
  "}"
  "]"
  ")"
] @indent.end

[
  "elif"
  "else"
  "except"
  "finally"
] @indent.branch
