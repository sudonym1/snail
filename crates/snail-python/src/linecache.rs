use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};

const SNAIL_TRACE_PREFIX: &str = "snail:";

pub(crate) fn display_filename(filename: &str) -> String {
    if filename.starts_with(SNAIL_TRACE_PREFIX) {
        filename.to_string()
    } else {
        format!("{SNAIL_TRACE_PREFIX}{filename}")
    }
}

pub(crate) fn strip_display_prefix(filename: &str) -> &str {
    filename
        .strip_prefix(SNAIL_TRACE_PREFIX)
        .unwrap_or(filename)
}

pub(crate) fn register_linecache(py: Python<'_>, filename: &str, source: &str) -> PyResult<()> {
    let linecache = py.import_bound("linecache")?;
    let cache = linecache.getattr("cache")?;
    let lines = split_source_lines(source);
    let entry = PyTuple::new_bound(
        py,
        vec![
            source.len().into_py(py),
            py.None().into_py(py),
            PyList::new_bound(py, lines).into_py(py),
            filename.into_py(py),
        ],
    );
    cache.set_item(filename, entry)?;
    Ok(())
}

fn split_source_lines(source: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (idx, ch) in source.char_indices() {
        if ch == '\n' {
            let end = idx + 1;
            lines.push(source[start..end].to_string());
            start = end;
        }
    }
    if start < source.len() {
        lines.push(source[start..].to_string());
    }
    lines
}
