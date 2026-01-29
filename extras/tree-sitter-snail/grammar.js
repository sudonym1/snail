/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

/**
 * Tree-sitter grammar for the Snail programming language
 * Based on the Pest grammar at src/snail.pest
 */

const PREC = {
  // From lowest to highest precedence
  CONDITIONAL: 1,      // if/else ternary
  OR: 2,               // or
  AND: 3,              // and
  NOT: 4,              // not
  PIPELINE: 5,         // |
  COMPARE: 6,          // == != < > <= >= in is
  ADD: 7,              // + -
  MUL: 8,              // * / // %
  UNARY: 9,            // unary + -
  POWER: 10,           // **
  TRY: 11,             // ? compact try
  CALL: 12,            // function call, attribute, index
};

const SET_START = token(prec(1, '#{'));
const DICT_START = token(prec(1, '%{'));

module.exports = grammar({
  name: 'snail',

  extras: $ => [
    /[ \t]/,           // whitespace (not newlines)
    $.comment,
  ],

  externals: $ => [],

  conflicts: $ => [
    [$.primary],
    [$._atom, $.tuple_literal],
    [$._stmt_sep],
    [$.assign_target, $._atom],
    [$.stmt_list],
    [$.program],
    [$.if_stmt],
    [$.while_stmt],
    [$.for_stmt],
    [$.try_stmt],
    [$._try_except_tail],
    [$._try_finally_tail],
    [$.awk_entry_list],
    [$.awk_program],
    [$.block],
  ],

  word: $ => $.identifier,

  rules: {
    // Top-level program entry point
    program: $ => seq(
      repeat($._stmt_sep),
      optional($.stmt_list),
      repeat($._stmt_sep),
    ),

    // AWK program entry point
    awk_program: $ => seq(
      repeat($._stmt_sep),
      optional($.awk_entry_list),
      repeat($._stmt_sep),
    ),

    // AWK mode structures
    awk_entry_list: $ => seq(
      $.awk_entry,
      repeat(seq(repeat($._stmt_sep), $.awk_entry)),
      repeat($._stmt_sep),
    ),

    awk_entry: $ => choice(
      $.awk_begin,
      $.awk_end,
      $.awk_rule,
    ),

    awk_begin: $ => seq('BEGIN', $.block),
    awk_end: $ => seq('END', $.block),
    awk_rule: $ => choice(
      $.block,
      seq($.awk_pattern, optional($.block)),
    ),
    awk_pattern: $ => $._expr,

    // Statement list
    stmt_list: $ => seq(
      $._stmt,
      repeat(seq($._stmt_sep, $._stmt)),
      optional($._stmt_sep),
    ),

    _stmt_sep: $ => choice(
      seq(';', repeat($._newline)),
      repeat1($._newline),
    ),

    _newline: $ => /\r?\n/,

    // Statements
    _stmt: $ => choice(
      $._compound_stmt,
      $._simple_stmt,
    ),

    _compound_stmt: $ => choice(
      $.if_stmt,
      $.while_stmt,
      $.for_stmt,
      $.def_stmt,
      $.class_stmt,
      $.try_stmt,
      $.with_stmt,
    ),

    _simple_stmt: $ => choice(
      $.return_stmt,
      $.break_stmt,
      $.continue_stmt,
      $.pass_stmt,
      $.raise_stmt,
      $.assert_stmt,
      $.del_stmt,
      $.import_from,
      $.import_names,
      $.assign_stmt,
      $.expr_stmt,
    ),

    // Block: braced statements
    block: $ => seq(
      '{',
      repeat($._stmt_sep),
      optional($.stmt_list),
      repeat($._stmt_sep),
      '}',
    ),

    // Control flow statements
    if_stmt: $ => seq(
      'if',
      field('condition', $._expr),
      field('consequence', $.block),
      repeat(seq(repeat($._stmt_sep), $.elif_clause)),
      optional(seq(repeat($._stmt_sep), $.else_clause)),
    ),

    elif_clause: $ => seq(
      'elif',
      field('condition', $._expr),
      field('consequence', $.block),
    ),

    while_stmt: $ => seq(
      'while',
      field('condition', $._expr),
      field('body', $.block),
      optional(seq(repeat($._stmt_sep), $.else_clause)),
    ),

    for_stmt: $ => seq(
      'for',
      field('variable', $.identifier),
      'in',
      field('iterable', $._expr),
      field('body', $.block),
      optional(seq(repeat($._stmt_sep), $.else_clause)),
    ),

    else_clause: $ => seq('else', $.block),

    // Function and class definitions
    def_stmt: $ => seq(
      'def',
      field('name', $.identifier),
      field('parameters', $.parameters),
      field('body', $.block),
    ),

    class_stmt: $ => seq(
      'class',
      field('name', $.identifier),
      field('body', $.block),
    ),

    // Exception handling
    try_stmt: $ => seq(
      'try',
      $.block,
      choice(
        $._try_except_tail,
        $._try_finally_tail,
      ),
    ),

    _try_except_tail: $ => seq(
      repeat1(seq(repeat($._stmt_sep), $.except_clause)),
      optional(seq(repeat($._stmt_sep), $.try_else_clause)),
      optional(seq(repeat($._stmt_sep), $.finally_clause)),
    ),

    _try_finally_tail: $ => seq(
      repeat($._stmt_sep),
      $.finally_clause,
    ),

    except_clause: $ => seq(
      'except',
      optional(seq(
        $._expr,
        optional(seq('as', $.identifier)),
      )),
      $.block,
    ),

    try_else_clause: $ => seq('else', $.block),
    finally_clause: $ => seq('finally', $.block),

    // Context managers
    with_stmt: $ => seq(
      'with',
      $.with_items,
      $.block,
    ),

    with_items: $ => seq(
      $.with_item,
      repeat(seq(',', $.with_item)),
    ),

    with_item: $ => seq(
      $._expr,
      optional(seq('as', $.assign_target)),
    ),

    // Function parameters
    parameters: $ => seq(
      '(',
      optional($.param_list),
      ')',
    ),

    param_list: $ => seq(
      $._parameter,
      repeat(seq(',', $._parameter)),
      optional(','),
    ),

    _parameter: $ => choice(
      $.regular_param,
      $.star_param,
      $.kw_param,
    ),

    regular_param: $ => seq(
      $.identifier,
      optional(seq('=', $._expr)),
    ),

    star_param: $ => seq('*', $.identifier),
    kw_param: $ => seq('**', $.identifier),

    // Simple statements
    return_stmt: $ => seq('return', optional($._expr)),
    raise_stmt: $ => seq('raise', optional(seq($._expr, optional(seq('from', $._expr))))),
    assert_stmt: $ => seq('assert', $._expr, optional(seq(',', $._expr))),
    del_stmt: $ => seq('del', $.assign_target, repeat(seq(',', $.assign_target))),
    break_stmt: $ => 'break',
    continue_stmt: $ => 'continue',
    pass_stmt: $ => 'pass',

    // Import statements
    import_from: $ => seq(
      'from',
      $.dotted_name,
      'import',
      $.import_items,
    ),

    import_names: $ => seq(
      'import',
      $.import_items,
    ),

    import_items: $ => seq(
      $.import_item,
      repeat(seq(',', $.import_item)),
    ),

    import_item: $ => seq(
      $.dotted_name,
      optional(seq('as', $.identifier)),
    ),

    dotted_name: $ => seq(
      $.identifier,
      repeat(seq('.', $.identifier)),
    ),

    // Assignment and expression statements
    assign_stmt: $ => seq(
      $.assign_target,
      '=',
      $._expr,
    ),

    assign_target: $ => seq(
      $.identifier,
      repeat(choice($.attribute, $.index)),
    ),

    expr_stmt: $ => $._expr,

    // Expressions with precedence
    _expr: $ => $.if_expr,

    // Ternary conditional: value if condition else alternative
    if_expr: $ => choice(
      prec.right(PREC.CONDITIONAL, seq(
        field('value', $.or_expr),
        'if',
        field('condition', $.or_expr),
        'else',
        field('alternative', $.if_expr),
      )),
      $.or_expr,
    ),

    // Boolean operators
    or_expr: $ => choice(
      prec.left(PREC.OR, seq($.or_expr, 'or', $.and_expr)),
      $.and_expr,
    ),

    and_expr: $ => choice(
      prec.left(PREC.AND, seq($.and_expr, 'and', $.not_expr)),
      $.not_expr,
    ),

    not_expr: $ => choice(
      prec(PREC.NOT, seq('not', $.not_expr)),
      $.pipeline,
    ),

    // Pipeline operator
    pipeline: $ => choice(
      prec.left(PREC.PIPELINE, seq($.pipeline, '|', $.comparison)),
      $.comparison,
    ),

    // Comparison operators
    comparison: $ => choice(
      prec.left(PREC.COMPARE, seq($.comparison, $.comp_op, $.sum)),
      $.sum,
    ),

    comp_op: $ => choice(
      '==',
      '!=',
      '<=',
      '>=',
      '<',
      '>',
      'in',
      'is',
    ),

    // Arithmetic operators
    sum: $ => choice(
      prec.left(PREC.ADD, seq($.sum, $.add_op, $.product)),
      $.product,
    ),

    add_op: $ => choice('+', '-'),

    product: $ => choice(
      prec.left(PREC.MUL, seq($.product, $.mul_op, $.unary)),
      $.unary,
    ),

    mul_op: $ => choice('//', '*', '/', '%'),

    unary: $ => choice(
      prec(PREC.UNARY, seq($.unary_op, $.unary)),
      $.power,
    ),

    unary_op: $ => choice('+', '-'),

    power: $ => choice(
      prec.right(PREC.POWER, seq($.primary, '**', $.power)),
      $.primary,
    ),

    // Primary expressions: atoms with postfix operations
    primary: $ => prec.left(PREC.CALL, seq(
      $._atom,
      repeat(choice(
        $.call,
        $.attribute,
        $.index,
        $.try_suffix,
      )),
    )),

    // Compact try operator: expr? or expr:fallback?
    try_suffix: $ => prec(PREC.TRY, seq(
      optional($.try_fallback),
      '?',
    )),

    try_fallback: $ => seq(':', $.try_fallback_unary),

    // Fallback expression grammar (mirrors unary/power/primary but without try_suffix)
    try_fallback_unary: $ => choice(
      prec(PREC.UNARY, seq($.unary_op, $.try_fallback_unary)),
      $.try_fallback_power,
    ),

    try_fallback_power: $ => choice(
      prec.right(PREC.POWER, seq($.try_fallback_primary, '**', $.try_fallback_power)),
      $.try_fallback_primary,
    ),

    try_fallback_primary: $ => prec.left(PREC.CALL, seq(
      $._atom,
      repeat(choice(
        $.call,
        $.attribute,
        $.index,
      )),
    )),

    // Function calls
    call: $ => seq(
      '(',
      optional(seq(
        $.argument,
        repeat(seq(',', $.argument)),
      )),
      ')',
    ),

    argument: $ => choice(
      $.kw_argument,
      $.star_arg,
      $.kw_star_arg,
      $._expr,
    ),

    kw_argument: $ => seq(
      $.identifier,
      '=',
      $._expr,
    ),

    star_arg: $ => seq('*', $._expr),
    kw_star_arg: $ => seq('**', $._expr),

    // Attribute access and indexing
    attribute: $ => seq('.', $.identifier),

    index: $ => seq('[', $.slice, ']'),

    slice: $ => choice(
      $.slice_expr,
      $._expr,
    ),

    slice_expr: $ => seq(
      optional(field('start', $.slice_start)),
      ':',
      optional(field('end', $.slice_end)),
    ),

    slice_start: $ => $._expr,
    slice_end: $ => $._expr,

    // Atomic expressions
    _atom: $ => choice(
      $._literal,
      $.regex,
      $.subprocess,
      $.structured_accessor,
      $.exception_var,
      $.field_index_var,
      $.injected_var,
      $.identifier,
      $.list_comp,
      $.list_literal,
      $.set_literal,
      $.dict_comp,
      $.dict_literal,
      $.tuple_literal,
      $.compound_expr,
      $.parenthesized_expr,
    ),

    parenthesized_expr: $ => seq('(', $._expr, ')'),

    // Compound expression: multiple semicolon-separated expressions
    compound_expr: $ => seq(
      '(',
      repeat($._newline),
      $._expr,
      repeat1(seq(';', repeat($._newline), $._expr)),
      repeat($._newline),
      optional(';'),
      ')',
    ),

    // Subprocess invocation
    subprocess: $ => choice(
      $.subprocess_capture,
      $.subprocess_status,
    ),

    subprocess_capture: $ => seq(
      '$(',
      repeat(choice(
        $.subprocess_expr,
        $.subprocess_text,
      )),
      ')',
    ),

    subprocess_status: $ => seq(
      '@(',
      repeat(choice(
        $.subprocess_expr,
        $.subprocess_text,
      )),
      ')',
    ),

    subprocess_expr: $ => seq('{', $._expr, '}'),

    subprocess_text: $ => /(\{\{|\}\}|[^{})])+/,

    // Structured pipeline accessor
    structured_accessor: $ => seq(
      '$[',
      optional($.structured_query_body),
      ']',
    ),

    structured_query_body: $ => repeat1(choice(
      seq('[', optional($.structured_query_body), ']'),
      /[^\[\]]+/,
    )),

    // Collection literals
    tuple_literal: $ => choice(
      seq('(', ')'),
      seq(
        '(',
        $._expr,
        ',',
        optional(seq(
          $._expr,
          repeat(seq(',', $._expr)),
          optional(','),
        )),
        ')',
      ),
    ),

    list_comp: $ => seq(
      '[',
      $._expr,
      $.comp_for,
      ']',
    ),

    list_literal: $ => seq(
      '[',
      optional(seq(
        $._expr,
        repeat(seq(',', $._expr)),
      )),
      ']',
    ),

    set_literal: $ => seq(
      SET_START,
      optional(seq(
        $._expr,
        repeat(seq(',', $._expr)),
      )),
      '}',
    ),

    dict_comp: $ => seq(
      DICT_START,
      $._expr,
      ':',
      $._expr,
      $.comp_for,
      '}',
    ),

    dict_literal: $ => seq(
      DICT_START,
      optional(seq(
        $.dict_entry,
        repeat(seq(',', $.dict_entry)),
      )),
      '}',
    ),

    dict_entry: $ => seq($._expr, ':', $._expr),

    // Comprehension clauses
    comp_for: $ => seq(
      'for',
      $.identifier,
      'in',
      $._expr,
      repeat($.comp_if),
    ),

    comp_if: $ => seq('if', $._expr),

    // Literals
    _literal: $ => choice(
      $.number,
      $.string,
      $.boolean,
      $.none,
    ),

    boolean: $ => choice('True', 'False'),
    none: $ => 'None',

    // Special variables
    exception_var: $ => '$e',

    field_index_var: $ => /\$[0-9]+/,

    injected_var: $ => choice(
      '$fn',  // Must come before $n
      '$n',
      '$p',
      '$m',
    ),

    // Number literals
    number: $ => /[0-9]+(\.[0-9]+)?/,

    // String literals with optional raw prefix
    string: $ => seq(
      optional($.raw_prefix),
      choice(
        $.triple_double_string,
        $.triple_single_string,
        $.double_string,
        $.single_string,
      ),
    ),

    raw_prefix: $ => 'r',

    triple_double_string: $ => seq(
      '"""',
      repeat(choice(
        $.string_interpolation,
        $.escape_sequence,
        $.triple_double_char,
      )),
      '"""',
    ),

    triple_single_string: $ => seq(
      "'''",
      repeat(choice(
        $.string_interpolation,
        $.escape_sequence,
        $.triple_single_char,
      )),
      "'''",
    ),

    triple_double_char: $ => choice(
      /[^"\\{]+/,
      seq('"', /[^"]/),  // Allow single " that's not part of """
      seq('"', '"', /[^"]/),  // Allow "" that's not part of """
    ),

    triple_single_char: $ => choice(
      /[^'\\{]+/,
      seq("'", /[^']/),  // Allow single ' that's not part of '''
      seq("'", "'", /[^']/),  // Allow '' that's not part of '''
    ),

    double_string: $ => seq(
      '"',
      repeat(choice(
        $.string_interpolation,
        $.escape_sequence,
        $.double_string_char,
      )),
      '"',
    ),

    single_string: $ => seq(
      "'",
      repeat(choice(
        $.string_interpolation,
        $.escape_sequence,
        $.single_string_char,
      )),
      "'",
    ),

    double_string_char: $ => /[^"\\{]+/,
    single_string_char: $ => /[^'\\{]+/,

    string_interpolation: $ => seq(
      '{',
      $._expr,
      '}',
    ),

    escape_sequence: $ => /\\./,

    // Regex literals
    regex: $ => seq(
      '/',
      repeat(choice(
        /[^/\\]/,
        /\\./,
      )),
      '/',
    ),

    // Identifiers (not keywords)
    identifier: $ => /[A-Za-z_][A-Za-z0-9_]*/,

    // Comment
    comment: $ => /#[^\r\n]*/,
  },
});
