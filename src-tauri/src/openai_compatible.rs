use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::errors::AppError;
use crate::subtitle_types::SubtitleSegment;

const KEYRING_SERVICE: &str = "com.ydlite.desktop";
const KEYRING_USER: &str = "openai-compatible-api-key";

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Value,
}

pub struct OpenAiCompatibleClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatibleClient {
    pub fn new(base_url: &str, api_key: String, model: &str) -> Result<Self, AppError> {
        validate_base_url(base_url)?;
        if model.trim().is_empty() {
            return Err(AppError::user(
                "请填写 OpenAI 兼容模型名称。",
                "Empty model",
            ));
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|error| AppError::user("无法初始化自定义 AI 客户端。", error.to_string()))?;
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model: model.trim().to_string(),
        })
    }

    pub async fn test_connection(&self) -> Result<String, AppError> {
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|error| AppError::user("自定义 AI 连接失败。", error.to_string()))?;
        if !response.status().is_success() {
            return Err(AppError::user(
                "自定义 AI 拒绝了连接。",
                format!("HTTP {}", response.status()),
            ));
        }
        Ok("OpenAI 兼容接口已连接。".into())
    }

    pub async fn translate(
        &self,
        segments: &mut [SubtitleSegment],
        target_language: &str,
        cancel: &CancellationToken,
        mut on_batch: impl FnMut(usize, usize),
    ) -> Result<(), AppError> {
        let total = segments.len().div_ceil(50);
        for (batch, chunk) in segments.chunks_mut(50).enumerate() {
            if cancel.is_cancelled() {
                return Err(AppError::user("字幕任务已取消。", "cancelled"));
            }
            let items: Vec<Value> = chunk
                .iter()
                .map(|segment| json!({"id": segment.id, "text": segment.source_text}))
                .collect();
            let body = json!({
                "model": self.model,
                "temperature": 0,
                "messages": [
                    {
                        "role": "system",
                        "content": format!(
                            "Translate every subtitle into {target_language}. Return strict JSON only: \
                             {{\"items\":[{{\"id\":\"same id\",\"text\":\"translation\"}}]}}. \
                             Keep exactly the same ids, count and order. Do not add explanations."
                        )
                    },
                    {"role": "user", "content": serde_json::to_string(&items)?}
                ]
            });
            let response = self
                .client
                .post(format!("{}/chat/completions", self.base_url))
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await
                .map_err(|error| AppError::user("自定义 AI 翻译请求失败。", error.to_string()))?;
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(AppError::user(
                    "自定义 AI 翻译失败。",
                    format!("HTTP {status}: {}", truncate(&text)),
                ));
            }
            let response: ChatResponse = serde_json::from_str(&text)
                .map_err(|error| AppError::user("自定义 AI 响应无法解析。", error.to_string()))?;
            let content = response
                .choices
                .first()
                .map(|choice| content_text(&choice.message.content))
                .transpose()?
                .ok_or_else(|| AppError::user("自定义 AI 没有返回译文。", "Missing choice"))?;
            apply_items(chunk, &content)?;
            on_batch(batch + 1, total);
        }
        Ok(())
    }
}

fn content_text(value: &Value) -> Result<String, AppError> {
    if let Some(text) = value.as_str() {
        return Ok(text.to_string());
    }
    if let Some(items) = value.as_array() {
        let text = items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("");
        if !text.is_empty() {
            return Ok(text);
        }
    }
    Err(AppError::user(
        "自定义 AI 返回了不支持的文本格式。",
        value.to_string(),
    ))
}

fn apply_items(segments: &mut [SubtitleSegment], content: &str) -> Result<(), AppError> {
    let trimmed = content.trim();
    let json_text = if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };
    let value: Value = serde_json::from_str(json_text)
        .map_err(|error| AppError::user("自定义 AI 返回的译文 JSON 无效。", error.to_string()))?;
    let items = value["items"]
        .as_array()
        .ok_or_else(|| AppError::user("自定义 AI 译文缺少 items。", truncate(json_text)))?;
    if items.len() != segments.len() {
        return Err(AppError::user(
            "自定义 AI 改变了字幕数量，结果未应用。",
            format!("Expected {}, received {}", segments.len(), items.len()),
        ));
    }
    for (segment, item) in segments.iter_mut().zip(items) {
        if item["id"].as_str() != Some(&segment.id) {
            return Err(AppError::user(
                "自定义 AI 改变了字幕顺序，结果未应用。",
                segment.id.clone(),
            ));
        }
        let text = item["text"].as_str().unwrap_or_default().trim();
        if text.is_empty() {
            return Err(AppError::user(
                "自定义 AI 返回了空译文，结果未应用。",
                segment.id.clone(),
            ));
        }
        segment.translated_text = Some(text.to_string());
    }
    Ok(())
}

pub fn validate_base_url(value: &str) -> Result<(), AppError> {
    let url = Url::parse(value.trim())
        .map_err(|error| AppError::user("自定义 API Base URL 无效。", error.to_string()))?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if url.scheme() != "https" && !(url.scheme() == "http" && local) {
        return Err(AppError::user(
            "远程自定义 API 必须使用 HTTPS。",
            value.to_string(),
        ));
    }
    Ok(())
}

fn truncate(value: &str) -> String {
    value.chars().take(500).collect()
}

#[cfg(target_os = "windows")]
pub fn save_api_key(value: Option<&str>) -> Result<(), AppError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| AppError::user("无法访问 Windows 凭据管理器。", error.to_string()))?;
    match value.map(str::trim) {
        Some(value) if !value.is_empty() => entry
            .set_password(value)
            .map_err(|error| AppError::user("无法保存自定义 AI API Key。", error.to_string())),
        _ => {
            let _ = entry.delete_credential();
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
pub fn load_api_key() -> Result<String, AppError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| AppError::user("无法访问 Windows 凭据管理器。", error.to_string()))?;
    entry.get_password().map_err(|error| {
        AppError::user(
            "尚未配置自定义 AI API Key。",
            format!("Credential unavailable: {error}"),
        )
    })
}

#[cfg(not(target_os = "windows"))]
pub fn save_api_key(_value: Option<&str>) -> Result<(), AppError> {
    Err(AppError::user(
        "当前平台暂不支持安全保存自定义 AI Key。",
        "Windows Credential Manager is required",
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn load_api_key() -> Result<String, AppError> {
    Err(AppError::user(
        "当前平台暂不支持自定义 AI Key。",
        "Windows Credential Manager is required",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_insecure_remote_url_but_allows_localhost() {
        assert!(validate_base_url("http://example.com/v1").is_err());
        assert!(validate_base_url("http://127.0.0.1:11434/v1").is_ok());
        assert!(validate_base_url("https://example.com/v1").is_ok());
    }

    #[test]
    fn applies_only_matching_translation_items() {
        let mut segments = vec![SubtitleSegment {
            id: "a".into(),
            start_ms: 0,
            end_ms: 1_000,
            source_text: "hello".into(),
            translated_text: None,
        }];
        apply_items(&mut segments, r#"{"items":[{"id":"a","text":"你好"}]}"#).unwrap();
        assert_eq!(segments[0].translated_text.as_deref(), Some("你好"));
    }
}
