use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream;
use reqwest::{header::HeaderMap, Body, Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::errors::AppError;
use crate::subtitle_types::{GeminiSegment, GeminiTranscriptResult, SubtitleUsage};

const INTERACTIONS_URL: &str = "https://generativelanguage.googleapis.com/v1/interactions";
const FILES_UPLOAD_URL: &str = "https://generativelanguage.googleapis.com/upload/v1beta/files";
const FILES_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const KEYRING_SERVICE: &str = "com.ydlite.desktop";
const KEYRING_USER: &str = "gemini-auth-key";
const UPLOAD_ATTEMPTS: usize = 3;
const UPLOAD_START_TIMEOUT: Duration = Duration::from_secs(15);
const UPLOAD_STREAM_CHUNK_BYTES: usize = 64 * 1024;

pub type UploadProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;
pub type RetryWaitCallback = Arc<dyn Fn(usize, u64) + Send + Sync>;

#[derive(Debug, Deserialize)]
struct UploadedFileEnvelope {
    file: UploadedFile,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFile {
    pub name: String,
    pub uri: String,
}

#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    api_key: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .map_err(|error| AppError::user("无法初始化 Gemini 客户端。", error.to_string()))?;
        Ok(Self { client, api_key })
    }

    pub async fn test_connection(&self) -> Result<String, AppError> {
        let body = json!({
            "model": "gemini-3.1-flash-lite",
            "input": "Reply with the single word READY.",
            "store": false,
            "generation_config": {
                "max_output_tokens": 8,
                "temperature": 0
            }
        });
        let value = self
            .send_interaction(body, &CancellationToken::new(), None)
            .await?;
        extract_output_text(&value)
            .map(|_| "Gemini API 已连接。".to_string())
            .ok_or_else(|| AppError::user("Gemini 返回了无法识别的响应。", "Missing model output"))
    }

    pub async fn upload_audio(
        &self,
        path: &Path,
        cancel: &CancellationToken,
        on_progress: UploadProgressCallback,
        mut on_retry: impl FnMut(usize),
    ) -> Result<UploadedFile, AppError> {
        let bytes = tokio::fs::read(path).await?;
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("ydlite-audio.m4a");
        let upload_timeout = upload_timeout(bytes.len());
        let mut last_error = None;
        for attempt in 0..UPLOAD_ATTEMPTS {
            if cancel.is_cancelled() {
                return Err(cancelled());
            }
            if attempt > 0 {
                on_retry(attempt + 1);
                tokio::select! {
                    _ = cancel.cancelled() => return Err(cancelled()),
                    _ = sleep(Duration::from_secs(attempt as u64)) => {}
                }
            }
            match self
                .upload_audio_once(
                    &bytes,
                    display_name,
                    upload_timeout,
                    cancel,
                    on_progress.clone(),
                )
                .await
            {
                Ok(file) => return Ok(file),
                Err(error)
                    if attempt + 1 < UPLOAD_ATTEMPTS && upload_error_is_retryable(&error) =>
                {
                    last_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error.unwrap_or_else(|| {
            AppError::user(
                "Gemini 文件上传失败。",
                "Upload retry loop ended unexpectedly",
            )
        }))
    }

    async fn upload_audio_once(
        &self,
        bytes: &[u8],
        display_name: &str,
        upload_timeout: Duration,
        cancel: &CancellationToken,
        on_progress: UploadProgressCallback,
    ) -> Result<UploadedFile, AppError> {
        let start = self
            .client
            .post(FILES_UPLOAD_URL)
            .timeout(UPLOAD_START_TIMEOUT)
            .header("x-goog-api-key", &self.api_key)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header("X-Goog-Upload-Header-Content-Length", bytes.len())
            .header("X-Goog-Upload-Header-Content-Type", "audio/m4a")
            .json(&json!({ "file": { "display_name": display_name } }));
        let start_response = cancellable(cancel, start.send()).await?;
        ensure_success(start_response.status(), start_response.headers(), None).await?;
        let upload_url = start_response
            .headers()
            .get("X-Goog-Upload-URL")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                AppError::user(
                    "Gemini 文件上传初始化失败。",
                    "Missing X-Goog-Upload-URL response header",
                )
            })?
            .to_string();
        drop(start_response);

        let total_bytes = bytes.len();
        let upload_bytes = bytes.to_vec();
        let upload_stream = stream::unfold(
            (upload_bytes, 0_usize, on_progress),
            |(bytes, offset, progress)| async move {
                if offset >= bytes.len() {
                    return None;
                }
                let end = (offset + UPLOAD_STREAM_CHUNK_BYTES).min(bytes.len());
                let chunk = bytes[offset..end].to_vec();
                progress(end as u64, bytes.len() as u64);
                Some((Ok::<Vec<u8>, std::io::Error>(chunk), (bytes, end, progress)))
            },
        );
        let upload = self
            .client
            .post(upload_url)
            .timeout(upload_timeout)
            .header("x-goog-api-key", &self.api_key)
            .header("Content-Length", total_bytes)
            .header("X-Goog-Upload-Offset", "0")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .header("Content-Type", "audio/m4a")
            .body(Body::wrap_stream(upload_stream));
        let response = cancellable(cancel, upload.send()).await?;
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await.unwrap_or_default();
        ensure_success(status, &headers, Some(&text)).await?;
        serde_json::from_str::<UploadedFileEnvelope>(&text)
            .map(|value| value.file)
            .map_err(|error| {
                AppError::user(
                    "Gemini 文件上传响应无法解析。",
                    format!("{error}: {}", truncate(&text)),
                )
            })
    }

    pub async fn delete_file(&self, name: &str) {
        let url = format!("{FILES_BASE_URL}/{name}");
        let _ = self
            .client
            .delete(url)
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await;
    }

    pub async fn transcribe(
        &self,
        file: &UploadedFile,
        model: &str,
        duration_ms: u64,
        translate: bool,
        target_language: Option<&str>,
        cancel: &CancellationToken,
        on_retry_wait: Option<RetryWaitCallback>,
    ) -> Result<(GeminiTranscriptResult, SubtitleUsage), AppError> {
        let translation_instruction = if translate {
            format!(
                "Also translate every segment into {} and populate translatedText.",
                target_language.unwrap_or("zh-CN")
            )
        } else {
            "Do not include translatedText.".to_string()
        };
        let prompt = format!(
            "Transcribe the supplied audio faithfully. Do not summarize, rewrite, or invent speech. \
             Return consecutive subtitle segments, normally 1 to 8 seconds each. Times are relative \
             to this audio chunk and expressed as integer milliseconds. The chunk duration is \
             {duration_ms} ms. {translation_instruction}"
        );
        let response_format = json_response_format(transcript_schema(translate));
        let body = json!({
            "model": model,
            "input": [user_input_step(vec![
                json!({ "type": "text", "text": prompt }),
                json!({ "type": "audio", "uri": file.uri, "mime_type": "audio/m4a" })
            ])],
            "response_format": response_format,
            "store": false,
            "generation_config": {
                "temperature": 0,
                "max_output_tokens": 32768
            }
        });
        let value = self.send_interaction(body, cancel, on_retry_wait).await?;
        let usage = parse_usage(&value, model);
        let text = extract_output_text(&value).ok_or_else(|| {
            AppError::user(
                "Gemini 未返回转录结果。",
                format!("Response status: {}", value["status"]),
            )
        })?;
        let mut result: GeminiTranscriptResult = serde_json::from_str(&text).map_err(|error| {
            AppError::user(
                "Gemini 返回的字幕 JSON 无效。",
                format!("{error}: {}", truncate(&text)),
            )
        })?;
        normalize_gemini_result(&mut result, duration_ms);
        validate_gemini_result(&result, duration_ms, translate)?;
        Ok((result, usage))
    }

    pub async fn transform_segments(
        &self,
        model: &str,
        task: &str,
        target_language: Option<&str>,
        segments: &Value,
        cancel: &CancellationToken,
    ) -> Result<(Value, SubtitleUsage), AppError> {
        let prompt = match task {
            "translate" => format!(
                "Translate each sourceText into {}. Return exactly the same ids and put the translation \
                 in text. Do not merge, split, omit, or reorder items.",
                target_language.unwrap_or("zh-CN")
            ),
            _ => "Correct punctuation, obvious ASR mistakes, and proper nouns in each sourceText. \
                  Preserve meaning. Return exactly the same ids and put the corrected source in text. \
                  Do not merge, split, omit, or reorder items."
                .to_string(),
        };
        let response_format = json_response_format(json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "text": { "type": "string" }
                        },
                        "required": ["id", "text"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["items"],
            "additionalProperties": false
        }));
        let body = json!({
            "model": model,
            "input": format!("{prompt}\n\nINPUT JSON:\n{segments}"),
            "response_format": response_format,
            "store": false,
            "generation_config": {
                "temperature": 0,
                "max_output_tokens": 32768
            }
        });
        let value = self.send_interaction(body, cancel, None).await?;
        let usage = parse_usage(&value, model);
        let text = extract_output_text(&value)
            .ok_or_else(|| AppError::user("Gemini 未返回文本处理结果。", "Missing model output"))?;
        let parsed: Value = serde_json::from_str(&text).map_err(|error| {
            AppError::user(
                "Gemini 返回的文本处理 JSON 无效。",
                format!("{error}: {}", truncate(&text)),
            )
        })?;
        Ok((parsed, usage))
    }

    async fn send_interaction(
        &self,
        body: Value,
        cancel: &CancellationToken,
        on_retry_wait: Option<RetryWaitCallback>,
    ) -> Result<Value, AppError> {
        let mut last_error = None;
        for attempt in 0..3 {
            if cancel.is_cancelled() {
                return Err(cancelled());
            }
            let response = cancellable(
                cancel,
                self.client
                    .post(INTERACTIONS_URL)
                    .header("x-goog-api-key", &self.api_key)
                    .json(&body)
                    .send(),
            )
            .await?;
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            if status.is_success() {
                return serde_json::from_str(&text).map_err(|error| {
                    AppError::user(
                        "Gemini 响应无法解析。",
                        format!("{error}: {}", truncate(&text)),
                    )
                });
            }
            let retryable = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
            let error = http_error(status, &text);
            if !retryable || attempt == 2 {
                return Err(error);
            }
            last_error = Some(error);
            let wait = retry_after(&headers).unwrap_or(2_u64.pow(attempt + 1));
            if let Some(callback) = &on_retry_wait {
                callback((attempt + 2) as usize, wait.min(30));
            }
            tokio::select! {
                _ = cancel.cancelled() => return Err(cancelled()),
                _ = sleep(Duration::from_secs(wait.min(30))) => {}
            }
        }
        Err(last_error.unwrap_or_else(|| {
            AppError::user("Gemini 请求失败。", "Retry loop completed unexpectedly")
        }))
    }
}

fn upload_timeout(bytes: usize) -> Duration {
    const MIN_BYTES_PER_SECOND: usize = 128 * 1024;
    let transfer_seconds = bytes.div_ceil(MIN_BYTES_PER_SECOND) as u64;
    Duration::from_secs((30 + transfer_seconds).clamp(35, 120))
}

fn upload_error_is_retryable(error: &AppError) -> bool {
    match error {
        AppError::User { message, detail } => {
            message == "Gemini 网络请求失败。"
                || detail.contains("HTTP 408")
                || detail.contains("HTTP 429")
                || (500..=599).any(|status| detail.contains(&format!("HTTP {status}")))
        }
        AppError::Io(_) => true,
        AppError::Json(_) => false,
    }
}

async fn cancellable<T>(
    cancel: &CancellationToken,
    future: impl std::future::Future<Output = Result<T, reqwest::Error>>,
) -> Result<T, AppError> {
    tokio::select! {
        _ = cancel.cancelled() => Err(cancelled()),
        result = future => result.map_err(|error| AppError::user("Gemini 网络请求失败。", error.to_string())),
    }
}

fn cancelled() -> AppError {
    AppError::user("字幕任务已取消。", "Subtitle task cancelled")
}

async fn ensure_success(
    status: StatusCode,
    _headers: &HeaderMap,
    body: Option<&str>,
) -> Result<(), AppError> {
    if status.is_success() {
        Ok(())
    } else {
        Err(http_error(status, body.unwrap_or_default()))
    }
}

fn http_error(status: StatusCode, body: &str) -> AppError {
    let message = match status.as_u16() {
        400 => "Gemini 拒绝了请求，请检查模型与参数。",
        401 | 403 => "Gemini Auth Key 无效、受限或无权访问该模型。",
        402 => "Gemini 项目无法计费，请检查账单状态。",
        429 => "Gemini 已达到当前项目的调用或费用限制。",
        500..=599 => "Gemini 服务暂时不可用。",
        _ => "Gemini 请求失败。",
    };
    AppError::user(message, format!("HTTP {status}: {}", truncate(body)))
}

fn retry_after(headers: &HeaderMap) -> Option<u64> {
    headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
}

fn transcript_schema(translate: bool) -> Value {
    let mut properties = serde_json::Map::from_iter([
        ("startMs".to_string(), json!({ "type": "integer" })),
        ("endMs".to_string(), json!({ "type": "integer" })),
        ("sourceText".to_string(), json!({ "type": "string" })),
    ]);
    let mut required = vec!["startMs", "endMs", "sourceText"];
    if translate {
        properties.insert("translatedText".to_string(), json!({ "type": "string" }));
        required.push("translatedText");
    }
    json!({
        "type": "object",
        "properties": {
            "detectedLanguage": { "type": "string" },
            "segments": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                    "additionalProperties": false
                }
            }
        },
        "required": ["detectedLanguage", "segments"],
        "additionalProperties": false
    })
}

fn json_response_format(schema: Value) -> Value {
    json!({
        "type": "text",
        "mime_type": "application/json",
        "schema": schema
    })
}

fn user_input_step(content: Vec<Value>) -> Value {
    json!({
        "type": "user_input",
        "content": content
    })
}

fn extract_output_text(value: &Value) -> Option<String> {
    if let Some(text) = value.get("output_text").and_then(Value::as_str) {
        return Some(text.to_string());
    }
    value
        .get("steps")
        .and_then(Value::as_array)?
        .iter()
        .filter(|step| step.get("type").and_then(Value::as_str) == Some("model_output"))
        .flat_map(|step| {
            step.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .find_map(|content| content.get("text").and_then(Value::as_str))
        .map(ToString::to_string)
}

fn parse_usage(value: &Value, model: &str) -> SubtitleUsage {
    let input_tokens = value
        .pointer("/usage/total_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let output_tokens = value
        .pointer("/usage/total_output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let (input_rate, output_rate) = if model == "gemini-3.5-flash" {
        (1.50, 9.00)
    } else {
        (0.50, 1.50)
    };
    SubtitleUsage {
        input_tokens,
        output_tokens,
        estimated_usd: input_tokens as f64 / 1_000_000.0 * input_rate
            + output_tokens as f64 / 1_000_000.0 * output_rate,
    }
}

fn validate_gemini_result(
    result: &GeminiTranscriptResult,
    duration_ms: u64,
    require_translation: bool,
) -> Result<(), AppError> {
    let mut previous_end = 0;
    for (index, segment) in result.segments.iter().enumerate() {
        if segment.source_text.trim().is_empty()
            || segment.end_ms <= segment.start_ms
            || segment.end_ms > duration_ms.saturating_add(2_000)
            || (index > 0 && segment.start_ms < previous_end)
        {
            return Err(AppError::user(
                "Gemini 返回的字幕时间轴无效。",
                format!("Invalid segment at index {index}"),
            ));
        }
        if require_translation
            && segment
                .translated_text
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(AppError::user(
                "Gemini 返回的字幕缺少译文。",
                format!("Missing translation at index {index}"),
            ));
        }
        previous_end = segment.end_ms;
    }
    Ok(())
}

fn append_text(target: &mut String, addition: &str) {
    let addition = addition.trim();
    if addition.is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push(' ');
    }
    target.push_str(addition);
}

fn merge_segment_text(target: &mut GeminiSegment, source: &GeminiSegment) {
    append_text(&mut target.source_text, &source.source_text);
    if let Some(translation) = source.translated_text.as_deref() {
        let target_translation = target.translated_text.get_or_insert_with(String::new);
        append_text(target_translation, translation);
    }
    target.end_ms = target.end_ms.max(source.end_ms);
}

fn normalize_gemini_result(result: &mut GeminiTranscriptResult, duration_ms: u64) {
    let mut segments = std::mem::take(&mut result.segments);
    segments.sort_by_key(|segment| (segment.start_ms, segment.end_ms));
    let mut normalized: Vec<GeminiSegment> = Vec::with_capacity(segments.len());

    for mut segment in segments {
        segment.source_text = segment.source_text.trim().to_string();
        segment.translated_text = segment.translated_text.map(|text| text.trim().to_string());
        if segment.source_text.is_empty() {
            continue;
        }
        segment.start_ms = segment.start_ms.min(duration_ms);
        segment.end_ms = segment.end_ms.min(duration_ms);

        if segment.end_ms <= segment.start_ms {
            if let Some(previous) = normalized.last_mut() {
                merge_segment_text(previous, &segment);
            }
            continue;
        }

        if let Some(previous) = normalized.last_mut() {
            if segment.start_ms < previous.end_ms {
                if segment.end_ms <= previous.end_ms.saturating_add(250) {
                    merge_segment_text(previous, &segment);
                    continue;
                }
                segment.start_ms = previous.end_ms;
            }
        }

        if segment.end_ms > segment.start_ms {
            normalized.push(segment);
        }
    }
    result.segments = normalized;
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
            .map_err(|error| AppError::user("无法保存 Gemini Auth Key。", error.to_string())),
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
            "尚未配置 Gemini Auth Key。",
            format!("Credential unavailable: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_json_schema_for_interactions_api() {
        let format = json_response_format(json!({"type": "object"}));
        assert_eq!(format["type"], "text");
        assert_eq!(format["mime_type"], "application/json");
        assert_eq!(format["schema"]["type"], "object");
    }

    #[test]
    fn transcript_schema_requires_translation_only_when_requested() {
        let plain = transcript_schema(false);
        let translated = transcript_schema(true);
        assert!(plain
            .pointer("/properties/segments/items/properties/translatedText")
            .is_none());
        assert!(translated
            .pointer("/properties/segments/items/properties/translatedText")
            .is_some());
    }

    #[test]
    fn requests_do_not_force_an_optional_service_tier() {
        let unsupported_field = ["service", "tier"].join("_");
        let source = include_str!("gemini.rs");
        assert!(!source.lines().any(|line| {
            !line.contains("unsupported_field")
                && !line.contains("requests_do_not_force")
                && line.contains(&format!("\"{unsupported_field}\""))
        }));
    }

    #[test]
    fn multimodal_input_is_wrapped_as_one_user_step() {
        let input = user_input_step(vec![
            json!({"type": "text", "text": "Transcribe"}),
            json!({"type": "audio", "uri": "https://example.test/file", "mime_type": "audio/m4a"}),
        ]);
        assert_eq!(input["type"], "user_input");
        assert_eq!(input["content"][0]["type"], "text");
        assert_eq!(input["content"][1]["type"], "audio");
    }

    #[test]
    fn upload_timeout_is_short_but_scales_for_larger_chunks() {
        assert_eq!(upload_timeout(650_000), Duration::from_secs(35));
        assert_eq!(upload_timeout(6 * 1024 * 1024), Duration::from_secs(78));
        assert_eq!(upload_timeout(100 * 1024 * 1024), Duration::from_secs(120));
    }

    #[test]
    fn retries_upload_timeouts_and_server_errors_only() {
        assert!(upload_error_is_retryable(&AppError::user(
            "Gemini 网络请求失败。",
            "operation timed out"
        )));
        assert!(upload_error_is_retryable(&AppError::user(
            "Gemini 服务暂时不可用。",
            "HTTP 503 Service Unavailable"
        )));
        assert!(!upload_error_is_retryable(&AppError::user(
            "Gemini 拒绝了请求，请检查模型与参数。",
            "HTTP 400 Bad Request"
        )));
    }

    #[test]
    fn repairs_out_of_order_overlapping_and_out_of_range_timestamps() {
        let mut result = GeminiTranscriptResult {
            detected_language: "en".to_string(),
            segments: vec![
                GeminiSegment {
                    start_ms: 2_800,
                    end_ms: 5_500,
                    source_text: "third".to_string(),
                    translated_text: Some("第三".to_string()),
                },
                GeminiSegment {
                    start_ms: 0,
                    end_ms: 2_000,
                    source_text: "first".to_string(),
                    translated_text: Some("第一".to_string()),
                },
                GeminiSegment {
                    start_ms: 1_900,
                    end_ms: 3_000,
                    source_text: "second".to_string(),
                    translated_text: Some("第二".to_string()),
                },
            ],
        };
        normalize_gemini_result(&mut result, 5_000);
        assert_eq!(result.segments.len(), 3);
        assert_eq!(result.segments[1].start_ms, 2_000);
        assert_eq!(result.segments[2].start_ms, 3_000);
        assert_eq!(result.segments[2].end_ms, 5_000);
        validate_gemini_result(&result, 5_000, true).unwrap();
    }
}

#[cfg(not(target_os = "windows"))]
pub fn save_api_key(_value: Option<&str>) -> Result<(), AppError> {
    Err(AppError::user(
        "当前平台暂不支持安全保存 Gemini Key。",
        "Windows Credential Manager is required",
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn load_api_key() -> Result<String, AppError> {
    Err(AppError::user(
        "当前平台暂不支持 Gemini Key。",
        "Windows Credential Manager is required",
    ))
}
