//! One-shot (non-streaming) explanation flow and lightweight AI connection
//! checks. Requests can target Anthropic-compatible `/messages` endpoints or
//! OpenAI-compatible `/chat/completions` endpoints based on resolved provider
//! configuration.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::error::{format_reqwest_error, AiProviderError};
use super::prompt::{build_explanation_prompt, truncate_content, ExplanationApiProtocol};

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type", default)]
    block_type: String,
    #[serde(default)]
    text: String,
}

#[cfg_attr(feature = "ipc-codegen", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionTestResult {
    pub ok: bool,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

fn request_builder(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    protocol: ExplanationApiProtocol,
) -> reqwest::RequestBuilder {
    let mut request = client
        .post(api_url)
        .header("content-type", "application/json");
    if protocol.is_anthropic_compatible() {
        request = request
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01");
    } else {
        request = request.header("authorization", format!("Bearer {}", api_key));
    }
    request
}

fn parse_response_text(body: &str) -> Option<String> {
    if let Ok(claude_resp) = serde_json::from_str::<ClaudeResponse>(body) {
        if let Some(block) = claude_resp
            .content
            .iter()
            .find(|b| b.block_type.is_empty() || b.block_type == "text")
        {
            if !block.text.is_empty() {
                return Some(block.text.clone());
            }
        }
    }

    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|val| {
            val.get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .map(ToString::to_string)
        })
}

fn error_code_for_status(status_code: u16) -> &'static str {
    if status_code == 401 || status_code == 403 {
        super::AI_INVALID_API_KEY
    } else if status_code == 429 {
        super::AI_RATE_LIMIT
    } else {
        super::AI_RESPONSE_ERROR
    }
}

fn message_for_status(status: reqwest::StatusCode) -> String {
    let status_code = status.as_u16();
    if status_code == 401 || status_code == 403 {
        "The API key is invalid or does not have permission for this provider.".to_string()
    } else if status_code == 429 {
        "The provider rate limited the request. Try again later or reduce AI Tag concurrency."
            .to_string()
    } else {
        format!("The AI provider returned HTTP {status}.")
    }
}

pub(crate) async fn explain_skill(
    pool: &crate::db::DbPool,
    secrets: &dyn crate::secrets::SecretStore,
    content: String,
) -> Result<String, AiProviderError> {
    let config = super::resolve_ai_provider_config(pool).await;
    let api_key = super::get_ai_api_key_for_provider(pool, secrets, &config.provider)
        .await?
        .ok_or_else(|| {
            AiProviderError::MissingApiKey(super::coded_error(
                super::AI_MISSING_API_KEY,
                "Configure an AI API key in Settings before requesting an AI explanation.",
            ))
        })?;

    let client = {
        let builder = reqwest::Client::builder()
            .user_agent(crate::commands::APP_USER_AGENT)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60));
        #[cfg(test)]
        let builder = builder.no_proxy();
        builder.build().map_err(|e| {
            AiProviderError::Http(super::coded_error_with_details(
                super::AI_CLIENT_BUILD_FAILED,
                "Failed to initialize the AI HTTP client.",
                e.to_string(),
            ))
        })?
    };

    let truncated = truncate_content(&content);
    let prompt = build_explanation_prompt(&truncated, "zh");
    let request = ClaudeRequest {
        model: config.model,
        max_tokens: 1024,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: prompt,
        }],
    };

    let resp = request_builder(&client, &config.api_url, &api_key, config.protocol)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            AiProviderError::Http(super::coded_error_with_details(
                super::AI_REQUEST_FAILED,
                "AI request failed.",
                format_reqwest_error(&e),
            ))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let status_code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(AiProviderError::from_status(
            status_code,
            super::coded_error_with_details(
                error_code_for_status(status_code),
                message_for_status(status),
                format!("HTTP {status}: {body}"),
            ),
        ));
    }

    let body = resp.text().await.map_err(|e| {
        AiProviderError::Http(super::coded_error_with_details(
            super::AI_RESPONSE_READ_FAILED,
            "Failed to read the AI response.",
            e.to_string(),
        ))
    })?;

    if let Some(text) = parse_response_text(&body) {
        return Ok(text);
    }

    Err(AiProviderError::Parse(super::coded_error_with_details(
        super::AI_RESPONSE_PARSE_FAILED,
        "Unable to parse the AI response.",
        &body[..body.len().min(500)],
    )))
}

pub(crate) async fn test_ai_connection(
    pool: &crate::db::DbPool,
    secrets: &dyn crate::secrets::SecretStore,
) -> Result<AiConnectionTestResult, AiProviderError> {
    let config = super::resolve_ai_provider_config(pool).await;
    if config.api_url.trim().is_empty() {
        return Ok(AiConnectionTestResult {
            ok: false,
            msg: "Configure an AI API URL before testing the connection.".to_string(),
            code: Some(super::AI_REQUEST_FAILED.to_string()),
            details: Some("Resolved AI API URL is empty.".to_string()),
        });
    }
    if config.model.trim().is_empty() {
        return Ok(AiConnectionTestResult {
            ok: false,
            msg: "Configure an AI model before testing the connection.".to_string(),
            code: Some(super::AI_REQUEST_FAILED.to_string()),
            details: Some("Resolved AI model is empty.".to_string()),
        });
    }

    let Some(api_key) = super::get_ai_api_key_for_provider(pool, secrets, &config.provider).await?
    else {
        return Ok(AiConnectionTestResult {
            ok: false,
            msg: "Configure an AI API key in Settings before testing the connection.".to_string(),
            code: Some(super::AI_MISSING_API_KEY.to_string()),
            details: Some(format!(
                "No API key saved for provider '{}'.",
                config.provider
            )),
        });
    };

    let client = {
        let builder = reqwest::Client::builder()
            .user_agent(crate::commands::APP_USER_AGENT)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15));
        #[cfg(test)]
        let builder = builder.no_proxy();
        builder.build().map_err(|e| {
            AiProviderError::Http(super::coded_error_with_details(
                super::AI_CLIENT_BUILD_FAILED,
                "Failed to initialize the AI HTTP client.",
                e.to_string(),
            ))
        })?
    };

    let request = ClaudeRequest {
        model: config.model.clone(),
        max_tokens: 1,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: "ping".to_string(),
        }],
    };

    let resp = match request_builder(&client, &config.api_url, &api_key, config.protocol)
        .json(&request)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(error) => {
            return Ok(AiConnectionTestResult {
                ok: false,
                msg: "AI connection test failed.".to_string(),
                code: Some(super::AI_REQUEST_FAILED.to_string()),
                details: Some(format_reqwest_error(&error)),
            });
        }
    };

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Ok(AiConnectionTestResult {
            ok: false,
            msg: message_for_status(status),
            code: Some(error_code_for_status(status.as_u16()).to_string()),
            details: Some(format!("HTTP {status}: {body}")),
        });
    }

    if parse_response_text(&body).is_some()
        || serde_json::from_str::<serde_json::Value>(&body).is_ok()
    {
        Ok(AiConnectionTestResult {
            ok: true,
            msg: format!("AI connection test succeeded for {}.", config.provider),
            code: None,
            details: Some(format!("{} responded with HTTP {status}.", config.api_url)),
        })
    } else {
        Ok(AiConnectionTestResult {
            ok: false,
            msg: "The AI provider returned an unreadable response.".to_string(),
            code: Some(super::AI_RESPONSE_PARSE_FAILED.to_string()),
            details: Some(body[..body.len().min(500)].to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::secrets::{MockSecretStore, AI_API_KEY_SECRET_KEY};
    use crate::test_support::mem_pool as setup_test_db;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn spawn_connection_test_server(body: &'static str) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("addr");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_task = Arc::clone(&requests);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let requests = Arc::clone(&requests_for_task);
                tokio::spawn(async move {
                    let mut buffer = [0_u8; 4096];
                    let bytes = socket.read(&mut buffer).await.unwrap_or(0);
                    requests
                        .lock()
                        .expect("requests")
                        .push(String::from_utf8_lossy(&buffer[..bytes]).to_string());
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        (format!("http://{address}"), requests)
    }

    #[tokio::test]
    async fn test_ai_connection_accepts_anthropic_minimal_response() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::with_value(AI_API_KEY_SECRET_KEY, "sk-test");
        let (base_url, requests) =
            spawn_connection_test_server(r#"{"content":[{"type":"text","text":"ok"}]}"#).await;
        db::set_setting(&pool, "ai_provider", "custom")
            .await
            .unwrap();
        db::set_setting(&pool, "ai_model__custom", "test-model")
            .await
            .unwrap();
        db::set_setting(&pool, "ai_custom_base_url__custom", &base_url)
            .await
            .unwrap();
        db::set_setting(&pool, "ai_protocol__custom", "anthropic")
            .await
            .unwrap();

        let result = test_ai_connection(&pool, &secrets).await.unwrap();

        assert!(result.ok, "{result:?}");
        let request = requests.lock().expect("requests").join("\n");
        assert!(request.contains("POST /v1/messages"), "{request}");
        assert!(request.contains("x-api-key: sk-test"), "{request}");
        assert!(request.contains(r#""max_tokens":1"#), "{request}");
    }

    #[tokio::test]
    async fn test_ai_connection_accepts_openai_minimal_response() {
        let pool = setup_test_db().await;
        let secrets = MockSecretStore::with_value("ai_api_key__custom", "sk-openai");
        let (base_url, requests) =
            spawn_connection_test_server(r#"{"choices":[{"message":{"content":"ok"}}]}"#).await;
        db::set_setting(&pool, "ai_provider", "custom")
            .await
            .unwrap();
        db::set_setting(&pool, "ai_model__custom", "test-model")
            .await
            .unwrap();
        db::set_setting(
            &pool,
            "ai_custom_base_url__custom",
            &format!("{base_url}/v1"),
        )
        .await
        .unwrap();
        db::set_setting(&pool, "ai_protocol__custom", "openai")
            .await
            .unwrap();

        let result = test_ai_connection(&pool, &secrets).await.unwrap();

        assert!(result.ok, "{result:?}");
        let request = requests.lock().expect("requests").join("\n");
        assert!(request.contains("POST /v1/chat/completions"), "{request}");
        assert!(
            request.contains("authorization: Bearer sk-openai"),
            "{request}"
        );
        assert!(request.contains(r#""max_tokens":1"#), "{request}");
    }
}
