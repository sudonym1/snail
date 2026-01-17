// Constants that need to be public for codegen
pub const SNAIL_TRY_HELPER: &str = "__snail_compact_try";
pub const SNAIL_EXCEPTION_VAR: &str = "__snail_compact_exc";
pub const SNAIL_SUBPROCESS_CAPTURE_CLASS: &str = "__SnailSubprocessCapture";
pub const SNAIL_SUBPROCESS_STATUS_CLASS: &str = "__SnailSubprocessStatus";
pub const SNAIL_REGEX_SEARCH: &str = "__snail_regex_search";
pub const SNAIL_REGEX_COMPILE: &str = "__snail_regex_compile";
pub const SNAIL_JMESPATH_QUERY: &str = "__snail_jmespath_query";
pub const SNAIL_PARTIAL_HELPER: &str = "__snail_partial";

// Awk-related constants (public within crate)
pub(crate) const SNAIL_AWK_NR: &str = "$n";
pub(crate) const SNAIL_AWK_FNR: &str = "$fn";
pub(crate) const SNAIL_AWK_PATH: &str = "$p";
pub(crate) const SNAIL_AWK_MATCH: &str = "$m";
pub(crate) const SNAIL_AWK_LINE_PYVAR: &str = "__snail_line";
pub(crate) const SNAIL_AWK_FIELDS_PYVAR: &str = "__snail_fields";
pub(crate) const SNAIL_AWK_NR_PYVAR: &str = "__snail_nr_user";
pub(crate) const SNAIL_AWK_FNR_PYVAR: &str = "__snail_fnr_user";
pub(crate) const SNAIL_AWK_PATH_PYVAR: &str = "__snail_path_user";
pub(crate) const SNAIL_AWK_MATCH_PYVAR: &str = "__snail_match";

pub(crate) fn injected_py_name(name: &str) -> Option<&'static str> {
    match name {
        SNAIL_AWK_NR => Some(SNAIL_AWK_NR_PYVAR),
        SNAIL_AWK_FNR => Some(SNAIL_AWK_FNR_PYVAR),
        SNAIL_AWK_PATH => Some(SNAIL_AWK_PATH_PYVAR),
        SNAIL_AWK_MATCH => Some(SNAIL_AWK_MATCH_PYVAR),
        _ => None,
    }
}

pub(crate) fn escape_for_python_string(value: &str) -> String {
    // Escape special characters for a Python string literal
    // This is used for raw source text that needs to be embedded in a Python string
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
