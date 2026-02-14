#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryExpectation {
    MustContinue,
    MaySeparate,
    MustFail,
}

#[derive(Debug, Clone, Copy)]
pub struct WsCase {
    pub name: &'static str,
    pub base: &'static str,
    pub variants: &'static [&'static str],
    pub expectation: BoundaryExpectation,
}

pub const SESSION_CORPUS_CASES: &[WsCase] = &[
    WsCase {
        name: "session_print_paren_continuation",
        base: "print(1)\n",
        variants: &["print(\n1)\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_for_header_newlines",
        base: "for x in range(1) { }\n",
        variants: &["for\nx\nin\nrange(1) { }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_infix_operator_newlines",
        base: "1 + 1\n",
        variants: &["1\n+\n1\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_mixed_expression_continuations",
        base: "call_value = print(1)\nparen_value = (1)\nlist_value = [1, 2]\ndict_value = %{\"a\": 1, \"b\": 2}\nsum_value = 1 + 2\nassigned = 3\n",
        variants: &[
            "call_value = print(\n1\n)\nparen_value = (\n1\n)\nlist_value = [1,\n2]\ndict_value = %{\"a\": 1,\n\"b\": 2}\nsum_value = 1 +\n2\nassigned =\n3\n",
        ],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_return_newline_continuation",
        base: "def ret() { return 1 }\n",
        variants: &["def ret() { return\n1 }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_raise_from_newline_continuation",
        base: "def boom() { raise ValueError(\"bad\") from err }\n",
        variants: &["def boom() { raise\nValueError(\"bad\")\nfrom\nerr }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_attribute_access_newline_before_dot",
        base: "value = obj.attr\n",
        variants: &["value = obj\n.attr\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_attribute_assignment_newline_before_dot",
        base: "obj.attr = 1\n",
        variants: &["obj\n.attr = 1\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_attribute_access_newline_after_dot",
        base: "value = obj.attr\n",
        variants: &["value = obj.\nattr\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_attribute_assignment_newline_after_dot",
        base: "obj.attr = 1\n",
        variants: &["obj.\nattr = 1\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_multiline_if_while_with_headers",
        base: "if True { pass }\nwhile False { pass }\nwith ctx { pass }\n",
        variants: &["if\nTrue\n{ pass }\nwhile\nFalse\n{ pass }\nwith\nctx\n{ pass }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_multiline_def_class_try_headers",
        base: "def foo() { pass }\nclass C { pass }\ntry { pass }\nexcept Exception { pass }\nfinally { pass }\n",
        variants: &[
            "def\nfoo\n()\n{ pass }\nclass\nC\n{ pass }\ntry\n{ pass }\nexcept\nException\n{ pass }\nfinally\n{ pass }\n",
        ],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_except_as_newline_continuation",
        base: "try { pass }\nexcept Exception as e { pass }\n",
        variants: &["try { pass }\nexcept Exception\nas\ne\n{ pass }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_assert_del_import_from_import_newlines",
        base: "assert True, \"ok\"\ndel items[0]\nimport os\nfrom os import path\n",
        variants: &["assert\nTrue\n,\n\"ok\"\ndel\nitems[0]\nimport\nos\nfrom\nos\nimport\npath\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_destructuring_assignment_and_for_target_newlines",
        base: "x, y = [1, 2]\n[a, b] = pair\nx, *rest = values\nfor x, y in [(1, 2)] { pass }\nfor [a, b] in [[1, 2]] { pass }\n",
        variants: &[
            "x,\ny = [1, 2]\n[a,\nb] = pair\nx,\n*rest = values\nfor\nx,\ny\nin\n[(1, 2)]\n{ pass }\nfor\n[a,\nb]\nin\n[[1, 2]]\n{ pass }\n",
        ],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "session_assignment_boundary_does_not_continue",
        base: "x = 1\n",
        variants: &["x\n= 1\n"],
        expectation: BoundaryExpectation::MustFail,
    },
    WsCase {
        name: "session_newline_before_call_paren_may_separate",
        base: "x(1)\n",
        variants: &["x\n(1)\n"],
        expectation: BoundaryExpectation::MaySeparate,
    },
    WsCase {
        name: "session_simple_statement_separator_required",
        base: "a; b\n",
        variants: &["a b\n"],
        expectation: BoundaryExpectation::MustFail,
    },
];

pub const STATEMENT_MATRIX_CASES: &[WsCase] = &[
    WsCase {
        name: "statement_if_elif_else_keyword_splits",
        base: "if cond { pass }\nelif other { pass }\nelse { pass }\n",
        variants: &["if\ncond\n{ pass }\nelif\nother\n{ pass }\nelse\n{ pass }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_while_else_keyword_splits",
        base: "while flag { pass }\nelse { pass }\n",
        variants: &["while\nflag\n{ pass }\nelse\n{ pass }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_for_else_and_in_splits",
        base: "for item in items { pass }\nelse { pass }\n",
        variants: &["for\nitem\nin\nitems\n{ pass }\nelse\n{ pass }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_if_let_target_and_equals_splits",
        base: "if let [lhs, rhs] = pair; lhs { pass }\n",
        variants: &["if let\n[lhs,\nrhs]\n=\npair;\nlhs\n{ pass }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_while_let_target_equals_guard_splits",
        base: "while let value = next(); value { pass }\n",
        variants: &["while let\nvalue\n=\nnext();\nvalue\n{ pass }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_def_header_with_param_default_splits",
        base: "def add(a, b=1) { return a + b }\n",
        variants: &["def\nadd\n(\na,\nb\n=\n1\n)\n{ return a + b }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_class_header_splits",
        base: "class Bucket { pass }\n",
        variants: &["class\nBucket\n{ pass }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_try_except_else_finally_splits",
        base: "try { pass }\nexcept Exception as err { pass }\nelse { pass }\nfinally { pass }\n",
        variants: &[
            "try\n{ pass }\nexcept\nException\nas\nerr\n{ pass }\nelse\n{ pass }\nfinally\n{ pass }\n",
        ],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_with_as_splits",
        base: "with open(\"data\") as handle { pass }\n",
        variants: &["with\nopen(\"data\")\nas\nhandle\n{ pass }\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_simple_stmt_splits",
        base: "def ret() { return 1 }\ndef boom(err) { raise ValueError(\"bad\") from err }\nassert True, \"ok\"\ndel items[0]\nimport os\nfrom os import path\n",
        variants: &[
            "def ret() { return\n1 }\ndef boom(err) { raise\nValueError(\"bad\")\nfrom\nerr }\nassert\nTrue\n,\n\"ok\"\ndel\nitems[0]\nimport\nos\nfrom\nos\nimport\npath\n",
        ],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_destructuring_target_splits",
        base: "x, y = [1, 2]\n[a, b] = pair\nx, *rest = values\nfor x, y in [(1, 2)] { pass }\nfor [a, b] in [[1, 2]] { pass }\n",
        variants: &[
            "x,\ny = [1, 2]\n[a,\nb] = pair\nx,\n*rest = values\nfor\nx,\ny\nin\n[(1, 2)]\n{ pass }\nfor\n[a,\nb]\nin\n[[1, 2]]\n{ pass }\n",
        ],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_dict_colon_separator_splits",
        base: "mapping = %{\"a\": 1, \"b\": 2}\n",
        variants: &["mapping = %{\"a\"\n:\n1,\n\"b\"\n:\n2}\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_infix_and_unary_operator_splits",
        base: "value = -x + y\n",
        variants: &["value = -x\n+\ny\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_postfix_attribute_and_inner_index_splits",
        base: "value = obj.attr[0]\n",
        variants: &["value = obj\n.attr[\n0]\n"],
        expectation: BoundaryExpectation::MustContinue,
    },
    WsCase {
        name: "statement_newline_before_index_bracket_may_separate",
        base: "value = obj.attr[0]\n",
        variants: &["value = obj.attr\n[0]\n"],
        expectation: BoundaryExpectation::MaySeparate,
    },
    WsCase {
        name: "statement_newline_before_call_paren_may_separate",
        base: "target(1)\n",
        variants: &["target\n(1)\n"],
        expectation: BoundaryExpectation::MaySeparate,
    },
    WsCase {
        name: "statement_newline_before_equals_must_fail",
        base: "item = 1\n",
        variants: &["item\n= 1\n"],
        expectation: BoundaryExpectation::MustFail,
    },
];

pub const EXTENDED_CASES: &[WsCase] = &[];

pub fn selected_cases() -> Vec<&'static WsCase> {
    let mut cases = Vec::with_capacity(SESSION_CORPUS_CASES.len() + STATEMENT_MATRIX_CASES.len());
    cases.extend(SESSION_CORPUS_CASES.iter());
    cases.extend(STATEMENT_MATRIX_CASES.iter());
    if std::env::var_os("SNAIL_WHITESPACE_EXTENDED").is_some() {
        cases.extend(EXTENDED_CASES.iter());
    }
    cases
}
