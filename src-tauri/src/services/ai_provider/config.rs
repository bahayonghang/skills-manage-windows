use super::prompt::{resolve_api_protocol, resolve_custom_url, ExplanationApiProtocol};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiProviderConfig {
    pub(crate) provider: String,
    pub(crate) api_url: String,
    pub(crate) protocol: ExplanationApiProtocol,
    pub(crate) model: String,
}

fn provider_scoped_key(name: &str, provider: &str) -> String {
    format!("{name}__{provider}")
}

async fn scoped_ai_setting(pool: &crate::db::DbPool, provider: &str, name: &str) -> Option<String> {
    let scoped = provider_scoped_key(name, provider);
    if let Some(value) = super::get_ai_setting(pool, &scoped).await {
        Some(value)
    } else {
        super::get_ai_setting(pool, name).await
    }
}

pub(crate) async fn resolve_ai_provider_config(pool: &crate::db::DbPool) -> AiProviderConfig {
    let provider = super::get_ai_setting(pool, "ai_provider")
        .await
        .unwrap_or_else(|| "claude".to_string());
    let explicit_protocol = default_protocol_for_provider(&provider)
        .map(str::to_string)
        .or(scoped_ai_setting(pool, &provider, "ai_protocol").await);
    let model = scoped_ai_setting(pool, &provider, "ai_model")
        .await
        .unwrap_or_else(|| default_model_for_provider(&provider).to_string());
    let raw_api_url = normalize_provider_api_url(
        &provider,
        &scoped_ai_setting(pool, &provider, "ai_api_url")
            .await
            .unwrap_or_else(|| default_api_url_for_provider(&provider).to_string()),
    );
    let detected_protocol = resolve_api_protocol(&raw_api_url, explicit_protocol.as_deref());
    let api_url = if provider == "custom" {
        let custom_base = scoped_ai_setting(pool, &provider, "ai_custom_base_url")
            .await
            .unwrap_or(raw_api_url);
        resolve_custom_url(&custom_base, detected_protocol)
    } else {
        raw_api_url
    };
    let protocol = resolve_api_protocol(&api_url, explicit_protocol.as_deref());

    AiProviderConfig {
        provider,
        api_url,
        protocol,
        model,
    }
}

fn default_model_for_provider(provider: &str) -> &'static str {
    match provider {
        "glm" => "glm-5",
        "minimax" => "MiniMax-M2.7",
        "kimi" => "kimi-k2.5",
        "deepseek" => "deepseek-v4-flash",
        "openrouter" => "anthropic/claude-sonnet-4.6",
        "custom" => "",
        _ => "claude-sonnet-4-20250514",
    }
}

fn default_api_url_for_provider(provider: &str) -> &'static str {
    match provider {
        "glm" => "https://api.z.ai/api/anthropic/v1/messages",
        "minimax" => "https://api.minimax.io/anthropic/v1/messages",
        "kimi" => "https://api.moonshot.cn/anthropic/v1/messages",
        "deepseek" => "https://api.deepseek.com/anthropic/v1/messages",
        "openrouter" => "https://openrouter.ai/api/v1/chat/completions",
        "custom" => "",
        _ => "https://api.anthropic.com/v1/messages",
    }
}

fn default_protocol_for_provider(provider: &str) -> Option<&'static str> {
    match provider {
        "openrouter" => Some("openai"),
        _ => None,
    }
}

fn normalize_provider_api_url(provider: &str, api_url: &str) -> String {
    let normalized = api_url.trim().trim_end_matches('/');
    if provider == "openrouter" && normalized == "https://openrouter.ai/api/v1/messages" {
        default_api_url_for_provider(provider).to_string()
    } else {
        api_url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::test_support::mem_pool as setup_test_db;

    #[tokio::test]
    async fn resolver_falls_back_to_legacy_unsuffixed_settings() {
        let pool = setup_test_db().await;
        db::set_setting(&pool, "ai_provider", "deepseek")
            .await
            .unwrap();
        db::set_setting(&pool, "ai_model", "legacy-model")
            .await
            .unwrap();
        db::set_setting(
            &pool,
            "ai_api_url",
            "https://legacy.example.com/v1/messages",
        )
        .await
        .unwrap();

        let config = resolve_ai_provider_config(&pool).await;

        assert_eq!(config.model, "legacy-model");
        assert_eq!(config.api_url, "https://legacy.example.com/v1/messages");
        assert_eq!(config.protocol, ExplanationApiProtocol::AnthropicCompatible);
    }

    #[tokio::test]
    async fn resolver_prefers_provider_scoped_settings() {
        let pool = setup_test_db().await;
        db::set_setting(&pool, "ai_provider", "custom")
            .await
            .unwrap();
        db::set_setting(&pool, "ai_model", "legacy-model")
            .await
            .unwrap();
        db::set_setting(&pool, "ai_model__custom", "custom-model")
            .await
            .unwrap();
        db::set_setting(
            &pool,
            "ai_custom_base_url__custom",
            "https://proxy.example.com/v1",
        )
        .await
        .unwrap();
        db::set_setting(&pool, "ai_protocol__custom", "openai")
            .await
            .unwrap();

        let config = resolve_ai_provider_config(&pool).await;

        assert_eq!(config.model, "custom-model");
        assert_eq!(
            config.api_url,
            "https://proxy.example.com/v1/chat/completions"
        );
        assert_eq!(config.protocol, ExplanationApiProtocol::OpenAiCompatible);
    }

    #[tokio::test]
    async fn resolver_uses_openrouter_openai_chat_completions_by_default() {
        let pool = setup_test_db().await;
        db::set_setting(&pool, "ai_provider", "openrouter")
            .await
            .unwrap();

        let config = resolve_ai_provider_config(&pool).await;

        assert_eq!(config.model, "anthropic/claude-sonnet-4.6");
        assert_eq!(
            config.api_url,
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(config.protocol, ExplanationApiProtocol::OpenAiCompatible);
    }

    #[tokio::test]
    async fn resolver_forces_openrouter_openai_protocol_over_legacy_setting() {
        let pool = setup_test_db().await;
        db::set_setting(&pool, "ai_provider", "openrouter")
            .await
            .unwrap();
        db::set_setting(&pool, "ai_protocol__openrouter", "anthropic")
            .await
            .unwrap();

        let config = resolve_ai_provider_config(&pool).await;

        assert_eq!(
            config.api_url,
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(config.protocol, ExplanationApiProtocol::OpenAiCompatible);
    }
}
