use serde::{Deserialize, Serialize};

pub const ECONOMY_MODEL: &str = "gemini-3.1-flash-lite";
pub const QUALITY_MODEL: &str = "gemini-3.5-flash";
pub const PRICING_EFFECTIVE_AT: &str = "2026-07-17";

fn default_processing_mode() -> String {
    "local_free".to_string()
}

fn default_whisper_model() -> String {
    "large-v3-turbo-q5".to_string()
}

fn default_whisper_runtime() -> String {
    "cpu".to_string()
}

fn default_openai_api_base() -> String {
    "https://api.openai.com/v1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleSegment {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_text: String,
    #[serde(default)]
    pub translated_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleArtifact {
    pub kind: String,
    pub path: String,
    pub format: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleStageRecord {
    pub stage: String,
    pub started_at_ms: u64,
    #[serde(default)]
    pub finished_at_ms: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    pub status: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubtitlePerformance {
    #[serde(default)]
    pub stages: Vec<SubtitleStageRecord>,
    #[serde(default)]
    pub encoder: Option<String>,
    #[serde(default)]
    pub output_bytes: Option<u64>,
    #[serde(default)]
    pub uploaded_bytes: u64,
    #[serde(default)]
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleProject {
    pub id: String,
    pub title: String,
    pub source_path: String,
    pub source_fingerprint: String,
    pub duration_ms: u64,
    pub model: String,
    #[serde(default)]
    pub source_language: Option<String>,
    #[serde(default)]
    pub target_language: Option<String>,
    #[serde(default)]
    pub completed_chunks: Vec<String>,
    #[serde(default)]
    pub segments: Vec<SubtitleSegment>,
    #[serde(default)]
    pub usage: SubtitleUsage,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub asr_provider: Option<String>,
    #[serde(default)]
    pub translation_provider: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<SubtitleArtifact>,
    #[serde(default)]
    pub performance: SubtitlePerformance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSettings {
    pub has_api_key: bool,
    pub default_model: String,
    pub default_target_language: String,
    pub max_cost_usd: f64,
    pub max_concurrency: u8,
    #[serde(default = "default_processing_mode")]
    pub processing_mode: String,
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,
    #[serde(default = "default_whisper_runtime")]
    pub whisper_runtime: String,
    #[serde(default)]
    pub has_openai_api_key: bool,
    #[serde(default = "default_openai_api_base")]
    pub openai_api_base: String,
    #[serde(default)]
    pub openai_model: String,
}

impl Default for GeminiSettings {
    fn default() -> Self {
        Self {
            has_api_key: false,
            default_model: ECONOMY_MODEL.to_string(),
            default_target_language: "zh-CN".to_string(),
            max_cost_usd: 2.0,
            max_concurrency: 2,
            processing_mode: default_processing_mode(),
            whisper_model: default_whisper_model(),
            whisper_runtime: default_whisper_runtime(),
            has_openai_api_key: false,
            openai_api_base: default_openai_api_base(),
            openai_model: String::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGeminiSettingsRequest {
    pub api_key: Option<String>,
    pub default_model: String,
    pub default_target_language: String,
    pub max_cost_usd: f64,
    pub max_concurrency: u8,
    pub processing_mode: String,
    pub whisper_model: String,
    pub whisper_runtime: String,
    pub openai_api_key: Option<String>,
    pub openai_api_base: String,
    pub openai_model: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub download_bytes: u64,
    pub download_size: String,
    pub installed: bool,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperRuntimeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub download_bytes: u64,
    pub download_size: String,
    pub installed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperAssetRequest {
    pub id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostEstimate {
    pub model: String,
    pub duration_ms: u64,
    pub input_tokens: u64,
    pub output_tokens_low: u64,
    pub output_tokens_high: u64,
    pub estimated_usd_low: f64,
    pub estimated_usd_high: f64,
    pub pricing_effective_at: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubtitleProjectRequest {
    pub source_path: String,
    pub title: Option<String>,
    pub duration_ms: Option<u64>,
    pub model: Option<String>,
    pub target_language: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTranscriptionRequest {
    pub project_id: String,
    pub translate: bool,
    pub target_language: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTextTaskRequest {
    pub project_id: String,
    pub target_language: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartWhisperTranscriptionRequest {
    pub project_id: String,
    pub model: String,
    pub runtime: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSegmentsRequest {
    pub project_id: String,
    pub segments: Vec<SubtitleSegment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSubtitlesRequest {
    pub project_id: String,
    pub output_path: String,
    pub format: String,
    pub content: String,
}

fn default_subtitle_font_family() -> String {
    "Microsoft YaHei".to_string()
}

fn default_subtitle_font_size() -> u16 {
    48
}

fn default_subtitle_primary_color() -> String {
    "#F8F8F8".to_string()
}

fn default_subtitle_outline_color() -> String {
    "#151515".to_string()
}

fn default_subtitle_background_color() -> String {
    "#151515".to_string()
}

fn default_subtitle_outline_width() -> f32 {
    3.0
}

fn default_subtitle_shadow() -> f32 {
    1.0
}

fn default_subtitle_position() -> String {
    "bottom".to_string()
}

fn default_subtitle_margin_vertical() -> u16 {
    64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleStyle {
    #[serde(default = "default_subtitle_font_family")]
    pub font_family: String,
    #[serde(default = "default_subtitle_font_size")]
    pub font_size: u16,
    #[serde(default = "default_subtitle_font_size")]
    pub translated_font_size: u16,
    #[serde(default = "default_subtitle_primary_color")]
    pub primary_color: String,
    #[serde(default = "default_subtitle_primary_color")]
    pub translated_color: String,
    #[serde(default = "default_subtitle_outline_color")]
    pub outline_color: String,
    #[serde(default = "default_subtitle_background_color")]
    pub background_color: String,
    #[serde(default)]
    pub background_opacity: u8,
    #[serde(default = "default_subtitle_outline_width")]
    pub outline_width: f32,
    #[serde(default = "default_subtitle_shadow")]
    pub shadow: f32,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub boxed: bool,
    #[serde(default = "default_subtitle_position")]
    pub position: String,
    #[serde(default = "default_subtitle_margin_vertical")]
    pub margin_vertical: u16,
    #[serde(default)]
    pub translated_first: bool,
}

impl Default for SubtitleStyle {
    fn default() -> Self {
        Self {
            font_family: default_subtitle_font_family(),
            font_size: default_subtitle_font_size(),
            translated_font_size: default_subtitle_font_size(),
            primary_color: default_subtitle_primary_color(),
            translated_color: default_subtitle_primary_color(),
            outline_color: default_subtitle_outline_color(),
            background_color: default_subtitle_background_color(),
            background_opacity: 0,
            outline_width: default_subtitle_outline_width(),
            shadow: default_subtitle_shadow(),
            bold: false,
            boxed: false,
            position: default_subtitle_position(),
            margin_vertical: default_subtitle_margin_vertical(),
            translated_first: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BurnSubtitlesRequest {
    pub project_id: String,
    pub output_path: String,
    pub content: String,
    #[serde(default)]
    pub style: SubtitleStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrackInfo {
    pub stream_index: usize,
    pub codec: String,
    pub language: String,
    pub title: Option<String>,
    pub is_default: bool,
    pub is_forced: bool,
    pub is_text: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSubtitleAnalysis {
    pub source_path: String,
    pub duration_ms: u64,
    pub detected_language: Option<String>,
    pub tracks: Vec<SubtitleTrackInfo>,
    pub recommended_track: Option<SubtitleTrackInfo>,
    pub strategy: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeMediaRequest {
    pub source_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSubtitleTrackRequest {
    pub project_id: String,
    pub stream_index: usize,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleProgressEvent {
    pub project_id: String,
    pub stage: String,
    pub percent: f32,
    pub chunk_index: Option<usize>,
    pub chunk_total: Option<usize>,
    pub message: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub recoverable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiTranscriptResult {
    pub detected_language: String,
    pub segments: Vec<GeminiSegment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_text: String,
    #[serde(default)]
    pub translated_text: Option<String>,
}

pub fn validate_model(model: &str) -> bool {
    matches!(model, ECONOMY_MODEL | QUALITY_MODEL)
}

pub fn estimate_cost(model: &str, duration_ms: u64, with_translation: bool) -> CostEstimate {
    let seconds = duration_ms as f64 / 1000.0;
    let input_tokens = (seconds * 32.0).ceil() as u64;
    let hours = seconds / 3600.0;
    let (per_million_input, per_million_output) = match model {
        QUALITY_MODEL => (1.50, 9.00),
        _ => (0.50, 1.50),
    };
    let low_per_hour = if with_translation { 30_000.0 } else { 20_000.0 };
    let high_per_hour = if with_translation { 60_000.0 } else { 40_000.0 };
    let output_tokens_low = (hours * low_per_hour).ceil() as u64;
    let output_tokens_high = (hours * high_per_hour).ceil() as u64;
    let input_cost = input_tokens as f64 / 1_000_000.0 * per_million_input;

    CostEstimate {
        model: model.to_string(),
        duration_ms,
        input_tokens,
        output_tokens_low,
        output_tokens_high,
        estimated_usd_low: input_cost + output_tokens_low as f64 / 1_000_000.0 * per_million_output,
        estimated_usd_high: input_cost
            + output_tokens_high as f64 / 1_000_000.0 * per_million_output,
        pricing_effective_at: PRICING_EFFECTIVE_AT,
    }
}

pub fn validate_segments(
    segments: &[SubtitleSegment],
    duration_ms: u64,
    require_translation: bool,
) -> Result<(), String> {
    let mut previous_end = 0;
    for (index, segment) in segments.iter().enumerate() {
        if segment.source_text.trim().is_empty() {
            return Err(format!("第 {} 条字幕原文为空。", index + 1));
        }
        if segment.end_ms <= segment.start_ms {
            return Err(format!("第 {} 条字幕时间范围无效。", index + 1));
        }
        if segment.end_ms > duration_ms.saturating_add(2_000) {
            return Err(format!("第 {} 条字幕超出媒体时长。", index + 1));
        }
        if index > 0 && segment.start_ms < previous_end {
            return Err(format!("第 {} 条字幕与上一条重叠。", index + 1));
        }
        if require_translation
            && segment
                .translated_text
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!("第 {} 条字幕缺少译文。", index + 1));
        }
        previous_end = segment.end_ms;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_flash_lite_audio_cost() {
        let estimate = estimate_cost(ECONOMY_MODEL, 3_600_000, true);
        assert_eq!(estimate.input_tokens, 115_200);
        assert!((estimate.estimated_usd_low - 0.1026).abs() < 0.0001);
        assert!((estimate.estimated_usd_high - 0.1476).abs() < 0.0001);
    }

    #[test]
    fn rejects_overlapping_segments() {
        let segments = vec![
            SubtitleSegment {
                id: "a".into(),
                start_ms: 0,
                end_ms: 2_000,
                source_text: "one".into(),
                translated_text: None,
            },
            SubtitleSegment {
                id: "b".into(),
                start_ms: 1_900,
                end_ms: 3_000,
                source_text: "two".into(),
                translated_text: None,
            },
        ];
        assert!(validate_segments(&segments, 3_000, false).is_err());
    }

    #[test]
    fn old_projects_receive_default_performance_data() {
        let value = serde_json::json!({
            "id": "legacy",
            "title": "Legacy",
            "sourcePath": "demo.mp4",
            "sourceFingerprint": "1",
            "durationMs": 1000,
            "model": ECONOMY_MODEL,
            "completedChunks": [],
            "segments": [],
            "usage": {
                "inputTokens": 0,
                "outputTokens": 0,
                "estimatedUsd": 0.0
            },
            "status": "ready",
            "createdAt": 1,
            "updatedAt": 1
        });
        let project: SubtitleProject = serde_json::from_value(value).unwrap();
        assert!(project.performance.stages.is_empty());
        assert_eq!(project.performance.uploaded_bytes, 0);
    }

    #[test]
    fn old_burn_requests_receive_default_subtitle_style() {
        let value = serde_json::json!({
            "projectId": "legacy",
            "outputPath": "demo.mp4",
            "content": "translated"
        });
        let request: BurnSubtitlesRequest = serde_json::from_value(value).unwrap();
        assert_eq!(request.style.font_family, "Microsoft YaHei");
        assert_eq!(request.style.font_size, 48);
        assert_eq!(request.style.translated_font_size, 48);
    }
}
