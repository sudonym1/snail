/// Go-style semicolon injection preprocessor.
///
/// Scans source text and replaces statement-boundary newlines with `\x1e`
/// (ASCII Record Separator). This character is then treated as a statement
/// separator by the Pest grammar, while being invisible to
/// `check_trailing_semicolon` (which only looks for `;`).
use snail_ast::{SourcePos, SourceSpan};
use snail_error::ParseError;

const RS: u8 = 0x1E; // ASCII Record Separator

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BracketKind {
    Block,
    Paren,
    Bracket,
    SetLiteral,
    DictLiteral,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LastToken {
    StmtEnder,
    Continuation,
    None,
}

/// Preprocess Snail source: replace newlines at statement boundaries with \x1e.
/// The returned string has the same byte length as the input.
pub fn preprocess(source: &str) -> Result<String, ParseError> {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let len = bytes.len();

    let mut i = 0;
    let mut bracket_stack: Vec<BracketKind> = Vec::new();
    let mut last_token = LastToken::None;
    let mut in_header = false;
    // Track whether the next keyword token is at the start of a statement
    // (vs. in expression context like a ternary `if`/`else`).
    // True at: SOI, after `{`, after `;`, after injecting `\x1e`.
    let mut at_stmt_start = true;
    // Track the byte that precedes a `{` for #{/% detection
    let mut prev_non_ws_byte: Option<u8> = Option::None;

    while i < len {
        let b = bytes[i];

        // === Skip string literals ===
        if is_string_prefix_start(bytes, i) {
            let start = i;
            // Advance past prefix
            i = skip_string_prefix(bytes, i);
            if i < len && (bytes[i] == b'\'' || bytes[i] == b'"') {
                i = skip_string_body(bytes, i);
                // String literal is a StmtEnder
                last_token = LastToken::StmtEnder;
                at_stmt_start = false;
                // Update prev_non_ws_byte to the last byte of the string
                if i > start {
                    prev_non_ws_byte = Some(bytes[i - 1]);
                }
                continue;
            }
            // Not actually a string, fall through from prefix position
            i = start;
        } else if bytes[i] == b'\'' || bytes[i] == b'"' {
            i = skip_string_body(bytes, i);
            last_token = LastToken::StmtEnder;
            at_stmt_start = false;
            if i > 0 {
                prev_non_ws_byte = Some(bytes[i - 1]);
            }
            continue;
        }

        // === Skip comments ===
        if b == b'#' && !(i + 1 < len && bytes[i + 1] == b'{') {
            // Comment: skip to end of line but DON'T update last_token
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            // Don't update last_token — the token before the comment determines injection
            continue;
        }

        // === Newline handling ===
        if b == b'\n' {
            if should_inject(&bracket_stack, last_token, in_header, bytes, i) {
                out[i] = RS;
                at_stmt_start = true;
            }
            i += 1;
            continue;
        }

        if b == b'\r' {
            // \r\n: handle as a single newline
            if i + 1 < len && bytes[i + 1] == b'\n' {
                if should_inject(&bracket_stack, last_token, in_header, bytes, i + 1) {
                    out[i + 1] = RS;
                    at_stmt_start = true;
                }
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        // === Skip plain whitespace ===
        if b == b' ' || b == b'\t' {
            i += 1;
            continue;
        }

        // === Bracket tracking ===
        match b {
            b'(' => {
                if in_header {
                    // Still in header mode — parens in the header (e.g., `def foo(...)`)
                }
                bracket_stack.push(BracketKind::Paren);
                last_token = LastToken::Continuation;
                at_stmt_start = false;
                prev_non_ws_byte = Some(b);
                i += 1;
                continue;
            }
            b'[' => {
                bracket_stack.push(BracketKind::Bracket);
                last_token = LastToken::Continuation;
                at_stmt_start = false;
                prev_non_ws_byte = Some(b);
                i += 1;
                continue;
            }
            b'{' => {
                let kind = match prev_non_ws_byte {
                    Some(b'#') => BracketKind::SetLiteral,
                    Some(b'%') => BracketKind::DictLiteral,
                    _ => BracketKind::Block,
                };
                bracket_stack.push(kind);
                if in_header && kind == BracketKind::Block {
                    in_header = false;
                }
                last_token = LastToken::Continuation;
                at_stmt_start = kind == BracketKind::Block;
                prev_non_ws_byte = Some(b);
                i += 1;
                continue;
            }
            b')' => {
                pop_matching(&mut bracket_stack, BracketKind::Paren);
                last_token = LastToken::StmtEnder;
                at_stmt_start = false;
                prev_non_ws_byte = Some(b);
                i += 1;
                continue;
            }
            b']' => {
                pop_matching(&mut bracket_stack, BracketKind::Bracket);
                last_token = LastToken::StmtEnder;
                at_stmt_start = false;
                prev_non_ws_byte = Some(b);
                i += 1;
                continue;
            }
            b'}' => {
                pop_matching(&mut bracket_stack, BracketKind::Block);
                last_token = LastToken::StmtEnder;
                at_stmt_start = false;
                prev_non_ws_byte = Some(b);
                i += 1;
                continue;
            }
            _ => {}
        }

        // === Regex literal ===
        if b == b'/' && last_token != LastToken::StmtEnder {
            // Regex: only valid after an operator/continuation or at start
            let end = skip_regex(bytes, i);
            if end > i + 1 {
                // It was a regex
                last_token = LastToken::StmtEnder;
                at_stmt_start = false;
                prev_non_ws_byte = Some(bytes[end - 1]);
                i = end;
                continue;
            }
        }

        // === Operators and punctuation ===
        if let Some((advance, token_kind)) = classify_punctuation(bytes, i) {
            last_token = token_kind;
            // `;` starts a new statement context
            at_stmt_start = bytes[i] == b';';
            prev_non_ws_byte = Some(bytes[i + advance - 1]);
            i += advance;
            continue;
        }

        // === Identifiers and keywords ===
        if is_ident_start(b) {
            let start = i;
            while i < len && is_ident_continue(bytes[i]) {
                i += 1;
            }
            let word = &bytes[start..i];
            let (token_kind, triggers_header) = classify_keyword(word);
            last_token = token_kind;

            if triggers_header && at_stmt_level(&bracket_stack) && at_stmt_start {
                in_header = true;
            }

            at_stmt_start = false;
            prev_non_ws_byte = Some(bytes[i - 1]);
            continue;
        }

        // === Numbers ===
        if b.is_ascii_digit() {
            while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            last_token = LastToken::StmtEnder;
            at_stmt_start = false;
            prev_non_ws_byte = Some(bytes[i - 1]);
            continue;
        }

        // === Dollar variables ($0, $n, $fn, $f, $m, $e, $src, $text, $fd, $env) ===
        if b == b'$' {
            i += 1;
            // $( and $[ are not identifiers — they're subprocess/accessor
            if i < len && (bytes[i] == b'(' || bytes[i] == b'[') {
                last_token = LastToken::Continuation;
                at_stmt_start = false;
                prev_non_ws_byte = Some(b);
                continue;
            }
            while i < len && is_ident_continue(bytes[i]) {
                i += 1;
            }
            // Also handle $0, $1, ... (digit variables)
            last_token = LastToken::StmtEnder;
            at_stmt_start = false;
            prev_non_ws_byte = Some(bytes[i - 1]);
            continue;
        }

        // === Backslash line continuation ===
        // Consume `\`, any trailing whitespace, and the newline.
        // More forgiving than Python (which requires `\` immediately before newline).
        // Only active at statement level — inside brackets/parens, `\` is a regular
        // character (e.g. JMESPath escapes in $[\"key\"]).
        if b == b'\\' && at_stmt_level(&bracket_stack) {
            let mut j = i + 1;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < len && bytes[j] == b'\n' {
                // Replace \ + whitespace + LF with spaces
                for slot in &mut out[i..=j] {
                    *slot = b' ';
                }
                i = j + 1;
                continue;
            } else if j + 1 < len && bytes[j] == b'\r' && bytes[j + 1] == b'\n' {
                // Replace \ + whitespace + CR LF with spaces
                for slot in &mut out[i..=j + 1] {
                    *slot = b' ';
                }
                i = j + 2;
                continue;
            }
            // Stray backslash at statement level — not followed by a newline
            return Err(stray_backslash_error(source, i));
        }

        // === Anything else: skip ===
        prev_non_ws_byte = Some(b);
        i += 1;
    }

    // SAFETY: We only replaced \n (valid ASCII) with \x1e (valid ASCII), so the output is valid UTF-8.
    Ok(String::from_utf8(out).expect("preprocessed output should be valid UTF-8"))
}

fn stray_backslash_error(source: &str, offset: usize) -> ParseError {
    let (line, column) = line_col(source, offset);
    let line_text = source.lines().nth(line - 1).map(|s| s.to_string());
    let span = SourceSpan {
        start: SourcePos {
            offset,
            line,
            column,
        },
        end: SourcePos {
            offset: offset + 1,
            line,
            column: column + 1,
        },
    };
    let mut err =
        ParseError::new("stray '\\' (backslash line continuation must be followed by a newline)");
    err.span = Some(span);
    err.line_text = line_text;
    err
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn should_inject(
    bracket_stack: &[BracketKind],
    last_token: LastToken,
    in_header: bool,
    bytes: &[u8],
    newline_pos: usize,
) -> bool {
    // Only inject at statement level
    if !at_stmt_level(bracket_stack) {
        return false;
    }
    // Don't inject in header mode
    if in_header {
        return false;
    }
    // Only inject after StmtEnder
    if last_token != LastToken::StmtEnder {
        return false;
    }
    // Don't inject if next significant token is `}`
    let next = next_significant_byte(bytes, newline_pos + 1);
    if next == Some(b'}') {
        return false;
    }
    true
}

fn at_stmt_level(bracket_stack: &[BracketKind]) -> bool {
    bracket_stack.is_empty() || bracket_stack.last() == Some(&BracketKind::Block)
}

fn next_significant_byte(bytes: &[u8], start: usize) -> Option<u8> {
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
            i += 1;
            continue;
        }
        // Skip comments
        if b == b'#' && !(i + 1 < bytes.len() && bytes[i + 1] == b'{') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        return Some(b);
    }
    Option::None
}

fn pop_matching(stack: &mut Vec<BracketKind>, expected: BracketKind) {
    if let Some(top) = stack.last()
        && (*top == expected
            || (expected == BracketKind::Block
                && matches!(
                    top,
                    BracketKind::Block | BracketKind::SetLiteral | BracketKind::DictLiteral
                )))
    {
        stack.pop();
    }
}

fn is_string_prefix_start(bytes: &[u8], i: usize) -> bool {
    let b = bytes[i];
    if b == b'r' || b == b'b' {
        // Check if this starts a string prefix
        let remaining = &bytes[i..];
        if remaining.len() >= 2 {
            if remaining[1] == b'\'' || remaining[1] == b'"' {
                return true;
            }
            if remaining.len() >= 3
                && ((remaining[0] == b'r' && remaining[1] == b'b')
                    || (remaining[0] == b'b' && remaining[1] == b'r'))
                && (remaining[2] == b'\'' || remaining[2] == b'"')
            {
                return true;
            }
        }
    }
    false
}

fn skip_string_prefix(bytes: &[u8], i: usize) -> usize {
    let mut j = i;
    if j < bytes.len() && (bytes[j] == b'r' || bytes[j] == b'b') {
        j += 1;
    }
    if j < bytes.len() && (bytes[j] == b'r' || bytes[j] == b'b') {
        j += 1;
    }
    j
}

fn skip_string_body(bytes: &[u8], i: usize) -> usize {
    let len = bytes.len();
    if i >= len {
        return i;
    }
    let quote = bytes[i];
    // Check for triple-quoted
    if i + 2 < len && bytes[i + 1] == quote && bytes[i + 2] == quote {
        // Triple-quoted string
        let mut j = i + 3;
        while j + 2 < len {
            if bytes[j] == quote && bytes[j + 1] == quote && bytes[j + 2] == quote {
                return j + 3;
            }
            if bytes[j] == b'\\' {
                j += 2; // skip escaped char
            } else {
                j += 1;
            }
        }
        return len; // unterminated
    }
    // Single-quoted string
    let mut j = i + 1;
    while j < len {
        if bytes[j] == quote {
            return j + 1;
        }
        if bytes[j] == b'\\' {
            j += 2;
        } else {
            j += 1;
        }
    }
    len // unterminated
}

fn skip_regex(bytes: &[u8], i: usize) -> usize {
    // /pattern/ — can't span newlines
    let len = bytes.len();
    if i >= len || bytes[i] != b'/' {
        return i;
    }
    // Check it's not // (floor-div operator)
    if i + 1 < len && bytes[i + 1] == b'/' {
        return i;
    }
    let mut j = i + 1;
    while j < len {
        if bytes[j] == b'/' {
            return j + 1;
        }
        if bytes[j] == b'\\' {
            j += 2;
        } else if bytes[j] == b'\n' {
            return i; // Not a regex — can't span lines
        } else {
            j += 1;
        }
    }
    i // unterminated, not a regex
}

fn classify_punctuation(bytes: &[u8], i: usize) -> Option<(usize, LastToken)> {
    let len = bytes.len();
    let b = bytes[i];

    match b {
        b'?' => Some((1, LastToken::StmtEnder)),
        b',' => Some((1, LastToken::Continuation)),
        b'.' => Some((1, LastToken::Continuation)),
        b':' => Some((1, LastToken::Continuation)),
        b';' => Some((1, LastToken::Continuation)),
        b'@' => Some((1, LastToken::Continuation)),
        b'+' => {
            if i + 1 < len && bytes[i + 1] == b'+' {
                Some((2, LastToken::StmtEnder)) // ++
            } else if i + 1 < len && bytes[i + 1] == b'=' {
                Some((2, LastToken::Continuation)) // +=
            } else {
                Some((1, LastToken::Continuation)) // +
            }
        }
        b'-' => {
            if i + 1 < len && bytes[i + 1] == b'-' {
                Some((2, LastToken::StmtEnder)) // --
            } else if i + 1 < len && bytes[i + 1] == b'=' {
                Some((2, LastToken::Continuation)) // -=
            } else {
                Some((1, LastToken::Continuation)) // -
            }
        }
        b'*' => {
            if i + 2 < len && bytes[i + 1] == b'*' && bytes[i + 2] == b'=' {
                Some((3, LastToken::Continuation)) // **=
            } else if i + 1 < len && matches!(bytes[i + 1], b'*' | b'=') {
                Some((2, LastToken::Continuation)) // ** or *=
            } else {
                Some((1, LastToken::Continuation)) // *
            }
        }
        b'/' => {
            if i + 2 < len && bytes[i + 1] == b'/' && bytes[i + 2] == b'=' {
                Some((3, LastToken::Continuation)) // //=
            } else if i + 1 < len && matches!(bytes[i + 1], b'/' | b'=') {
                Some((2, LastToken::Continuation)) // // or /=
            } else {
                // Single / could be regex start — handled elsewhere
                Option::None
            }
        }
        b'%' => {
            if i + 1 < len && matches!(bytes[i + 1], b'=' | b'{') {
                // %= or %{ (dict literal sigil, { handled separately)
                let advance = if bytes[i + 1] == b'=' { 2 } else { 1 };
                Some((advance, LastToken::Continuation))
            } else {
                Some((1, LastToken::Continuation)) // %
            }
        }
        b'=' => {
            if i + 1 < len && bytes[i + 1] == b'=' {
                Some((2, LastToken::Continuation)) // ==
            } else {
                Some((1, LastToken::Continuation)) // =
            }
        }
        b'!' => {
            if i + 1 < len && bytes[i + 1] == b'=' {
                Some((2, LastToken::Continuation)) // !=
            } else {
                Some((1, LastToken::Continuation))
            }
        }
        b'<' => {
            if i + 1 < len && bytes[i + 1] == b'=' {
                Some((2, LastToken::Continuation)) // <=
            } else {
                Some((1, LastToken::Continuation)) // <
            }
        }
        b'>' => {
            if i + 1 < len && bytes[i + 1] == b'=' {
                Some((2, LastToken::Continuation)) // >=
            } else {
                Some((1, LastToken::Continuation)) // >
            }
        }
        b'|' => Some((1, LastToken::Continuation)),
        _ => Option::None,
    }
}

fn classify_keyword(word: &[u8]) -> (LastToken, bool) {
    // Returns (token_kind, triggers_header)
    match word {
        // StmtEnder keywords
        b"break" | b"continue" | b"pass" => (LastToken::StmtEnder, false),
        b"return" | b"yield" | b"raise" => (LastToken::StmtEnder, false),
        // Literals
        b"True" | b"False" | b"None" => (LastToken::StmtEnder, false),
        // Header-triggering compound statement keywords (Continuation)
        b"if" | b"elif" => (LastToken::Continuation, true),
        b"else" => (LastToken::Continuation, true),
        b"while" => (LastToken::Continuation, true),
        b"for" => (LastToken::Continuation, true),
        b"def" => (LastToken::Continuation, true),
        b"class" => (LastToken::Continuation, true),
        b"try" => (LastToken::Continuation, true),
        b"except" => (LastToken::Continuation, true),
        b"finally" => (LastToken::Continuation, true),
        b"with" => (LastToken::Continuation, true),
        // Simple continuation keywords (no header)
        b"in" | b"and" | b"or" | b"not" | b"as" => (LastToken::Continuation, false),
        b"from" | b"import" | b"del" | b"assert" | b"let" => (LastToken::Continuation, false),
        b"lines" | b"files" => (LastToken::Continuation, true),
        // Regular identifiers are StmtEnders
        _ => (LastToken::StmtEnder, false),
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pp(s: &str) -> String {
        preprocess(s).unwrap()
    }

    fn has_rs(s: &str) -> bool {
        s.contains('\x1e')
    }

    #[test]
    fn simple_two_statements() {
        let result = pp("x = 1\ny = 2\n");
        // After `1` (StmtEnder), newline before `y` (not `}`) → inject
        assert!(has_rs(&result));
        assert!(result.contains("1\x1e"));
    }

    #[test]
    fn no_inject_inside_parens() {
        let result = pp("f(\n1\n)");
        // Inside parens — no injection
        assert!(!has_rs(&result));
    }

    #[test]
    fn no_inject_inside_brackets() {
        let result = pp("[1,\n2]");
        assert!(!has_rs(&result));
    }

    #[test]
    fn no_inject_after_operator() {
        let result = pp("1 +\n2");
        assert!(!has_rs(&result));
    }

    #[test]
    fn no_inject_before_closing_brace() {
        let result = pp("if x {\n  y\n}");
        // `y` is StmtEnder, but next significant is `}` → don't inject
        let bytes = result.as_bytes();
        // Find the newline after 'y'
        let y_pos = result.find('y').unwrap();
        let nl_after_y = result[y_pos..].find('\n').map(|p| p + y_pos).unwrap();
        assert_ne!(bytes[nl_after_y], RS);
    }

    #[test]
    fn inject_between_statements_in_block() {
        let result = pp("if x {\n  y\n  z\n}");
        // `y` is StmtEnder, next significant is `z` (not `}`) → inject
        assert!(has_rs(&result));
    }

    #[test]
    fn header_mode_suppresses_injection() {
        let result = pp("for\nx\nin\nrange(1) { }");
        // In header mode, no injection until `{`
        // `for` → header mode on, no inject before `x`
        // `x` is StmtEnder but in header mode → no inject
        // Check that no RS appears before the `{`
        let brace_pos = result.find('{').unwrap();
        let before_brace = &result[..brace_pos];
        assert!(!has_rs(before_brace));
    }

    #[test]
    fn header_mode_for_if() {
        let result = pp("if\nTrue\n{ pass }");
        let brace_pos = result.find('{').unwrap();
        let before_brace = &result[..brace_pos];
        assert!(!has_rs(before_brace));
    }

    #[test]
    fn comment_does_not_affect_last_token() {
        let result = pp("x # comment\ny");
        // `x` is StmtEnder, comment is transparent, next is `y` → inject
        assert!(has_rs(&result));
    }

    #[test]
    fn string_is_stmt_ender() {
        let result = pp("\"hello\"\nx");
        assert!(has_rs(&result));
    }

    #[test]
    fn no_inject_after_comma() {
        let result = pp("x,\ny = [1, 2]");
        // comma is Continuation → no inject
        assert!(!has_rs(&result));
    }

    #[test]
    fn no_inject_after_dot() {
        let result = pp("obj.\nattr");
        assert!(!has_rs(&result));
    }

    #[test]
    fn return_is_stmt_ender() {
        // return\n1 → inject (bare return + 1)
        let result = pp("return\n1");
        assert!(has_rs(&result));
    }

    #[test]
    fn yield_is_stmt_ender() {
        let result = pp("yield\n1");
        assert!(has_rs(&result));
    }

    #[test]
    fn raise_is_stmt_ender() {
        let result = pp("raise\nExc()");
        assert!(has_rs(&result));
    }

    #[test]
    fn no_inject_in_set_literal() {
        let result = pp("x = #{1,\n2}");
        assert!(!has_rs(&result));
    }

    #[test]
    fn no_inject_in_dict_literal() {
        let result = pp("x = %{\"a\":\n1}");
        assert!(!has_rs(&result));
    }

    #[test]
    fn preserves_length() {
        let source = "x = 1\ny = 2\nif True {\n  pass\n}\n";
        let result = pp(source);
        assert_eq!(result.len(), source.len());
    }

    #[test]
    fn except_header_mode() {
        // `except` triggers header mode
        let result = pp("try { pass }\nexcept\nException\n{ pass }");
        // Between `except` and `{`, no injection (header mode)
        let except_pos = result.find("except").unwrap();
        let brace_pos = result[except_pos..].find('{').unwrap() + except_pos;
        let between = &result[except_pos..brace_pos];
        assert!(!has_rs(between));
    }

    #[test]
    fn def_header_mode() {
        let result = pp("def\nfoo\n()\n{ pass }");
        let def_pos = result.find("def").unwrap();
        let brace_pos = result[def_pos..].find('{').unwrap() + def_pos;
        let between = &result[def_pos..brace_pos];
        assert!(!has_rs(between));
    }

    #[test]
    fn class_header_mode() {
        let result = pp("class\nC\n{ pass }");
        let class_pos = result.find("class").unwrap();
        let brace_pos = result[class_pos..].find('{').unwrap() + class_pos;
        let between = &result[class_pos..brace_pos];
        assert!(!has_rs(between));
    }

    #[test]
    fn triple_quoted_string() {
        let result = pp("x = '''hello\nworld'''\ny");
        // Triple-quoted string spans newlines, then `y` on next line
        // After the string (StmtEnder), newline before `y` → inject
        assert!(has_rs(&result));
        // But no injection inside the string
        let str_start = result.find("'''").unwrap();
        let str_end = result[str_start + 3..].find("'''").unwrap() + str_start + 3;
        let inside = &result[str_start..str_end];
        assert!(!has_rs(inside));
    }

    #[test]
    fn stmt_sep_semicolon_not_injected_after() {
        let result = pp("x = 1;\ny = 2");
        // `;` is Continuation, so no injection after it
        // The first newline should NOT be replaced
        let semi_pos = result.find(';').unwrap();
        let after = &result[semi_pos + 1..];
        // The newline right after `;` should still be `\n`
        assert_eq!(after.as_bytes()[0], b'\n');
    }

    #[test]
    fn postfix_incr_is_stmt_ender() {
        let result = pp("x++\ny");
        assert!(has_rs(&result));
    }

    #[test]
    fn question_mark_is_stmt_ender() {
        let result = pp("x?\ny");
        assert!(has_rs(&result));
    }

    #[test]
    fn close_brace_is_stmt_ender_but_no_inject_before_close() {
        // `}` followed by newline and then a stmt → inject
        let result = pp("if x { y }\nz");
        assert!(has_rs(&result));
    }

    #[test]
    fn elif_else_no_inject_before_keyword() {
        // `}\nelse` — `}` is StmtEnder, `else` is next significant
        // Since `else` is not `}`, we DO inject \x1e.
        // But the grammar has `stmt_sep*` before `else`, so this is fine.
        let result = pp("if x { y }\nelse { z }");
        assert!(has_rs(&result));
    }

    #[test]
    fn assign_after_equals_no_inject() {
        let result = pp("x =\n1");
        // `=` is Continuation → no inject
        assert!(!has_rs(&result));
    }

    #[test]
    fn no_inject_after_import_keyword() {
        let result = pp("import\nos");
        // `import` is Continuation → no inject
        assert!(!has_rs(&result));
    }

    #[test]
    fn from_import_multiline_injects_after_identifiers() {
        // `from` is Continuation, so no inject after it.
        // But `os` is a regular identifier (StmtEnder), so inject after it.
        // This means `from\nos\nimport\npath` becomes `from os\x1eimport\x1epath`.
        // Simple statement keywords (from, import, etc.) must stay on one line
        // or be broken after commas/operators.
        let result = pp("from\nos\nimport\npath");
        // `from` → no inject; `os` → inject; `import` → no inject; `path` → would inject at end
        assert!(has_rs(&result));
        // Specifically: no inject between `from` and `os` (from is Continuation)
        let from_pos = result.find("from").unwrap();
        let os_pos = result.find("os").unwrap();
        let between = &result[from_pos + 4..os_pos];
        assert!(!has_rs(between), "should not inject between from and os");
    }

    #[test]
    fn no_inject_after_assert_keyword() {
        let result = pp("assert\nTrue");
        assert!(!has_rs(&result));
    }

    #[test]
    fn no_inject_after_del_keyword() {
        let result = pp("del\nx");
        assert!(!has_rs(&result));
    }

    #[test]
    fn regex_is_stmt_ender() {
        // After regex literal, inject
        let result = pp("x in /pattern/\ny");
        assert!(has_rs(&result));
    }

    #[test]
    fn dollar_var_is_stmt_ender() {
        let result = pp("$0\nx");
        assert!(has_rs(&result));
    }

    // === Backslash line continuation tests ===

    #[test]
    fn backslash_continuation_basic() {
        let source = "return \\\n1";
        let result = pp(source);
        assert!(!has_rs(&result), "continuation should suppress injection");
        assert_eq!(result.len(), source.len(), "length must be preserved");
    }

    #[test]
    fn backslash_continuation_after_operator() {
        let source = "x = 1 + \\\n2";
        let result = pp(source);
        assert!(!has_rs(&result));
        assert_eq!(result.len(), source.len());
    }

    #[test]
    fn backslash_continuation_suppresses_injection() {
        // Without continuation, `x\ny` would inject \x1e
        let source = "x \\\ny";
        let result = pp(source);
        assert!(!has_rs(&result), "backslash should prevent stmt boundary");
        assert_eq!(result.len(), source.len());
    }

    #[test]
    fn backslash_continuation_multiple() {
        let source = "x = \\\n1 + \\\n2";
        let result = pp(source);
        assert!(!has_rs(&result));
        assert_eq!(result.len(), source.len());
    }

    #[test]
    fn backslash_in_string_not_continuation() {
        // Backslash inside a string should not be treated as continuation
        let source = "\"hello\\\nworld\"";
        let result = pp(source);
        // The string body is skipped before the backslash check runs,
        // so the string content should be unchanged
        assert_eq!(result, source);
        assert_eq!(result.len(), source.len());
    }

    #[test]
    fn backslash_in_comment_not_continuation() {
        // `x` is StmtEnder, comment is transparent, `y` follows → inject
        let source = "x # comment \\\ny";
        let result = pp(source);
        assert!(
            has_rs(&result),
            "comment backslash should not suppress injection"
        );
        assert_eq!(result.len(), source.len());
    }

    #[test]
    fn backslash_continuation_crlf() {
        let source = "return \\\r\n1";
        let result = pp(source);
        assert!(
            !has_rs(&result),
            "CRLF continuation should suppress injection"
        );
        assert_eq!(result.len(), source.len());
    }

    #[test]
    fn backslash_trailing_space_is_continuation() {
        // Trailing whitespace between backslash and newline is consumed
        let source = "return \\ \n1";
        let result = pp(source);
        assert!(
            !has_rs(&result),
            "trailing space should not prevent continuation"
        );
        assert_eq!(result.len(), source.len());
    }

    #[test]
    fn backslash_trailing_tabs_is_continuation() {
        let source = "return \\\t\t\n1";
        let result = pp(source);
        assert!(
            !has_rs(&result),
            "trailing tabs should not prevent continuation"
        );
        assert_eq!(result.len(), source.len());
    }

    // === Stray backslash error tests ===

    #[test]
    fn stray_backslash_before_token() {
        // `return \1` — backslash not followed by newline
        let err = preprocess("return \\1").unwrap_err();
        assert!(err.message.contains("stray '\\'"));
    }

    #[test]
    fn stray_backslash_with_space_before_token() {
        // `return \ 1` — backslash, space, then non-newline
        let err = preprocess("return \\ 1").unwrap_err();
        assert!(err.message.contains("stray '\\'"));
    }

    #[test]
    fn stray_backslash_at_eof() {
        let err = preprocess("return \\").unwrap_err();
        assert!(err.message.contains("stray '\\'"));
    }

    #[test]
    fn stray_backslash_reports_correct_position() {
        let err = preprocess("x = 1\nreturn \\1").unwrap_err();
        let span = err.span.unwrap();
        assert_eq!(span.start.line, 2);
        assert_eq!(span.start.column, 8);
    }

    #[test]
    fn backslash_inside_brackets_not_error() {
        // Backslash inside $[...] is a JMESPath escape, not a continuation
        let source = "x = $[\\\"key\\\"]";
        let result = pp(source);
        assert_eq!(result.len(), source.len());
    }

    #[test]
    fn backslash_inside_parens_not_error() {
        let source = "f(\\x)";
        let result = pp(source);
        assert_eq!(result.len(), source.len());
    }
}
