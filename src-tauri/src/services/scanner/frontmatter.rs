//! Shared SKILL.md frontmatter fence extraction. The single implementation of
//! `---` fence stripping in the repo: `scanner::parse_skill_md_content` (local
//! and remote scans via `ssh_batch`) and `github_import::parse_frontmatter`
//! both delegate here, so BOM and fence edge cases cannot diverge again.

/// Extract the raw YAML block between the opening and closing `---` fences.
///
/// Semantics (adopted from the stricter of the two historical parsers):
/// * a leading UTF-8 BOM and leading whitespace are skipped;
/// * the opening fence is the first line and must trim to exactly `---`;
/// * the closing fence must be an independent line trimming to exactly `---`
///   (a `---` embedded mid-line, e.g. inside a quoted YAML scalar, does not
///   close the block);
/// * CRLF is accepted throughout.
pub fn extract_frontmatter_block(content: &str) -> Option<&str> {
    let content = content.trim_start_matches('\u{feff}').trim_start();
    let opening_end = content.find('\n')?;
    if content[..opening_end].trim() != "---" {
        return None;
    }

    let rest = &content[opening_end + 1..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim() == "---" {
            return Some(&rest[..offset]);
        }
        offset += line.len();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::extract_frontmatter_block;

    #[test]
    fn strips_utf8_bom_before_opening_fence() {
        let content = "\u{feff}---\nname: bom-skill\n---\nBody.\n";
        assert_eq!(
            extract_frontmatter_block(content),
            Some("name: bom-skill\n")
        );
    }

    #[test]
    fn allows_leading_blank_lines() {
        let content = "\n\n---\nname: padded\n---\n";
        assert_eq!(extract_frontmatter_block(content), Some("name: padded\n"));
    }

    #[test]
    fn accepts_crlf_fences() {
        let content = "---\r\nname: crlf\r\n---\r\nBody.";
        assert_eq!(extract_frontmatter_block(content), Some("name: crlf\r\n"));
    }

    #[test]
    fn closing_fence_at_eof_without_trailing_newline() {
        let content = "---\nname: eof\n---";
        assert_eq!(extract_frontmatter_block(content), Some("name: eof\n"));
    }

    #[test]
    fn mid_line_triple_dash_does_not_close_the_block() {
        let content = "---\nname: x\ndescription: \"runs --- somewhere\"\n---\n";
        assert_eq!(
            extract_frontmatter_block(content),
            Some("name: x\ndescription: \"runs --- somewhere\"\n")
        );
    }

    #[test]
    fn rejects_missing_or_decorated_fences() {
        assert!(extract_frontmatter_block("# no frontmatter\n").is_none());
        assert!(extract_frontmatter_block("").is_none());
        assert!(extract_frontmatter_block("---\nname: open-only\n").is_none());
        assert!(extract_frontmatter_block("---\nname: x\n---extra\n").is_none());
    }
}
