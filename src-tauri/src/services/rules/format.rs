use super::RulesError;

pub(super) struct ParsedRule<'a> {
    pub body: &'a str,
    pub prefix: &'a str,
    pub description: String,
    pub supported: bool,
    pub always_apply: bool,
}

pub(super) fn parse(bytes: &[u8]) -> Result<ParsedRule<'_>, RulesError> {
    let text = std::str::from_utf8(bytes).map_err(|_| RulesError::Unsupported)?;
    let mut body = text;
    let mut description = String::new();
    let mut supported = true;
    let mut always_apply = false;
    if text.starts_with("---\n") || text.starts_with("---\r\n") {
        let opening = text.find('\n').ok_or(RulesError::Unsupported)? + 1;
        let mut end = None;
        let mut offset = opening;
        for line in text[opening..].split_inclusive('\n') {
            if line.trim_end_matches(['\r', '\n']) == "---" {
                end = Some((offset, offset + line.len()));
                break;
            }
            offset += line.len();
        }
        let (header_end, body_start) = end.ok_or(RulesError::Unsupported)?;
        let value: serde_norway::Value = serde_norway::from_str(&text[opening..header_end])
            .map_err(|_| RulesError::Unsupported)?;
        let mapping = value.as_mapping().ok_or(RulesError::Unsupported)?;
        for (key, value) in mapping {
            let key = key.as_str().ok_or(RulesError::Unsupported)?;
            match key {
                "description" => {
                    description = value.as_str().ok_or(RulesError::Unsupported)?.to_string();
                }
                "alwaysApply" => {
                    always_apply = value.as_bool() == Some(true);
                    if value.as_bool() != Some(true) {
                        supported = false;
                    }
                }
                "enabled" => {
                    if value.as_bool() != Some(true) {
                        supported = false;
                    }
                }
                "paths" | "globs" | "condition" | "astCondition" | "question" | "scope"
                | "agents" => supported = false,
                _ => {}
            }
        }
        body = &text[body_start..];
    }
    let prefix = &text[..text.len() - body.len()];
    Ok(ParsedRule {
        body,
        prefix,
        description,
        supported,
        always_apply,
    })
}

pub(super) fn title(body: &str, name: &str) -> String {
    body.lines()
        .find_map(|line| {
            line.strip_prefix("# ")
                .map(str::trim)
                .filter(|v| !v.is_empty())
        })
        .unwrap_or(name)
        .to_string()
}

pub(super) fn shared(body: &str, description: &str) -> Vec<u8> {
    // A JSON string is also a YAML double-quoted scalar. Newlines and quotes cannot inject fields.
    let description = serde_json::to_string(description).expect("string serialization");
    format!("---\nalwaysApply: true\ndescription: {description}\n---\n{body}").into_bytes()
}
