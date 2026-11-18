//! Minimal YAML-style frontmatter.
//!
//! bulkhead's human-readable files start with a `---` block of `key: value`
//! lines. This is deliberately a small subset: no nesting, no lists. It is
//! enough for backlog and plan state and keeps every file `cat`-able.

use std::collections::BTreeMap;

/// Reasons frontmatter cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrontmatterError {
    /// A `---` opener had no closing `---`.
    #[error("frontmatter is missing its closing `---`")]
    Unterminated,
}

/// Splits frontmatter from the body.
///
/// Text without a leading `---` is returned whole with empty fields. Quotes
/// around a value are stripped.
pub fn parse(text: &str) -> Result<(BTreeMap<String, String>, String), FrontmatterError> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Ok((BTreeMap::new(), text.to_owned()));
    }

    let mut fields = BTreeMap::new();
    for line in lines.by_ref() {
        if line.trim() == "---" {
            let body: Vec<&str> = lines.collect();
            return Ok((fields, body.join("\n")));
        }
        if let Some((key, value)) = line.split_once(':') {
            fields.insert(
                key.trim().to_owned(),
                value.trim().trim_matches('"').to_owned(),
            );
        }
    }

    Err(FrontmatterError::Unterminated)
}

/// Renders `key: value` frontmatter followed by the body.
pub fn render(fields: &[(&str, &str)], body: &str) -> String {
    let mut output = String::from("---\n");
    for (key, value) in fields {
        output.push_str(key);
        output.push_str(": ");
        output.push_str(value);
        output.push('\n');
    }
    output.push_str("---\n");
    output.push_str(body);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fields_and_body() {
        let (fields, body) =
            parse("---\ntitle: Add parser\nstate: backlog\n---\nDo it.\n").unwrap();
        assert_eq!(fields["title"], "Add parser");
        assert_eq!(fields["state"], "backlog");
        assert_eq!(body, "Do it.");
    }

    #[test]
    fn text_without_frontmatter_is_all_body() {
        let (fields, body) = parse("# Just a heading\n").unwrap();
        assert!(fields.is_empty());
        assert_eq!(body, "# Just a heading\n");
    }

    #[test]
    fn unterminated_frontmatter_errors() {
        assert_eq!(
            parse("---\ntitle: x\n"),
            Err(FrontmatterError::Unterminated)
        );
    }

    #[test]
    fn render_round_trips_through_parse() {
        let text = render(&[("title", "Add parser"), ("state", "backlog")], "Do it.\n");
        let (fields, body) = parse(&text).unwrap();
        assert_eq!(fields["title"], "Add parser");
        assert_eq!(body, "Do it.");
    }
}
