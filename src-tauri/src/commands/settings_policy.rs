use serde::Deserialize;

const SETTING_KEY_FORBIDDEN: &str =
    "setting_key_forbidden: This setting cannot be changed through the generic settings API.";
const SETTING_VALUE_INVALID: &str = "setting_value_invalid: The setting value is invalid.";

const AI_PROVIDERS: &[&str] = &[
    "claude",
    "glm",
    "minimax",
    "kimi",
    "deepseek",
    "openrouter",
    "custom",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SettingCategory {
    Ai,
    Font,
    Platform,
    Update,
}

impl SettingCategory {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Ai => "ai",
            Self::Font => "font",
            Self::Platform => "platform",
            Self::Update => "update",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlatformCategoryVisibility {
    coding: bool,
    lobster: bool,
}

pub(super) fn category_for_key(key: &str) -> Option<SettingCategory> {
    match key {
        "platform_category_visibility" => Some(SettingCategory::Platform),
        "central_update_check_mode_v1" => Some(SettingCategory::Update),
        "font_scale_v1" => Some(SettingCategory::Font),
        "ai_provider"
        | "ai_tag_concurrency"
        | "ai_tag_interval_ms"
        | "ai_tag_stop_on_rate_limit" => Some(SettingCategory::Ai),
        _ if font_key_kind(key).is_some() => Some(SettingCategory::Font),
        _ if provider_scoped_ai_key(key).is_some() => Some(SettingCategory::Ai),
        _ => None,
    }
}

pub(super) fn validate_setting(key: &str, value: &str) -> Result<SettingCategory, String> {
    let category = category_for_key(key).ok_or_else(|| SETTING_KEY_FORBIDDEN.to_string())?;

    let valid = match key {
        "platform_category_visibility" => validate_platform_visibility(value),
        "central_update_check_mode_v1" => matches!(value, "regular" | "sync"),
        "font_scale_v1" => matches!(value, "0.875" | "1" | "1.125"),
        "ai_provider" => AI_PROVIDERS.contains(&value),
        "ai_tag_concurrency" => parse_integer_in_range(value, 1, 8),
        "ai_tag_interval_ms" => parse_integer_in_range(value, 0, 60_000),
        "ai_tag_stop_on_rate_limit" => matches!(value, "true" | "false"),
        _ => validate_family_value(key, value),
    };

    if valid {
        Ok(category)
    } else {
        Err(SETTING_VALUE_INVALID.to_string())
    }
}

pub(super) fn setting_audit_details<'a>(
    keys: impl Iterator<Item = &'a str>,
    value_stored: bool,
) -> serde_json::Value {
    let keys = keys.collect::<Vec<_>>();
    let categories = keys
        .iter()
        .filter_map(|key| category_for_key(key))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(SettingCategory::as_str)
        .collect::<Vec<_>>();
    serde_json::json!({
        "categories": categories,
        "keyCount": keys.len(),
        "valueStored": value_stored,
    })
}

fn validate_family_value(key: &str, value: &str) -> bool {
    if let Some(kind) = font_key_kind(key) {
        return match kind {
            FontKeyKind::Display => {
                matches!(
                    value,
                    "geist" | "jetbrains" | "inter" | "serif" | "system" | "custom"
                )
            }
            FontKeyKind::Body => {
                matches!(value, "jetbrains" | "geist" | "inter" | "system" | "custom")
            }
            FontKeyKind::ChineseFallback => {
                matches!(value, "system" | "sourceHanSerif" | "custom")
            }
            FontKeyKind::Custom => validate_bounded_text(value, 256, true),
        };
    }

    let Some(name) = provider_scoped_ai_key(key) else {
        return false;
    };
    match name {
        "ai_region" => matches!(value, "cn" | "intl"),
        "ai_protocol" => matches!(value, "" | "anthropic" | "openai"),
        "ai_model" => validate_bounded_text(value, 512, true),
        "ai_api_url" | "ai_custom_base_url" => validate_http_url(value),
        _ => false,
    }
}

fn validate_platform_visibility(value: &str) -> bool {
    serde_json::from_str::<PlatformCategoryVisibility>(value)
        .is_ok_and(|visibility| visibility.coding || visibility.lobster)
}

fn parse_integer_in_range(value: &str, min: u32, max: u32) -> bool {
    value
        .parse::<u32>()
        .is_ok_and(|parsed| parsed >= min && parsed <= max)
}

fn validate_bounded_text(value: &str, max_len: usize, allow_empty: bool) -> bool {
    (allow_empty || !value.is_empty())
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
}

fn validate_http_url(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    if !validate_bounded_text(value, 2_048, false) {
        return false;
    }
    reqwest::Url::parse(value)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

fn provider_scoped_ai_key(key: &str) -> Option<&str> {
    const NAMES: &[&str] = &[
        "ai_region",
        "ai_model",
        "ai_api_url",
        "ai_custom_base_url",
        "ai_protocol",
    ];

    let (name, provider) = key.rsplit_once("__")?;
    (NAMES.contains(&name) && AI_PROVIDERS.contains(&provider)).then_some(name)
}

#[derive(Clone, Copy)]
enum FontKeyKind {
    Display,
    Body,
    ChineseFallback,
    Custom,
}

fn font_key_kind(key: &str) -> Option<FontKeyKind> {
    const SUFFIXES: &[&str] = &["_v1", "_light_v2", "_dark_v2"];
    let base = SUFFIXES
        .iter()
        .find_map(|suffix| key.strip_suffix(suffix))?;
    match base {
        "display_font" => Some(FontKeyKind::Display),
        "body_font" => Some(FontKeyKind::Body),
        "display_chinese_fallback" | "body_chinese_fallback" => Some(FontKeyKind::ChineseFallback),
        "display_font_custom"
        | "body_font_custom"
        | "display_chinese_fallback_custom"
        | "body_chinese_fallback_custom" => Some(FontKeyKind::Custom),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_live_renderer_setting_families() {
        let cases = [
            (
                "platform_category_visibility",
                r#"{"coding":true,"lobster":false}"#,
            ),
            ("central_update_check_mode_v1", "sync"),
            ("display_font_v1", "geist"),
            ("display_font_custom_dark_v2", "Segoe UI"),
            ("body_chinese_fallback_light_v2", "sourceHanSerif"),
            ("font_scale_v1", "1.125"),
            ("ai_provider", "custom"),
            ("ai_tag_concurrency", "8"),
            ("ai_tag_interval_ms", "60000"),
            ("ai_tag_stop_on_rate_limit", "false"),
            ("ai_region__glm", "cn"),
            ("ai_model__custom", "local-model"),
            ("ai_api_url__custom", "https://example.com/v1/messages"),
            ("ai_custom_base_url__custom", ""),
            ("ai_protocol__custom", "openai"),
        ];

        for (key, value) in cases {
            assert!(validate_setting(key, value).is_ok(), "{key}");
        }

        for mode in ["v1", "light_v2", "dark_v2"] {
            for (base, value) in [
                ("display_font", "geist"),
                ("display_font_custom", "Segoe UI"),
                ("display_chinese_fallback", "system"),
                ("display_chinese_fallback_custom", "Microsoft YaHei"),
                ("body_font", "jetbrains"),
                ("body_font_custom", "Inter"),
                ("body_chinese_fallback", "sourceHanSerif"),
                ("body_chinese_fallback_custom", "SimSun"),
            ] {
                let key = format!("{base}_{mode}");
                assert!(validate_setting(&key, value).is_ok(), "{key}");
            }
        }

        for provider in AI_PROVIDERS {
            for (name, value) in [
                ("ai_region", "intl"),
                ("ai_model", "model"),
                ("ai_api_url", "https://example.com/v1/messages"),
                ("ai_custom_base_url", ""),
                ("ai_protocol", "anthropic"),
            ] {
                let key = format!("{name}__{provider}");
                assert!(validate_setting(&key, value).is_ok(), "{key}");
            }
        }
    }

    #[test]
    fn rejects_forbidden_domains_and_unknown_keys_without_echoing_input() {
        for key in [
            "ssh_targets_v1",
            "wsl_targets_v1",
            "active_target_id_v1",
            "target_config_quarantine_v1",
            "github_pat",
            "ai_api_key__deepseek",
            "migration_completed_v1",
            "feature_preview_enabled",
            "attacker_key_password=secret",
        ] {
            let error = validate_setting(key, "credential-value").unwrap_err();
            assert_eq!(error, SETTING_KEY_FORBIDDEN);
            assert!(!error.contains(key));
            assert!(!error.contains("credential-value"));
        }
    }

    #[test]
    fn rejects_invalid_typed_values_without_echoing_input() {
        for (key, value) in [
            ("central_update_check_mode_v1", "always"),
            (
                "platform_category_visibility",
                r#"{"coding":false,"lobster":false}"#,
            ),
            ("font_scale_v1", "9"),
            ("display_font_v1", "Comic Sans"),
            ("ai_tag_concurrency", "9"),
            ("ai_tag_interval_ms", "-1"),
            ("ai_tag_stop_on_rate_limit", "1"),
            ("ai_protocol__custom", "raw"),
            ("ai_api_url__custom", "file:///secret"),
            ("ai_model__unknown", "model"),
        ] {
            let error = validate_setting(key, value).unwrap_err();
            assert!(matches!(
                error.as_str(),
                SETTING_KEY_FORBIDDEN | SETTING_VALUE_INVALID
            ));
            assert!(!error.contains(value));
        }
    }
}
