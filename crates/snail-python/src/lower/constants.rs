// Constants that need to be public for codegen
pub const SNAIL_TRY_HELPER: &str = "__snail_compact_try";
pub const SNAIL_EXCEPTION_VAR: &str = "__snail_compact_exc";
pub const SNAIL_SUBPROCESS_CAPTURE_CLASS: &str = "__SnailSubprocessCapture";
pub const SNAIL_SUBPROCESS_STATUS_CLASS: &str = "__SnailSubprocessStatus";
pub const SNAIL_REGEX_SEARCH: &str = "__snail_regex_search";
pub const SNAIL_REGEX_COMPILE: &str = "__snail_regex_compile";
pub const SNAIL_JMESPATH_QUERY: &str = "__snail_jmespath_query";
pub const SNAIL_PARTIAL_HELPER: &str = "__snail_partial";
pub const SNAIL_CONTAINS_HELPER: &str = "__snail_contains__";
pub const SNAIL_CONTAINS_NOT_HELPER: &str = "__snail_contains_not__";
pub const SNAIL_INCR_ATTR: &str = "__snail_incr_attr";
pub const SNAIL_INCR_INDEX: &str = "__snail_incr_index";
pub const SNAIL_AUG_ATTR: &str = "__snail_aug_attr";
pub const SNAIL_AUG_INDEX: &str = "__snail_aug_index";
pub(crate) const SNAIL_LET_VALUE: &str = "__snail_let_value";
pub(crate) const SNAIL_LET_OK: &str = "__snail_let_ok";
pub(crate) const SNAIL_LET_KEEP: &str = "__snail_let_keep";
pub(crate) const SNAIL_COMPARE_LEFT: &str = "__snail_compare_left";
pub(crate) const SNAIL_COMPARE_RIGHT: &str = "__snail_compare_right";
pub(crate) const SNAIL_INCR_TMP: &str = "__snail_incr_tmp";

// Awk-related constants (public within crate)
pub(crate) const SNAIL_AWK_NR: &str = "$n";
pub(crate) const SNAIL_AWK_FNR: &str = "$fn";
pub(crate) const SNAIL_AWK_PATH: &str = "$p";
pub(crate) const SNAIL_AWK_MATCH: &str = "$m";
pub(crate) const SNAIL_AWK_FIELDS: &str = "$f";
pub(crate) const SNAIL_AWK_LINE_PYVAR: &str = "__snail_line";
pub(crate) const SNAIL_AWK_FIELDS_PYVAR: &str = "__snail_fields";
pub(crate) const SNAIL_AWK_NR_PYVAR: &str = "__snail_nr_user";
pub(crate) const SNAIL_AWK_FNR_PYVAR: &str = "__snail_fnr_user";
pub(crate) const SNAIL_AWK_PATH_PYVAR: &str = "__snail_path_user";
pub(crate) const SNAIL_AWK_MATCH_PYVAR: &str = "__snail_match";

// Map-related constants (public within crate)
pub(crate) const SNAIL_MAP_SRC: &str = "$src";
pub(crate) const SNAIL_MAP_FD: &str = "$fd";
pub(crate) const SNAIL_MAP_TEXT: &str = "$text";
pub(crate) const SNAIL_MAP_SRC_PYVAR: &str = "__snail_src";
pub(crate) const SNAIL_MAP_FD_PYVAR: &str = "__snail_fd";
pub(crate) const SNAIL_MAP_TEXT_PYVAR: &str = "__snail_text";
pub const SNAIL_LAZY_TEXT_CLASS: &str = "__SnailLazyText";
pub const SNAIL_LAZY_FILE_CLASS: &str = "__SnailLazyFile";

pub(crate) fn injected_py_name(name: &str) -> Option<&'static str> {
    match name {
        // Awk variables
        SNAIL_AWK_NR => Some(SNAIL_AWK_NR_PYVAR),
        SNAIL_AWK_FNR => Some(SNAIL_AWK_FNR_PYVAR),
        SNAIL_AWK_PATH => Some(SNAIL_AWK_PATH_PYVAR),
        SNAIL_AWK_MATCH => Some(SNAIL_AWK_MATCH_PYVAR),
        SNAIL_AWK_FIELDS => Some(SNAIL_AWK_FIELDS_PYVAR),
        // Map variables
        SNAIL_MAP_SRC => Some(SNAIL_MAP_SRC_PYVAR),
        SNAIL_MAP_FD => Some(SNAIL_MAP_FD_PYVAR),
        SNAIL_MAP_TEXT => Some(SNAIL_MAP_TEXT_PYVAR),
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
