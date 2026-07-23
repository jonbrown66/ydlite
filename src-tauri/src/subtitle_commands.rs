use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::future::join_all;
use regex::Regex;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::bing::BingTranslator;
use crate::errors::AppError;
use crate::gemini::{load_api_key, save_api_key, GeminiClient};
use crate::openai_compatible::{
    load_api_key as load_openai_key, save_api_key as save_openai_key, OpenAiCompatibleClient,
};
use crate::process_utils::hidden_command;
use crate::subtitle_export;
use crate::subtitle_store;
use crate::subtitle_types::*;
use crate::tool_paths;
use crate::{whisper, whisper_manager};

const CHUNK_MS: u64 = 15 * 60 * 1_000;
const OVERLAP_MS: u64 = 2_000;

#[derive(Default)]
pub struct SubtitleTaskState {
    tasks: Mutex<HashMap<String, CancellationToken>>,
}

impl SubtitleTaskState {
    pub fn has_active_tasks(&self) -> bool {
        !self.tasks.lock().unwrap().is_empty()
    }

    pub fn is_active(&self, project_id: &str) -> bool {
        self.tasks.lock().unwrap().contains_key(project_id)
    }
}

fn emit(app: &AppHandle, event: SubtitleProgressEvent) {
    let _ = app.emit("subtitle://progress", event);
}

fn add_usage(total: &mut SubtitleUsage, usage: &SubtitleUsage) {
    total.input_tokens += usage.input_tokens;
    total.output_tokens += usage.output_tokens;
    total.estimated_usd += usage.estimated_usd;
}

fn is_supported_project_model(model: &str) -> bool {
    validate_model(model) || whisper_manager::is_supported_model_id(model)
}

fn begin_stage(project: &mut SubtitleProject, stage: &str) {
    let now = subtitle_store::now_epoch_ms();
    if let Some(active) = project
        .performance
        .stages
        .iter_mut()
        .rev()
        .find(|record| record.status == "running")
    {
        active.finished_at_ms = Some(now);
        active.duration_ms = Some(now.saturating_sub(active.started_at_ms));
        active.status = "completed".into();
    }
    project.performance.stages.push(SubtitleStageRecord {
        stage: stage.into(),
        started_at_ms: now,
        finished_at_ms: None,
        duration_ms: None,
        status: "running".into(),
        detail: None,
    });
}

fn finish_stage(project: &mut SubtitleProject, status: &str, detail: Option<String>) {
    let now = subtitle_store::now_epoch_ms();
    if let Some(active) = project
        .performance
        .stages
        .iter_mut()
        .rev()
        .find(|record| record.status == "running")
    {
        active.finished_at_ms = Some(now);
        active.duration_ms = Some(now.saturating_sub(active.started_at_ms));
        active.status = status.into();
        active.detail = detail;
    }
}

#[tauri::command]
pub fn get_gemini_settings(app: AppHandle) -> Result<GeminiSettings, AppError> {
    let mut settings = subtitle_store::read_settings(&app)?;
    settings.has_api_key = load_api_key().is_ok();
    settings.has_openai_api_key = load_openai_key().is_ok();
    Ok(settings)
}

#[tauri::command]
pub fn save_gemini_settings(
    app: AppHandle,
    request: SaveGeminiSettingsRequest,
) -> Result<GeminiSettings, AppError> {
    if !validate_model(&request.default_model) {
        return Err(AppError::user(
            "不支持该 Gemini 模型。",
            request.default_model,
        ));
    }
    if !(0.0..=1000.0).contains(&request.max_cost_usd) {
        return Err(AppError::user(
            "单任务费用上限无效。",
            "Expected 0-1000 USD",
        ));
    }
    if !matches!(request.max_concurrency, 1 | 2) {
        return Err(AppError::user(
            "并发数只能为 1 或 2。",
            "Invalid concurrency",
        ));
    }
    if !matches!(
        request.processing_mode.as_str(),
        "local_free" | "local_custom" | "gemini"
    ) {
        return Err(AppError::user(
            "不支持该字幕处理模式。",
            request.processing_mode,
        ));
    }
    if !whisper_manager::is_supported_model_id(&request.whisper_model) {
        return Err(AppError::user(
            "不支持该 Whisper 模型。",
            request.whisper_model,
        ));
    }
    if !matches!(request.whisper_runtime.as_str(), "cpu" | "cuda") {
        return Err(AppError::user(
            "不支持该 Whisper 运行组件。",
            request.whisper_runtime,
        ));
    }
    if let Some(value) = request.api_key.as_deref() {
        save_api_key(Some(value))?;
    }
    if let Some(value) = request.openai_api_key.as_deref() {
        save_openai_key(Some(value))?;
    }
    crate::openai_compatible::validate_base_url(&request.openai_api_base)?;
    let settings = GeminiSettings {
        has_api_key: load_api_key().is_ok(),
        default_model: request.default_model,
        default_target_language: request.default_target_language.trim().to_string(),
        max_cost_usd: request.max_cost_usd,
        max_concurrency: request.max_concurrency,
        processing_mode: request.processing_mode,
        whisper_model: request.whisper_model,
        whisper_runtime: request.whisper_runtime,
        has_openai_api_key: load_openai_key().is_ok(),
        openai_api_base: request.openai_api_base.trim_end_matches('/').to_string(),
        openai_model: request.openai_model.trim().to_string(),
    };
    subtitle_store::write_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
pub async fn test_gemini_connection() -> Result<String, AppError> {
    GeminiClient::new(load_api_key()?)?.test_connection().await
}

#[tauri::command]
pub async fn test_openai_compatible_connection(app: AppHandle) -> Result<String, AppError> {
    let settings = subtitle_store::read_settings(&app)?;
    OpenAiCompatibleClient::new(
        &settings.openai_api_base,
        load_openai_key()?,
        &settings.openai_model,
    )?
    .test_connection()
    .await
}

#[tauri::command]
pub fn estimate_transcription_cost(
    model: String,
    duration_ms: u64,
    with_translation: bool,
) -> Result<CostEstimate, AppError> {
    if !is_supported_project_model(&model) {
        return Err(AppError::user("不支持该字幕模型。", model));
    }
    Ok(estimate_cost(&model, duration_ms, with_translation))
}

async fn probe_duration(path: &Path) -> Result<u64, AppError> {
    let output = hidden_command(tool_paths::ffmpeg())
        .arg("-i")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let regex = Regex::new(r"Duration:\s*(\d+):(\d+):(\d+(?:\.\d+)?)").unwrap();
    let captures = regex.captures(&stderr).ok_or_else(|| {
        AppError::user(
            "无法读取视频时长。",
            "FFmpeg output does not contain Duration",
        )
    })?;
    let hours: f64 = captures[1].parse().unwrap_or_default();
    let minutes: f64 = captures[2].parse().unwrap_or_default();
    let seconds: f64 = captures[3].parse().unwrap_or_default();
    Ok(((hours * 3600.0 + minutes * 60.0 + seconds) * 1000.0).round() as u64)
}

fn is_text_subtitle(codec: &str) -> bool {
    matches!(
        codec,
        "subrip" | "srt" | "ass" | "ssa" | "webvtt" | "mov_text" | "text" | "ttml"
    )
}

fn normalized_language(value: Option<&str>) -> String {
    value.unwrap_or("und").trim().to_ascii_lowercase()
}

fn is_chinese_language(value: &str) -> bool {
    matches!(
        value,
        "zh" | "zho" | "chi" | "cmn" | "zh-cn" | "zh-hans" | "chs"
    )
}

#[tauri::command]
pub async fn analyze_subtitle_source(
    request: AnalyzeMediaRequest,
) -> Result<MediaSubtitleAnalysis, AppError> {
    let source = PathBuf::from(&request.source_path);
    if !source.is_file() {
        return Err(AppError::user("找不到视频文件。", request.source_path));
    }
    let output = hidden_command(tool_paths::ffprobe())
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(&source)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if !output.status.success() {
        return Err(AppError::user(
            "无法分析视频中的字幕。",
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let duration_ms = value
        .pointer("/format/duration")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|seconds| (seconds * 1000.0).round() as u64)
        .unwrap_or_else(|| {
            value["streams"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|stream| stream["duration"].as_str()?.parse::<f64>().ok())
                .map(|seconds| (seconds * 1000.0).round() as u64)
                .max()
                .unwrap_or_default()
        });
    let tracks: Vec<SubtitleTrackInfo> = value["streams"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|stream| stream["codec_type"].as_str() == Some("subtitle"))
        .map(|stream| {
            let codec = stream["codec_name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            SubtitleTrackInfo {
                stream_index: stream["index"].as_u64().unwrap_or_default() as usize,
                codec: codec.clone(),
                language: normalized_language(
                    stream.pointer("/tags/language").and_then(Value::as_str),
                ),
                title: stream
                    .pointer("/tags/title")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                is_default: stream
                    .pointer("/disposition/default")
                    .and_then(Value::as_u64)
                    == Some(1),
                is_forced: stream
                    .pointer("/disposition/forced")
                    .and_then(Value::as_u64)
                    == Some(1),
                is_text: is_text_subtitle(&codec),
            }
        })
        .collect();
    let recommended_track = tracks
        .iter()
        .filter(|track| track.is_text)
        .max_by_key(|track| {
            (
                !track.is_forced,
                is_chinese_language(&track.language),
                track.is_default,
            )
        })
        .cloned();
    let (strategy, message, detected_language) = match &recommended_track {
        Some(track) if is_chinese_language(&track.language) => (
            "extract_chinese".to_string(),
            "检测到中文字幕，将直接提取，不调用 Gemini。".to_string(),
            Some(track.language.clone()),
        ),
        Some(track) => (
            "translate_subtitle".to_string(),
            format!(
                "检测到 {} 字幕，将保留原时间轴并翻译成中文。",
                if track.language == "und" {
                    "未标记语言的"
                } else {
                    &track.language
                }
            ),
            (track.language != "und").then(|| track.language.clone()),
        ),
        None => (
            "transcribe_audio".to_string(),
            if tracks.is_empty() {
                "未检测到内嵌字幕，将自动识别语音并生成中文字幕。".to_string()
            } else {
                "仅检测到图片字幕，将从音频重新识别并生成中文字幕。".to_string()
            },
            None,
        ),
    };
    Ok(MediaSubtitleAnalysis {
        source_path: source.to_string_lossy().to_string(),
        duration_ms,
        detected_language,
        tracks,
        recommended_track,
        strategy,
        message,
    })
}

fn parse_srt_timestamp(value: &str) -> Option<u64> {
    let normalized = value.trim().replace('.', ",");
    let parts: Vec<&str> = normalized.split([':', ',']).collect();
    if parts.len() != 4 {
        return None;
    }
    let hours = parts[0].parse::<u64>().ok()?;
    let minutes = parts[1].parse::<u64>().ok()?;
    let seconds = parts[2].parse::<u64>().ok()?;
    let millis = parts[3].parse::<u64>().ok()?;
    Some(hours * 3_600_000 + minutes * 60_000 + seconds * 1_000 + millis)
}

fn parse_srt(value: &str) -> Result<Vec<SubtitleSegment>, AppError> {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let timing = Regex::new(
        r"(?m)^\s*(\d{1,2}:\d{2}:\d{2}[,.]\d{3})\s*-->\s*(\d{1,2}:\d{2}:\d{2}[,.]\d{3})",
    )
    .unwrap();
    let matches: Vec<_> = timing.captures_iter(&normalized).collect();
    let mut segments = Vec::new();
    for (index, captures) in matches.iter().enumerate() {
        let whole = captures.get(0).unwrap();
        let text_start = whole.end();
        let text_end = matches
            .get(index + 1)
            .and_then(|next| next.get(0))
            .map(|next| {
                let prefix = &normalized[..next.start()];
                prefix.rfind("\n\n").unwrap_or(next.start())
            })
            .unwrap_or(normalized.len());
        let source_text = normalized[text_start..text_end]
            .trim()
            .lines()
            .filter(|line| {
                !line.trim().is_empty() && !line.trim().chars().all(|c| c.is_ascii_digit())
            })
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n");
        if source_text.is_empty() {
            continue;
        }
        segments.push(SubtitleSegment {
            id: Uuid::new_v4().to_string(),
            start_ms: parse_srt_timestamp(&captures[1]).unwrap_or_default(),
            end_ms: parse_srt_timestamp(&captures[2]).unwrap_or_default(),
            source_text,
            translated_text: None,
        });
    }
    if segments.is_empty() {
        return Err(AppError::user(
            "字幕轨道没有可读取的文本。",
            "No SRT segments parsed",
        ));
    }
    Ok(segments)
}

#[tauri::command]
pub async fn import_subtitle_track(
    app: AppHandle,
    request: ImportSubtitleTrackRequest,
) -> Result<SubtitleProject, AppError> {
    let mut project = subtitle_store::read_project(&app, &request.project_id)?;
    let temp = subtitle_store::temp_dir(&app, &project.id)?;
    std::fs::create_dir_all(&temp)?;
    let path = temp.join("source.srt");
    let output = hidden_command(tool_paths::ffmpeg())
        .args(["-y", "-i", &project.source_path, "-map"])
        .arg(format!("0:{}", request.stream_index))
        .args(["-c:s", "srt"])
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if !output.status.success() {
        subtitle_store::clean_temp(&app, &project.id);
        return Err(AppError::user(
            "字幕轨道提取失败。",
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    let text = std::fs::read_to_string(&path)?;
    project.segments = parse_srt(&text)?;
    project.source_language = request.language;
    project.completed_chunks = vec!["embedded-subtitle".to_string()];
    project.status = "ready".to_string();
    project.updated_at = subtitle_store::now_epoch();
    subtitle_store::write_project(&app, &project)?;
    subtitle_store::clean_temp(&app, &project.id);
    Ok(project)
}

#[tauri::command]
pub async fn create_subtitle_project(
    app: AppHandle,
    request: CreateSubtitleProjectRequest,
) -> Result<SubtitleProject, AppError> {
    let source = PathBuf::from(&request.source_path);
    if !source.is_file() {
        return Err(AppError::user(
            "找不到导入的视频文件。",
            request.source_path,
        ));
    }
    let settings = subtitle_store::read_settings(&app)?;
    let model = request.model.unwrap_or(settings.default_model);
    if !is_supported_project_model(&model) {
        return Err(AppError::user("不支持该字幕模型。", model));
    }
    let duration_ms = match request.duration_ms {
        Some(value) if value > 0 => value,
        _ => probe_duration(&source).await?,
    };
    let now = subtitle_store::now_epoch();
    let project = SubtitleProject {
        id: Uuid::new_v4().to_string(),
        title: request
            .title
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| {
                source
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .unwrap_or("字幕项目")
                    .to_string()
            }),
        source_path: source.to_string_lossy().to_string(),
        source_fingerprint: subtitle_store::fingerprint(&source)?,
        duration_ms,
        model,
        source_language: None,
        target_language: request.target_language,
        completed_chunks: Vec::new(),
        segments: Vec::new(),
        usage: SubtitleUsage::default(),
        status: "draft".into(),
        created_at: now,
        updated_at: now,
        last_error: None,
        asr_provider: None,
        translation_provider: None,
        artifacts: Vec::new(),
        performance: SubtitlePerformance::default(),
    };
    subtitle_store::write_project(&app, &project)?;
    Ok(project)
}

#[tauri::command]
pub fn list_subtitle_projects(app: AppHandle) -> Result<Vec<SubtitleProject>, AppError> {
    let mut projects = subtitle_store::list_projects(&app)?;
    for project in &mut projects {
        project
            .artifacts
            .retain(|artifact| Path::new(&artifact.path).is_file());
    }
    Ok(projects)
}

#[tauri::command]
pub fn delete_subtitle_project(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    project_id: String,
) -> Result<bool, AppError> {
    if state.is_active(&project_id) {
        return Err(AppError::user(
            "正在处理的字幕项目不能删除。",
            "Cancel or finish the active task first",
        ));
    }
    subtitle_store::delete_project(&app, &project_id)
}

#[tauri::command]
pub fn clear_subtitle_projects(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
) -> Result<usize, AppError> {
    if state.has_active_tasks() {
        return Err(AppError::user(
            "字幕任务运行期间不能清空项目记录。",
            "Cancel or finish the active task first",
        ));
    }
    subtitle_store::clear_projects(&app)
}

fn chunks(duration_ms: u64) -> Vec<(u64, u64)> {
    let mut result = Vec::new();
    let mut start = 0;
    while start < duration_ms {
        let end = (start + CHUNK_MS).min(duration_ms);
        result.push((start, end));
        if end == duration_ms {
            break;
        }
        start = end.saturating_sub(OVERLAP_MS);
    }
    result
}

async fn extract_audio(source: &str, output: &Path, start: u64, end: u64) -> Result<(), AppError> {
    let status = hidden_command(tool_paths::ffmpeg())
        .args([
            "-y",
            "-ss",
            &format!("{:.3}", start as f64 / 1000.0),
            "-i",
            source,
        ])
        .args(["-t", &format!("{:.3}", (end - start) as f64 / 1000.0)])
        .args([
            "-vn", "-ac", "1", "-ar", "16000", "-c:a", "aac", "-b:a", "48k",
        ])
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if !status.status.success() {
        return Err(AppError::user(
            "音频提取失败。",
            String::from_utf8_lossy(&status.stderr),
        ));
    }
    Ok(())
}

fn normalized(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn merge_chunk(project: &mut SubtitleProject, result: GeminiTranscriptResult, offset: u64) {
    for segment in result.segments {
        let start_ms = offset + segment.start_ms;
        let end_ms = (offset + segment.end_ms).min(project.duration_ms);
        let duplicate = project.segments.iter().rev().take(4).any(|existing| {
            normalized(&existing.source_text) == normalized(&segment.source_text)
                && start_ms <= existing.end_ms.saturating_add(OVERLAP_MS)
        });
        if duplicate || end_ms <= start_ms {
            continue;
        }
        let clipped_start = project
            .segments
            .last()
            .map(|s| s.end_ms.max(start_ms))
            .unwrap_or(start_ms);
        if end_ms <= clipped_start {
            continue;
        }
        project.segments.push(SubtitleSegment {
            id: Uuid::new_v4().to_string(),
            start_ms: clipped_start,
            end_ms,
            source_text: segment.source_text.trim().to_string(),
            translated_text: segment.translated_text.map(|v| v.trim().to_string()),
        });
    }
}

fn register_task(
    state: &SubtitleTaskState,
    project_id: &str,
) -> Result<CancellationToken, AppError> {
    let mut tasks = state.tasks.lock().unwrap();
    if tasks.contains_key(project_id) {
        return Err(AppError::user(
            "该项目已有任务正在运行。",
            "Task already active",
        ));
    }
    let token = CancellationToken::new();
    tasks.insert(project_id.to_string(), token.clone());
    Ok(token)
}

fn finish_task(state: &SubtitleTaskState, project_id: &str) {
    state.tasks.lock().unwrap().remove(project_id);
}

async fn transcribe_inner(
    app: &AppHandle,
    request: &StartTranscriptionRequest,
    cancel: &CancellationToken,
) -> Result<SubtitleProject, AppError> {
    let mut project = subtitle_store::read_project(app, &request.project_id)?;
    let settings = subtitle_store::read_settings(app)?;
    let model = request
        .model
        .clone()
        .unwrap_or_else(|| project.model.clone());
    if !validate_model(&model) {
        return Err(AppError::user("不支持该模型。", model));
    }
    let estimate = estimate_cost(&model, project.duration_ms, request.translate);
    if settings.max_cost_usd > 0.0 && estimate.estimated_usd_high > settings.max_cost_usd {
        return Err(AppError::user(
            "预计费用超过单任务上限，任务未启动。",
            format!(
                "High estimate ${:.4}, limit ${:.4}",
                estimate.estimated_usd_high, settings.max_cost_usd
            ),
        ));
    }
    let client = GeminiClient::new(load_api_key()?)?;
    let specs = chunks(project.duration_ms);
    let completed: HashSet<String> = project.completed_chunks.iter().cloned().collect();
    project.model = model.clone();
    project.target_language = if request.translate {
        request.target_language.clone()
    } else {
        project.target_language.clone()
    };
    project.status = "processing".into();
    project.last_error = None;
    begin_stage(&mut project, "gemini_transcription");
    subtitle_store::write_project(app, &project)?;
    let temp = subtitle_store::temp_dir(app, &project.id)?;
    std::fs::create_dir_all(&temp)?;
    let pending: Vec<(usize, u64, u64)> = specs
        .iter()
        .copied()
        .enumerate()
        .filter(|(index, _)| !completed.contains(&index.to_string()))
        .map(|(index, (start, end))| (index, start, end))
        .collect();
    let mut concurrency = settings.max_concurrency.clamp(1, 2) as usize;
    let mut cursor = 0;
    while cursor < pending.len() {
        if cancel.is_cancelled() {
            return Err(AppError::user("字幕任务已取消。", "cancelled"));
        }
        let batch_end = (cursor + concurrency).min(pending.len());
        let batch = &pending[cursor..batch_end];
        let mut outcomes = join_all(batch.iter().copied().map(|(index, start, end)| {
            process_gemini_chunk(
                app.clone(),
                client.clone(),
                cancel.clone(),
                project.id.clone(),
                project.source_path.clone(),
                temp.clone(),
                model.clone(),
                request.target_language.clone(),
                request.translate,
                specs.len(),
                index,
                start,
                end,
            )
        }))
        .await;
        outcomes.sort_by_key(|(index, _)| *index);

        if concurrency > 1
            && outcomes
                .iter()
                .any(|(_, result)| result.as_ref().err().is_some_and(AppError::is_rate_limited))
        {
            emit(
                app,
                SubtitleProgressEvent {
                    project_id: project.id.clone(),
                    stage: "transcribing".into(),
                    percent: cursor as f32 / pending.len().max(1) as f32 * 100.0,
                    chunk_index: None,
                    chunk_total: Some(specs.len()),
                    message: "Gemini 触发限流，已自动切换为单并发重试。".into(),
                    input_tokens: project.usage.input_tokens,
                    output_tokens: project.usage.output_tokens,
                    recoverable: true,
                },
            );
            concurrency = 1;
            for (index, result) in &mut outcomes {
                if result.as_ref().err().is_some_and(AppError::is_rate_limited) {
                    let (_, start, end) = pending
                        .iter()
                        .copied()
                        .find(|(candidate, _, _)| candidate == index)
                        .expect("chunk index must exist");
                    *result = process_gemini_chunk(
                        app.clone(),
                        client.clone(),
                        cancel.clone(),
                        project.id.clone(),
                        project.source_path.clone(),
                        temp.clone(),
                        model.clone(),
                        request.target_language.clone(),
                        request.translate,
                        specs.len(),
                        *index,
                        start,
                        end,
                    )
                    .await
                    .1;
                }
            }
        }

        for (index, result) in outcomes {
            let (start, result, usage, uploaded_bytes, retry_count) = result?;
            if project.source_language.is_none() {
                project.source_language = Some(result.detected_language.clone());
            }
            merge_chunk(&mut project, result, start);
            add_usage(&mut project.usage, &usage);
            project.performance.uploaded_bytes = project
                .performance
                .uploaded_bytes
                .saturating_add(uploaded_bytes);
            project.performance.retry_count =
                project.performance.retry_count.saturating_add(retry_count);
            project.completed_chunks.push(index.to_string());
            project.updated_at = subtitle_store::now_epoch();
            subtitle_store::write_project(app, &project)?;
            emit(
                app,
                SubtitleProgressEvent {
                    project_id: project.id.clone(),
                    stage: "transcribing".into(),
                    percent: project.completed_chunks.len() as f32 / specs.len() as f32 * 100.0,
                    chunk_index: Some(index + 1),
                    chunk_total: Some(specs.len()),
                    message: "音频块已完成并保存。".into(),
                    input_tokens: project.usage.input_tokens,
                    output_tokens: project.usage.output_tokens,
                    recoverable: false,
                },
            );
        }
        cursor = batch_end;
    }
    project.status = "ready".into();
    finish_stage(&mut project, "completed", None);
    project.updated_at = subtitle_store::now_epoch();
    subtitle_store::write_project(app, &project)?;
    subtitle_store::clean_temp(app, &project.id);
    Ok(project)
}

#[allow(clippy::too_many_arguments)]
async fn process_gemini_chunk(
    app: AppHandle,
    client: GeminiClient,
    cancel: CancellationToken,
    project_id: String,
    source_path: String,
    temp: PathBuf,
    model: String,
    target_language: Option<String>,
    translate: bool,
    chunk_total: usize,
    index: usize,
    start: u64,
    end: u64,
) -> (
    usize,
    Result<(u64, GeminiTranscriptResult, SubtitleUsage, u64, u32), AppError>,
) {
    let result = async {
        if cancel.is_cancelled() {
            return Err(AppError::user("字幕任务已取消。", "cancelled"));
        }
        emit(
            &app,
            SubtitleProgressEvent {
                project_id: project_id.clone(),
                stage: "extracting".into(),
                percent: index as f32 / chunk_total.max(1) as f32 * 100.0,
                chunk_index: Some(index + 1),
                chunk_total: Some(chunk_total),
                message: "正在提取音频…".into(),
                input_tokens: 0,
                output_tokens: 0,
                recoverable: false,
            },
        );
        let local = temp.join(format!("chunk-{index:04}.m4a"));
        if let Err(error) = extract_audio(&source_path, &local, start, end).await {
            let _ = std::fs::remove_file(&local);
            return Err(error);
        }
        emit(
            &app,
            SubtitleProgressEvent {
                project_id: project_id.clone(),
                stage: "uploading".into(),
                percent: index as f32 / chunk_total.max(1) as f32 * 100.0,
                chunk_index: Some(index + 1),
                chunk_total: Some(chunk_total),
                message: "正在上传音频块…".into(),
                input_tokens: 0,
                output_tokens: 0,
                recoverable: false,
            },
        );
        let retry_app = app.clone();
        let retry_project_id = project_id.clone();
        let upload_app = app.clone();
        let upload_project_id = project_id.clone();
        let upload_progress = Arc::new(move |sent: u64, total: u64| {
            let chunk_progress = if total == 0 {
                0.0
            } else {
                sent as f32 / total as f32
            };
            emit(
                &upload_app,
                SubtitleProgressEvent {
                    project_id: upload_project_id.clone(),
                    stage: "uploading".into(),
                    percent: ((index as f32 + chunk_progress) / chunk_total.max(1) as f32 * 100.0)
                        .clamp(0.0, 99.0),
                    chunk_index: Some(index + 1),
                    chunk_total: Some(chunk_total),
                    message: format!(
                        "安全上传 {:.1}/{:.1} MB",
                        sent as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    ),
                    input_tokens: 0,
                    output_tokens: 0,
                    recoverable: false,
                },
            );
        });
        let remote = match client
            .upload_audio(&local, &cancel, upload_progress, |attempt| {
                emit(
                    &retry_app,
                    SubtitleProgressEvent {
                        project_id: retry_project_id.clone(),
                        stage: "uploading".into(),
                        percent: index as f32 / chunk_total.max(1) as f32 * 100.0,
                        chunk_index: Some(index + 1),
                        chunk_total: Some(chunk_total),
                        message: format!("音频上传超时，正在进行第 {attempt} 次尝试…"),
                        input_tokens: 0,
                        output_tokens: 0,
                        recoverable: true,
                    },
                );
            })
            .await
        {
            Ok(remote) => remote,
            Err(error) => {
                let _ = std::fs::remove_file(&local);
                return Err(error);
            }
        };
        emit(
            &app,
            SubtitleProgressEvent {
                project_id: project_id.clone(),
                stage: "transcribing".into(),
                percent: index as f32 / chunk_total.max(1) as f32 * 100.0,
                chunk_index: Some(index + 1),
                chunk_total: Some(chunk_total),
                message: "音频上传完成，Gemini 正在识别并翻译…".into(),
                input_tokens: 0,
                output_tokens: 0,
                recoverable: false,
            },
        );
        let retry_count = Arc::new(AtomicU32::new(0));
        let wait_app = app.clone();
        let wait_project_id = project_id.clone();
        let retry_count_callback = retry_count.clone();
        let retry_wait = Arc::new(move |attempt: usize, wait: u64| {
            retry_count_callback.fetch_add(1, Ordering::Relaxed);
            emit(
                &wait_app,
                SubtitleProgressEvent {
                    project_id: wait_project_id.clone(),
                    stage: "rate_limited".into(),
                    percent: index as f32 / chunk_total.max(1) as f32 * 100.0,
                    chunk_index: Some(index + 1),
                    chunk_total: Some(chunk_total),
                    message: format!("Gemini 限流，{wait} 秒后进行第 {attempt} 次尝试"),
                    input_tokens: 0,
                    output_tokens: 0,
                    recoverable: true,
                },
            );
        });
        let generated = client
            .transcribe(
                &remote,
                &model,
                end - start,
                translate,
                target_language.as_deref(),
                &cancel,
                Some(retry_wait),
            )
            .await;
        client.delete_file(&remote.name).await;
        let uploaded_bytes = std::fs::metadata(&local)
            .map(|metadata| metadata.len())
            .unwrap_or_default();
        let _ = std::fs::remove_file(&local);
        generated.map(|(result, usage)| {
            (
                start,
                result,
                usage,
                uploaded_bytes,
                retry_count.load(Ordering::Relaxed),
            )
        })
    }
    .await;
    (index, result)
}

#[tauri::command]
pub async fn start_gemini_transcription(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    request: StartTranscriptionRequest,
) -> Result<SubtitleProject, AppError> {
    let cancel = register_task(&state, &request.project_id)?;
    let result = transcribe_inner(&app, &request, &cancel).await;
    finish_task(&state, &request.project_id);
    if let Err(error) = &result {
        if let Ok(mut project) = subtitle_store::read_project(&app, &request.project_id) {
            project.status = if cancel.is_cancelled() {
                "paused"
            } else {
                "failed"
            }
            .into();
            project.last_error = Some(error.to_string());
            finish_stage(
                &mut project,
                if cancel.is_cancelled() {
                    "cancelled"
                } else {
                    "failed"
                },
                Some(error.to_string()),
            );
            project.updated_at = subtitle_store::now_epoch();
            let _ = subtitle_store::write_project(&app, &project);
        }
        subtitle_store::clean_temp(&app, &request.project_id);
    }
    result
}

#[tauri::command]
pub async fn retry_subtitle_chunk(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    request: StartTranscriptionRequest,
) -> Result<SubtitleProject, AppError> {
    start_gemini_transcription(app, state, request).await
}

async fn whisper_transcribe_inner(
    app: &AppHandle,
    request: &StartWhisperTranscriptionRequest,
    cancel: &CancellationToken,
) -> Result<SubtitleProject, AppError> {
    let mut project = subtitle_store::read_project(app, &request.project_id)?;
    whisper_manager::ensure_model(app, &request.model)?;
    whisper_manager::executable_path(app, &request.runtime)?;
    project.status = "processing".into();
    project.last_error = None;
    project.model = request.model.clone();
    project.asr_provider = Some("whisper".into());
    project.translation_provider = None;
    begin_stage(&mut project, "whisper_transcription");
    subtitle_store::write_project(app, &project)?;
    let temp = subtitle_store::temp_dir(app, &project.id)?;
    std::fs::create_dir_all(&temp)?;
    let wav = temp.join("whisper-input.wav");
    emit(
        app,
        SubtitleProgressEvent {
            project_id: project.id.clone(),
            stage: "extracting".into(),
            percent: 5.0,
            chunk_index: None,
            chunk_total: None,
            message: "正在提取本地识别音频…".into(),
            input_tokens: 0,
            output_tokens: 0,
            recoverable: false,
        },
    );
    whisper::extract_wav(Path::new(&project.source_path), &wav).await?;
    if cancel.is_cancelled() {
        return Err(AppError::user("字幕任务已取消。", "cancelled"));
    }
    emit(
        app,
        SubtitleProgressEvent {
            project_id: project.id.clone(),
            stage: "transcribing".into(),
            percent: 20.0,
            chunk_index: None,
            chunk_total: None,
            message: format!("正在使用 {} 本地识别…", request.model),
            input_tokens: 0,
            output_tokens: 0,
            recoverable: false,
        },
    );
    let output_prefix = temp.join("whisper-result");
    let result = whisper::transcribe(
        app,
        &wav,
        &output_prefix,
        &request.model,
        &request.runtime,
        project.duration_ms,
        cancel,
    )
    .await?;
    project.source_language = Some(result.language);
    project.segments = result.segments;
    project.completed_chunks = vec!["local-whisper".into()];
    project.status = "ready".into();
    finish_stage(&mut project, "completed", None);
    project.updated_at = subtitle_store::now_epoch();
    subtitle_store::write_project(app, &project)?;
    emit(
        app,
        SubtitleProgressEvent {
            project_id: project.id.clone(),
            stage: "transcribing".into(),
            percent: 100.0,
            chunk_index: None,
            chunk_total: None,
            message: "本地语音识别完成。".into(),
            input_tokens: 0,
            output_tokens: 0,
            recoverable: false,
        },
    );
    subtitle_store::clean_temp(app, &project.id);
    Ok(project)
}

#[tauri::command]
pub async fn start_whisper_transcription(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    request: StartWhisperTranscriptionRequest,
) -> Result<SubtitleProject, AppError> {
    let cancel = register_task(&state, &request.project_id)?;
    let result = whisper_transcribe_inner(&app, &request, &cancel).await;
    finish_task(&state, &request.project_id);
    if let Err(error) = &result {
        if let Ok(mut project) = subtitle_store::read_project(&app, &request.project_id) {
            project.status = if cancel.is_cancelled() {
                "paused"
            } else {
                "failed"
            }
            .into();
            project.last_error = Some(error.to_string());
            finish_stage(
                &mut project,
                if cancel.is_cancelled() {
                    "cancelled"
                } else {
                    "failed"
                },
                Some(error.to_string()),
            );
            project.updated_at = subtitle_store::now_epoch();
            let _ = subtitle_store::write_project(&app, &project);
        }
        subtitle_store::clean_temp(&app, &request.project_id);
    }
    result
}

#[tauri::command]
pub async fn start_bing_translation(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    request: StartTextTaskRequest,
) -> Result<SubtitleProject, AppError> {
    let cancel = register_task(&state, &request.project_id)?;
    let result = async {
        let mut project = subtitle_store::read_project(&app, &request.project_id)?;
        if project.segments.is_empty() {
            return Err(AppError::user("项目还没有可翻译的字幕。", "No segments"));
        }
        project.status = "translate".into();
        project.last_error = None;
        begin_stage(&mut project, "bing_translation");
        subtitle_store::write_project(&app, &project)?;
        let mut translator = BingTranslator::new().await?;
        let project_id = project.id.clone();
        let result = translator
            .translate(&mut project.segments, "zh-Hans", &cancel, |batch, total| {
                emit(
                    &app,
                    SubtitleProgressEvent {
                        project_id: project_id.clone(),
                        stage: "translate".into(),
                        percent: batch as f32 / total.max(1) as f32 * 100.0,
                        chunk_index: Some(batch),
                        chunk_total: Some(total),
                        message: "正在使用必应免费翻译…".into(),
                        input_tokens: 0,
                        output_tokens: 0,
                        recoverable: true,
                    },
                );
            })
            .await?;
        if project.source_language.is_none() {
            project.source_language = result.detected_language;
        }
        project.target_language = Some("zh-CN".into());
        project.translation_provider = Some("bing".into());
        project.status = "ready".into();
        finish_stage(&mut project, "completed", None);
        project.updated_at = subtitle_store::now_epoch();
        subtitle_store::write_project(&app, &project)?;
        Ok(project)
    }
    .await;
    finish_task(&state, &request.project_id);
    if let Err(error) = &result {
        if let Ok(mut project) = subtitle_store::read_project(&app, &request.project_id) {
            project.status = if cancel.is_cancelled() {
                "paused"
            } else {
                "failed"
            }
            .into();
            project.last_error = Some(error.to_string());
            finish_stage(
                &mut project,
                if cancel.is_cancelled() {
                    "cancelled"
                } else {
                    "failed"
                },
                Some(error.to_string()),
            );
            project.updated_at = subtitle_store::now_epoch();
            let _ = subtitle_store::write_project(&app, &project);
        }
    }
    result
}

#[tauri::command]
pub async fn start_openai_compatible_translation(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    request: StartTextTaskRequest,
) -> Result<SubtitleProject, AppError> {
    let cancel = register_task(&state, &request.project_id)?;
    let result = async {
        let settings = subtitle_store::read_settings(&app)?;
        let client = OpenAiCompatibleClient::new(
            &settings.openai_api_base,
            load_openai_key()?,
            &settings.openai_model,
        )?;
        let mut project = subtitle_store::read_project(&app, &request.project_id)?;
        if project.segments.is_empty() {
            return Err(AppError::user("项目还没有可翻译的字幕。", "No segments"));
        }
        project.status = "translate".into();
        project.last_error = None;
        begin_stage(&mut project, "custom_ai_translation");
        subtitle_store::write_project(&app, &project)?;
        let project_id = project.id.clone();
        client
            .translate(
                &mut project.segments,
                "Simplified Chinese",
                &cancel,
                |batch, total| {
                    emit(
                        &app,
                        SubtitleProgressEvent {
                            project_id: project_id.clone(),
                            stage: "translate".into(),
                            percent: batch as f32 / total.max(1) as f32 * 100.0,
                            chunk_index: Some(batch),
                            chunk_total: Some(total),
                            message: "正在使用自定义 AI 翻译…".into(),
                            input_tokens: 0,
                            output_tokens: 0,
                            recoverable: true,
                        },
                    );
                },
            )
            .await?;
        project.target_language = Some("zh-CN".into());
        project.translation_provider = Some("openai-compatible".into());
        project.status = "ready".into();
        finish_stage(&mut project, "completed", None);
        project.updated_at = subtitle_store::now_epoch();
        subtitle_store::write_project(&app, &project)?;
        Ok(project)
    }
    .await;
    finish_task(&state, &request.project_id);
    if let Err(error) = &result {
        if let Ok(mut project) = subtitle_store::read_project(&app, &request.project_id) {
            project.status = if cancel.is_cancelled() {
                "paused"
            } else {
                "failed"
            }
            .into();
            project.last_error = Some(error.to_string());
            finish_stage(
                &mut project,
                if cancel.is_cancelled() {
                    "cancelled"
                } else {
                    "failed"
                },
                Some(error.to_string()),
            );
            project.updated_at = subtitle_store::now_epoch();
            let _ = subtitle_store::write_project(&app, &project);
        }
    }
    result
}

async fn text_task(
    app: &AppHandle,
    request: &StartTextTaskRequest,
    task: &str,
    cancel: &CancellationToken,
) -> Result<SubtitleProject, AppError> {
    let mut project = subtitle_store::read_project(app, &request.project_id)?;
    if project.segments.is_empty() {
        return Err(AppError::user("项目还没有可处理的字幕。", "No segments"));
    }
    let model = request
        .model
        .clone()
        .unwrap_or_else(|| project.model.clone());
    let settings = subtitle_store::read_settings(app)?;
    let client = GeminiClient::new(load_api_key()?)?;
    project.status = task.into();
    begin_stage(
        &mut project,
        if task == "translate" {
            "gemini_translation"
        } else {
            "gemini_polish"
        },
    );
    subtitle_store::write_project(app, &project)?;
    let total = project.segments.len().div_ceil(100);
    let mut cursor = 0;
    let concurrency = settings.max_concurrency.clamp(1, 2) as usize;
    while cursor < total {
        let batch_end = (cursor + concurrency).min(total);
        let mut outcomes = join_all((cursor..batch_end).map(|batch| {
            let start = batch * 100;
            let end = ((batch + 1) * 100).min(project.segments.len());
            let payload: Vec<Value> = project.segments[start..end]
                .iter()
                .map(|segment| json!({"id": segment.id, "sourceText": segment.source_text}))
                .collect();
            let client = client.clone();
            let model = model.clone();
            let task = task.to_string();
            let target_language = request.target_language.clone();
            let cancel = cancel.clone();
            async move {
                let result = client
                    .transform_segments(
                        &model,
                        &task,
                        target_language.as_deref(),
                        &json!({"items": payload}),
                        &cancel,
                    )
                    .await;
                (batch, start, end, result)
            }
        }))
        .await;
        outcomes.sort_by_key(|(batch, _, _, _)| *batch);

        for (batch, start, end, result) in outcomes {
            let (value, usage) = result?;
            let items = value["items"]
                .as_array()
                .ok_or_else(|| AppError::user("Gemini 文本结果无效。", "Missing items"))?;
            if items.len() != end - start {
                return Err(AppError::user(
                    "Gemini 改变了字幕段数量，结果未应用。",
                    "Item count mismatch",
                ));
            }
            for (segment, item) in project.segments[start..end].iter_mut().zip(items) {
                if item["id"].as_str() != Some(&segment.id) {
                    return Err(AppError::user(
                        "Gemini 改变了字幕顺序，结果未应用。",
                        "ID mismatch",
                    ));
                }
                let text = item["text"].as_str().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    return Err(AppError::user(
                        "Gemini 返回了空文本，结果未应用。",
                        "Empty text",
                    ));
                }
                if task == "translate" {
                    segment.translated_text = Some(text);
                } else {
                    segment.source_text = text;
                }
            }
            add_usage(&mut project.usage, &usage);
            emit(
                app,
                SubtitleProgressEvent {
                    project_id: project.id.clone(),
                    stage: task.into(),
                    percent: (batch + 1) as f32 / total as f32 * 100.0,
                    chunk_index: Some(batch + 1),
                    chunk_total: Some(total),
                    message: "文本批次已处理。".into(),
                    input_tokens: project.usage.input_tokens,
                    output_tokens: project.usage.output_tokens,
                    recoverable: false,
                },
            );
        }
        cursor = batch_end;
    }
    if task == "translate" {
        project.target_language = request.target_language.clone();
    }
    project.model = model;
    project.status = "ready".into();
    finish_stage(&mut project, "completed", None);
    project.updated_at = subtitle_store::now_epoch();
    subtitle_store::write_project(app, &project)?;
    Ok(project)
}

async fn run_text_command(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    request: StartTextTaskRequest,
    task: &str,
) -> Result<SubtitleProject, AppError> {
    let cancel = register_task(&state, &request.project_id)?;
    let result = text_task(&app, &request, task, &cancel).await;
    finish_task(&state, &request.project_id);
    if let Err(error) = &result {
        if let Ok(mut project) = subtitle_store::read_project(&app, &request.project_id) {
            project.status = if cancel.is_cancelled() {
                "paused"
            } else {
                "failed"
            }
            .into();
            project.last_error = Some(error.to_string());
            finish_stage(
                &mut project,
                if cancel.is_cancelled() {
                    "cancelled"
                } else {
                    "failed"
                },
                Some(error.to_string()),
            );
            project.updated_at = subtitle_store::now_epoch();
            let _ = subtitle_store::write_project(&app, &project);
        }
    }
    result
}

#[tauri::command]
pub async fn start_gemini_translation(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    request: StartTextTaskRequest,
) -> Result<SubtitleProject, AppError> {
    run_text_command(app, state, request, "translate").await
}

#[tauri::command]
pub async fn start_gemini_polish(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    request: StartTextTaskRequest,
) -> Result<SubtitleProject, AppError> {
    run_text_command(app, state, request, "polish").await
}

#[tauri::command]
pub fn cancel_subtitle_task(state: State<'_, SubtitleTaskState>, project_id: String) -> bool {
    if let Some(token) = state.tasks.lock().unwrap().get(&project_id) {
        token.cancel();
        true
    } else {
        false
    }
}

#[tauri::command]
pub fn save_subtitle_segments(
    app: AppHandle,
    request: SaveSegmentsRequest,
) -> Result<SubtitleProject, AppError> {
    let mut project = subtitle_store::read_project(&app, &request.project_id)?;
    validate_segments(&request.segments, project.duration_ms, false)
        .map_err(|e| AppError::user("字幕时间轴未通过检查。", e))?;
    project.segments = request.segments;
    project.updated_at = subtitle_store::now_epoch();
    subtitle_store::write_project(&app, &project)?;
    Ok(project)
}

#[tauri::command]
pub fn export_subtitles(
    app: AppHandle,
    request: ExportSubtitlesRequest,
) -> Result<String, AppError> {
    let mut project = subtitle_store::read_project(&app, &request.project_id)?;
    let output = available_output_path(Path::new(&request.output_path));
    project.status = "exporting".into();
    project.last_error = None;
    begin_stage(&mut project, "exporting");
    subtitle_store::write_project(&app, &project)?;
    if let Err(error) =
        subtitle_export::export(&project, &output, &request.format, &request.content)
    {
        project.status = "ready".into();
        project.last_error = Some(error.to_string());
        finish_stage(&mut project, "failed", Some(error.to_string()));
        project.updated_at = subtitle_store::now_epoch();
        let _ = subtitle_store::write_project(&app, &project);
        return Err(error);
    }
    project.artifacts.retain(|artifact| {
        !(artifact.kind == "subtitle" && artifact.path == output.to_string_lossy())
    });
    project.artifacts.push(SubtitleArtifact {
        kind: "subtitle".into(),
        path: output.to_string_lossy().to_string(),
        format: request.format,
        created_at: subtitle_store::now_epoch(),
    });
    project.updated_at = subtitle_store::now_epoch();
    project.status = "ready".into();
    finish_stage(&mut project, "completed", None);
    subtitle_store::write_project(&app, &project)?;
    Ok(output.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn burn_subtitles(
    app: AppHandle,
    state: State<'_, SubtitleTaskState>,
    request: BurnSubtitlesRequest,
) -> Result<String, AppError> {
    let cancel = register_task(&state, &request.project_id)?;
    let result = burn_subtitles_inner(&app, &request, &cancel).await;
    finish_task(&state, &request.project_id);
    if let Err(error) = &result {
        if let Ok(mut project) = subtitle_store::read_project(&app, &request.project_id) {
            project.status = "ready".into();
            project.last_error = Some(error.to_string());
            finish_stage(
                &mut project,
                if cancel.is_cancelled() {
                    "cancelled"
                } else {
                    "failed"
                },
                Some(error.to_string()),
            );
            project.updated_at = subtitle_store::now_epoch();
            let _ = subtitle_store::write_project(&app, &project);
        }
    }
    result
}

async fn burn_subtitles_inner(
    app: &AppHandle,
    request: &BurnSubtitlesRequest,
    cancel: &CancellationToken,
) -> Result<String, AppError> {
    let mut project = subtitle_store::read_project(&app, &request.project_id)?;
    let ass = subtitle_store::temp_dir(&app, &project.id)?.join("burn.ass");
    if let Some(parent) = ass.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let output = available_output_path(Path::new(&request.output_path));
    project.status = "burning".into();
    project.last_error = None;
    begin_stage(&mut project, "burning");
    project.updated_at = subtitle_store::now_epoch();
    subtitle_store::write_project(app, &project)?;
    emit(
        app,
        SubtitleProgressEvent {
            project_id: project.id.clone(),
            stage: "burning".into(),
            percent: 0.0,
            chunk_index: None,
            chunk_total: None,
            message: "正在生成带中文字幕的视频…".into(),
            input_tokens: project.usage.input_tokens,
            output_tokens: project.usage.output_tokens,
            recoverable: false,
        },
    );
    let project_id = project.id.clone();
    let input_tokens = project.usage.input_tokens;
    let output_tokens = project.usage.output_tokens;
    let burn_result = subtitle_export::burn(
        &project,
        &output,
        &request.content,
        &request.style,
        &ass,
        cancel,
        |percent, encoder| {
            emit(
                app,
                SubtitleProgressEvent {
                    project_id: project_id.clone(),
                    stage: "burning".into(),
                    percent,
                    chunk_index: None,
                    chunk_total: None,
                    message: format!("正在生成字幕视频 · {encoder}"),
                    input_tokens,
                    output_tokens,
                    recoverable: false,
                },
            );
        },
    )
    .await?;
    project.performance.encoder = Some(burn_result.encoder);
    project.performance.output_bytes = Some(burn_result.output_bytes);
    project.artifacts.push(SubtitleArtifact {
        kind: "video".into(),
        path: output.to_string_lossy().to_string(),
        format: "mp4".into(),
        created_at: subtitle_store::now_epoch(),
    });
    project.status = "completed".into();
    finish_stage(&mut project, "completed", None);
    project.updated_at = subtitle_store::now_epoch();
    subtitle_store::write_project(app, &project)?;
    emit(
        app,
        SubtitleProgressEvent {
            project_id: project.id.clone(),
            stage: "completed".into(),
            percent: 100.0,
            chunk_index: None,
            chunk_total: None,
            message: "中文字幕和字幕视频已生成。".into(),
            input_tokens: project.usage.input_tokens,
            output_tokens: project.usage.output_tokens,
            recoverable: false,
        },
    );
    subtitle_store::clean_temp(app, &project.id);
    Ok(output.to_string_lossy().to_string())
}

fn available_output_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 2..10_000 {
        let name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_overlapping_chunk_plan() {
        assert_eq!(
            chunks(1_800_000),
            vec![(0, 900_000), (898_000, 1_798_000), (1_796_000, 1_800_000)]
        );
    }

    #[test]
    fn accepts_cloud_and_local_models_for_projects() {
        assert!(is_supported_project_model(ECONOMY_MODEL));
        assert!(is_supported_project_model("small-q5"));
        assert!(is_supported_project_model("large-v3-turbo-q5"));
        assert!(!is_supported_project_model("unknown-model"));
    }
    #[test]
    fn normalizes_deduplication_text() {
        assert_eq!(normalized("Hello, 世界！"), normalized("hello 世界"));
    }

    #[test]
    fn parses_multiline_srt() {
        let segments = parse_srt(
            "1\r\n00:00:01,200 --> 00:00:03,400\r\nHello\r\nworld\r\n\r\n2\r\n00:00:04.000 --> 00:00:05.500\r\nNext\r\n",
        )
        .unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].start_ms, 1_200);
        assert_eq!(segments[0].source_text, "Hello\nworld");
        assert_eq!(segments[1].end_ms, 5_500);
    }

    #[test]
    fn classifies_subtitle_codecs_and_languages() {
        assert!(is_text_subtitle("ass"));
        assert!(!is_text_subtitle("hdmv_pgs_subtitle"));
        assert!(is_chinese_language("zh-cn"));
        assert!(!is_chinese_language("jpn"));
    }

    #[test]
    fn output_path_keeps_extension_when_numbering() {
        let missing = std::env::temp_dir().join(format!("ydlite-{}.srt", Uuid::new_v4()));
        assert_eq!(available_output_path(&missing), missing);
    }
}
