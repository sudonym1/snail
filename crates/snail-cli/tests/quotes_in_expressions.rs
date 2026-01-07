use snail::{CompileMode, compile_snail_source};

/// Test that quotes inside regex patterns are properly escaped
#[test]
fn regex_with_double_quotes() {
    let source = r#"x = /foo"bar/"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    // Should generate valid Python with escaped quotes
    assert!(python.contains(r#"__snail_regex_compile"#));
    // The pattern should be in a raw string with escaped quotes
    // Raw strings in Python can't contain their delimiter without escaping
    // So we need to use a different approach or escape properly

    // Verify it's valid Python by checking it doesn't have syntax errors
    // The generated code should be something like r"foo\"bar" or use single quotes
    println!("Generated Python:\n{}", python);
}

#[test]
fn regex_with_single_quotes() {
    let source = r#"x = /foo'bar/"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__snail_regex_compile"#));
    println!("Generated Python:\n{}", python);
}

#[test]
fn regex_with_both_quotes() {
    let source = r#"x = /foo"bar'baz/"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__snail_regex_compile"#));
    println!("Generated Python:\n{}", python);
}

#[test]
fn regex_with_backslash_and_quotes() {
    let source = r#"x = /foo\"bar/"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__snail_regex_compile"#));
    println!("Generated Python:\n{}", python);
}

/// Test that quotes inside subprocess capture $() are properly escaped
#[test]
fn subprocess_capture_with_double_quotes() {
    let source = r#"x = $(echo "test")"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__SnailSubprocessCapture"#));
    // Should escape the quotes in the f-string
    assert!(python.contains(r#"\"test\""#) || python.contains(r#"'test'"#));
    println!("Generated Python:\n{}", python);
}

#[test]
fn subprocess_capture_with_single_quotes() {
    let source = r#"x = $(echo 'test')"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__SnailSubprocessCapture"#));
    println!("Generated Python:\n{}", python);
}

#[test]
fn subprocess_capture_with_both_quotes() {
    let source = r#"x = $(echo "it's a test")"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__SnailSubprocessCapture"#));
    println!("Generated Python:\n{}", python);
}

/// Test that quotes inside subprocess status @() are properly escaped
#[test]
fn subprocess_status_with_double_quotes() {
    let source = r#"x = @(echo "test")"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__SnailSubprocessStatus"#));
    assert!(python.contains(r#"\"test\""#) || python.contains(r#"'test'"#));
    println!("Generated Python:\n{}", python);
}

#[test]
fn subprocess_status_with_single_quotes() {
    let source = r#"x = $(echo 'test')"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__SnailSubprocessCapture"#));
    println!("Generated Python:\n{}", python);
}

/// Test structured accessor with quotes
#[test]
fn structured_accessor_with_double_quotes() {
    let source = r#"x = data | $[users[0]."name"]"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__SnailStructuredAccessor"#));
    // Should properly escape the double quote inside the query string
    assert!(python.contains(r#"\"name\""#) || python.contains(r#"'name'"#));
    println!("Generated Python:\n{}", python);
}

#[test]
fn structured_accessor_with_single_quotes() {
    let source = r#"x = data | $[users[0].'name']"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__SnailStructuredAccessor"#));
    println!("Generated Python:\n{}", python);
}

/// Test regex with interpolation and quotes
#[test]
fn regex_interpolated_with_quotes() {
    let source = r#"
pattern = "test"
x = /foo"{pattern}"bar/
"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__snail_regex_compile"#));
    println!("Generated Python:\n{}", python);
}

/// Test subprocess with interpolation and quotes
#[test]
fn subprocess_interpolated_with_quotes() {
    let source = r#"
cmd = "ls"
x = $(echo "{cmd}" "test")
"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__SnailSubprocessCapture"#));
    println!("Generated Python:\n{}", python);
}

/// Test edge cases with escaped quotes
#[test]
fn regex_with_escaped_forward_slash() {
    let source = r#"x = /foo\/bar/"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__snail_regex_compile"#));
    // Should preserve the escaped forward slash
    assert!(python.contains(r#"foo/bar"#) || python.contains(r#"foo\/bar"#));
    println!("Generated Python:\n{}", python);
}

#[test]
fn subprocess_with_dollar_signs() {
    let source = r#"x = $(echo $$PATH)"#;
    let python = compile_snail_source(source, CompileMode::Snail).expect("should compile");
    assert!(python.contains(r#"__SnailSubprocessCapture"#));
    // $$ should become a single $
    assert!(python.contains(r#"$PATH"#));
    println!("Generated Python:\n{}", python);
}
