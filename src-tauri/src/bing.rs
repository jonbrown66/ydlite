use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::errors::AppError;
use crate::subtitle_types::SubtitleSegment;

const AUTH_URL: &str = "https://edge.microsoft.com/translate/auth";
const TRANSLATE_URL: &str = "https://api-edge.cognitive.microsofttranslator.com/translate";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0";
const BATCH_SIZE: usize = 80;

#[derive(Serialize)]
struct TextRequest<'a> {
    #[serde(rename = "Text")]
    text: &'a str,
}

#[derive(Debug, Deserialize)]
struct TranslationItem {
    #[serde(default, rename = "detectedLanguage")]
    detected_language: Option<DetectedLanguage>,
    translations: Vec<TranslatedText>,
}

#[derive(Debug, Deserialize)]
struct DetectedLanguage {
    language: String,
}

#[derive(Debug, Deserialize)]
struct TranslatedText {
    text: String,
}

pub struct BingTranslationResult {
    pub detected_language: Option<String>,
}

pub struct BingTranslator {
    client: Client,
    token: String,
}

impl BingTranslator {
    pub async fn new() -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|error| AppError::user("无法初始化必应翻译。", error.to_string()))?;
        let token = fetch_token(&client).await?;
        Ok(Self { client, token })
    }

    pub async fn translate(
        &mut self,
        segments: &mut [SubtitleSegment],
        target_language: &str,
        cancel: &CancellationToken,
        mut on_batch: impl FnMut(usize, usize),
    ) -> Result<BingTranslationResult, AppError> {
        let total = segments.len().div_ceil(BATCH_SIZE);
        let mut detected_language = None;
        for (batch, chunk) in segments.chunks_mut(BATCH_SIZE).enumerate() {
            if cancel.is_cancelled() {
                return Err(AppError::user("字幕任务已取消。", "cancelled"));
            }
            let texts: Vec<TextRequest<'_>> = chunk
                .iter()
                .map(|segment| TextRequest {
                    text: truncate_text(&segment.source_text, 5_000),
                })
                .collect();
            let response = self
                .send_with_retry(&texts, target_language, cancel)
                .await?;
            if response.len() != chunk.len() {
                return Err(AppError::user(
                    "必应返回的字幕数量不一致，结果未应用。",
                    format!("Expected {}, received {}", chunk.len(), response.len()),
                ));
            }
            for (segment, item) in chunk.iter_mut().zip(response) {
                let text = item
                    .translations
                    .first()
                    .map(|translation| translation.text.trim())
                    .unwrap_or_default();
                if text.is_empty() {
                    return Err(AppError::user(
                        "必应返回了空译文，结果未应用。",
                        segment.id.clone(),
                    ));
                }
                if detected_language.is_none() {
                    detected_language = item.detected_language.map(|language| language.language);
                }
                segment.translated_text = Some(text.to_string());
            }
            on_batch(batch + 1, total);
        }
        Ok(BingTranslationResult { detected_language })
    }

    async fn send_with_retry(
        &mut self,
        texts: &[TextRequest<'_>],
        target_language: &str,
        cancel: &CancellationToken,
    ) -> Result<Vec<TranslationItem>, AppError> {
        for attempt in 0..3 {
            if cancel.is_cancelled() {
                return Err(AppError::user("字幕任务已取消。", "cancelled"));
            }
            let response = self
                .client
                .post(TRANSLATE_URL)
                .query(&[
                    ("to", target_language),
                    ("api-version", "3.0"),
                    ("includeSentenceLength", "true"),
                ])
                .header(reqwest::header::USER_AGENT, USER_AGENT)
                .bearer_auth(&self.token)
                .json(texts)
                .send()
                .await
                .map_err(|error| AppError::user("必应翻译网络请求失败。", error.to_string()))?;
            let status = response.status();
            if status.is_success() {
                return response
                    .json::<Vec<TranslationItem>>()
                    .await
                    .map_err(|error| AppError::user("必应翻译响应无法解析。", error.to_string()));
            }
            if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
                self.token = fetch_token(&self.client).await?;
                continue;
            }
            if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                sleep(Duration::from_millis(500 * (1_u64 << attempt))).await;
                continue;
            }
            let detail = response.text().await.unwrap_or_default();
            return Err(AppError::user(
                "必应免费翻译拒绝了请求。",
                format!("HTTP {status}: {}", truncate_text(&detail, 500)),
            ));
        }
        Err(AppError::user(
            "必应免费翻译暂时不可用，请稍后重试。",
            "Retry limit reached",
        ))
    }
}

async fn fetch_token(client: &Client) -> Result<String, AppError> {
    let response = client
        .get(AUTH_URL)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|error| AppError::user("无法连接必应翻译。", error.to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::user(
            "无法获取必应翻译临时凭据。",
            format!("HTTP {}", response.status()),
        ));
    }
    let token = response
        .text()
        .await
        .map_err(|error| AppError::user("无法读取必应翻译临时凭据。", error.to_string()))?;
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(AppError::user("必应翻译没有返回临时凭据。", "Empty token"));
    }
    Ok(token)
}

fn truncate_text(value: &str, max_chars: usize) -> &str {
    value
        .char_indices()
        .nth(max_chars)
        .map(|(index, _)| &value[..index])
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_without_breaking_unicode() {
        assert_eq!(truncate_text("你好世界", 2), "你好");
    }
}
