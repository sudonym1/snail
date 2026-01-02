use std::path::Path;

use similar::TextDiff;

pub fn format_snail_source(source: &str) -> String {
    let mut formatted: Vec<String> = source
        .lines()
        .map(|line| line.trim_end_matches([' ', '\t']).to_string())
        .collect();

    if !formatted.is_empty() || source.ends_with('\n') {
        formatted.push(String::new());
    }

    formatted.join("\n")
}

pub fn unified_diff(original: &str, formatted: &str, path: &Path) -> Result<String, String> {
    let diff = TextDiff::from_lines(original, formatted);

    let mut out = Vec::new();
    diff.unified_diff()
        .context_radius(3)
        .header(&path.to_string_lossy(), &path.to_string_lossy())
        .to_writer(&mut out)
        .map_err(|err| err.to_string())?;

    String::from_utf8(out).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_trailing_whitespace_and_newline() {
        let input = "value = 1  \nnext_line\t\t";
        let formatted = format_snail_source(input);

        assert_eq!(formatted, "value = 1\nnext_line\n");
    }
}
