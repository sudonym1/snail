#[path = "whitespace_cases.rs"]
mod whitespace_cases;

use snail_ast::Program;
use snail_parser::parse as parse_program;
use whitespace_cases::{BoundaryExpectation, WsCase};

#[test]
fn whitespace_session_corpus() {
    run_group("session-corpus", whitespace_cases::SESSION_CORPUS_CASES);
}

#[test]
fn whitespace_statement_matrix() {
    run_group("statement-matrix", whitespace_cases::STATEMENT_MATRIX_CASES);
}

#[test]
fn whitespace_selected_cases_are_deterministic() {
    let cases = whitespace_cases::selected_cases();
    for case in cases {
        run_case("selected-cases", case);
    }
}

fn run_group(group: &str, cases: &[WsCase]) {
    for case in cases {
        run_case(group, case);
    }
}

fn run_case(group: &str, case: &WsCase) {
    assert!(
        !case.variants.is_empty(),
        "whitespace case has no variants\n\
         group: {group}\n\
         case id: {}\n\
         baseline:\n{}",
        case.name,
        case.base
    );

    let base_program = parse_program(case.base).unwrap_or_else(|err| {
        panic!(
            "baseline parse failed\n\
             group: {group}\n\
             case id: {}\n\
             expected: baseline should parse\n\
             actual: parse error\n\
             baseline:\n{}\n\
             parse error:\n{err}",
            case.name, case.base
        )
    });
    let base_canonical = canonical_program_debug(&base_program);

    for (variant_index, variant) in case.variants.iter().enumerate() {
        match case.expectation {
            BoundaryExpectation::MustContinue => {
                let variant_program = parse_program(variant).unwrap_or_else(|err| {
                    panic!(
                        "whitespace case failed\n\
                         group: {group}\n\
                         case id: {}\n\
                         variant index: {variant_index}\n\
                         expected: MustContinue\n\
                         actual: parse error\n\
                         baseline:\n{}\n\
                         variant:\n{}\n\
                         parse error:\n{err}",
                        case.name, case.base, variant
                    )
                });
                let variant_canonical = canonical_program_debug(&variant_program);
                assert_eq!(
                    base_canonical, variant_canonical,
                    "whitespace case failed\n\
                     group: {group}\n\
                     case id: {}\n\
                     variant index: {variant_index}\n\
                     expected: MustContinue\n\
                     actual: parsed but canonical AST differed\n\
                     baseline:\n{}\n\
                     variant:\n{}\n\
                     baseline canonical:\n{}\n\
                     variant canonical:\n{}",
                    case.name, case.base, variant, base_canonical, variant_canonical
                );
            }
            BoundaryExpectation::MaySeparate => {
                let variant_program = parse_program(variant).unwrap_or_else(|err| {
                    panic!(
                        "whitespace case failed\n\
                         group: {group}\n\
                         case id: {}\n\
                         variant index: {variant_index}\n\
                         expected: MaySeparate\n\
                         actual: parse error\n\
                         baseline:\n{}\n\
                         variant:\n{}\n\
                         parse error:\n{err}",
                        case.name, case.base, variant
                    )
                });
                let variant_canonical = canonical_program_debug(&variant_program);
                assert_ne!(
                    base_canonical, variant_canonical,
                    "whitespace case failed\n\
                     group: {group}\n\
                     case id: {}\n\
                     variant index: {variant_index}\n\
                     expected: MaySeparate\n\
                     actual: canonical AST matched baseline\n\
                     baseline:\n{}\n\
                     variant:\n{}\n\
                     canonical AST:\n{}",
                    case.name, case.base, variant, variant_canonical
                );
            }
            BoundaryExpectation::MustFail => {
                if let Ok(program) = parse_program(variant) {
                    let variant_canonical = canonical_program_debug(&program);
                    panic!(
                        "whitespace case failed\n\
                         group: {group}\n\
                         case id: {}\n\
                         variant index: {variant_index}\n\
                         expected: MustFail\n\
                         actual: parsed successfully\n\
                         baseline:\n{}\n\
                         variant:\n{}\n\
                         variant canonical:\n{}",
                        case.name, case.base, variant, variant_canonical
                    );
                }
            }
        }
    }
}

fn canonical_program_debug(program: &Program) -> String {
    let debug = format!("{program:#?}");
    strip_source_spans(&debug)
}

fn strip_source_spans(debug: &str) -> String {
    const SOURCE_SPAN_PREFIX: &str = "SourceSpan {";
    let brace_offset = SOURCE_SPAN_PREFIX
        .find('{')
        .expect("SourceSpan prefix should include an opening brace");

    let mut output = String::with_capacity(debug.len());
    let mut index = 0;
    while index < debug.len() {
        if debug[index..].starts_with(SOURCE_SPAN_PREFIX) {
            output.push_str("SourceSpan { .. }");
            index = skip_braced_block(debug, index + brace_offset);
            continue;
        }

        let ch = debug[index..]
            .chars()
            .next()
            .expect("index should be at a char boundary");
        output.push(ch);
        index += ch.len_utf8();
    }
    output
}

fn skip_braced_block(text: &str, brace_start: usize) -> usize {
    assert_eq!(text.as_bytes()[brace_start], b'{');

    let mut depth = 0usize;
    let mut index = brace_start;
    while index < text.len() {
        let ch = text[index..]
            .chars()
            .next()
            .expect("index should be at a char boundary");
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return index + ch.len_utf8();
                }
            }
            _ => {}
        }
        index += ch.len_utf8();
    }

    panic!("unbalanced braces while stripping SourceSpan from debug output");
}
