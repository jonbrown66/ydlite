use std::path::Path;
use std::process::Stdio;

use serde::Deserialize;
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::errors::AppError;
use crate::process_utils::hidden_command;
use crate::subtitle_types::SubtitleSegment;
use crate::{tool_paths, whisper_manager};

#[derive(Debug, Deserialize)]
struct WhisperJson {
    #[serde(default)]
    result: WhisperResult,
    #[serde(default)]
    transcription: Vec<WhisperSegment>,
}

#[derive(Debug, Default, Deserialize)]
struct WhisperResult {
    #[serde(default)]
    language: String,
}

#[derive(Debug, Default, Deserialize)]
struct WhisperSegment {
    #[serde(default)]
    offsets: WhisperOffsets,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Default, Deserialize)]
struct WhisperOffsets {
    #[serde(default)]
    from: u64,
    #[serde(default)]
    to: u64,
}

pub struct WhisperTranscript {
    pub language: String,
    pub segments: Vec<SubtitleSegment>,
}

pub async fn extract_wav(source: &Path, output: &Path) -> Result<(), AppError> {
    let result = hidden_command(tool_paths::ffmpeg())
        .args(["-y", "-v", "error", "-i"])
        .arg(source)
        .args(["-vn", "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if !result.status.success() {
        return Err(AppError::user(
            "无法为 Whisper 提取音频。",
            String::from_utf8_lossy(&result.stderr),
        ));
    }
    Ok(())
}

pub async fn transcribe(
    app: &AppHandle,
    audio: &Path,
    output_prefix: &Path,
    model_id: &str,
    runtime_id: &str,
    duration_ms: u64,
    cancel: &CancellationToken,
) -> Result<WhisperTranscript, AppError> {
    let executable = whisper_manager::executable_path(app, runtime_id)?;
    let model = whisper_manager::ensure_model(app, model_id)?;
    let mut command = hidden_command(executable);
    command
        .arg("-m")
        .arg(model)
        .arg("-f")
        .arg(audio)
        .args([
            "-l",
            "auto",
            "-oj",
            "-of",
            &output_prefix.to_string_lossy(),
            "-t",
            "8",
            "-ml",
            "42",
            "-sow",
            "-np",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if runtime_id == "cpu" {
        command.arg("-ng");
    }
    let mut child = command
        .spawn()
        .map_err(|error| AppError::user("无法启动本地 Whisper。", error.to_string()))?;
    let status = tokio::select! {
        result = child.wait() => result?,
        _ = cancel.cancelled() => {
            let _ = child.kill().await;
            return Err(AppError::user("字幕任务已取消。", "cancelled"));
        }
    };
    if !status.success() {
        return Err(AppError::user(
            "本地 Whisper 识别失败。",
            format!("whisper-cli exited with {status}"),
        ));
    }
    let json_path = output_prefix.with_extension("json");
    let bytes = tokio::fs::read(&json_path).await.map_err(|error| {
        AppError::user(
            "Whisper 未生成可读取的字幕结果。",
            format!("{}: {error}", json_path.display()),
        )
    })?;
    let value = parse_whisper_json(&bytes)
        .map_err(|error| AppError::user("Whisper 字幕结果无法解析。", error.to_string()))?;
    let _ = tokio::fs::remove_file(json_path).await;
    normalize(value, duration_ms)
}

fn parse_whisper_json(bytes: &[u8]) -> Result<WhisperJson, serde_json::Error> {
    match serde_json::from_slice(bytes) {
        Ok(value) => Ok(value),
        Err(_) => {
            let repaired = repair_whisper_json(bytes);
            serde_json::from_str(
                repaired
                    .trim_start_matches('\u{feff}')
                    .trim_end_matches(|character: char| {
                        character == '\0' || character.is_whitespace()
                    }),
            )
        }
    }
}

fn repair_whisper_json(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let bytes = text.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    let mut in_string = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'"' {
            in_string = !in_string;
            output.push(byte);
            index += 1;
            continue;
        }

        if in_string && byte == b'\\' {
            let run_start = index;
            while index < bytes.len() && bytes[index] == b'\\' {
                index += 1;
            }
            let run_length = index - run_start;
            output.extend(std::iter::repeat_n(b'\\', run_length - (run_length % 2)));

            if run_length % 2 == 0 {
                continue;
            }

            let escape_start = index - 1;
            if let Some(code_point) = unicode_escape_at(bytes, escape_start) {
                if (0xD800..=0xDBFF).contains(&code_point) {
                    let low_escape_start = escape_start + 6;
                    if let Some(low) = unicode_escape_at(bytes, low_escape_start) {
                        if (0xDC00..=0xDFFF).contains(&low) {
                            output.extend_from_slice(&bytes[escape_start..low_escape_start + 6]);
                            index = low_escape_start + 6;
                            continue;
                        }
                    }
                    output.extend_from_slice(br"\uFFFD");
                    index = escape_start + 6;
                    continue;
                }
                if (0xDC00..=0xDFFF).contains(&code_point) {
                    output.extend_from_slice(br"\uFFFD");
                    index = escape_start + 6;
                    continue;
                }
                output.extend_from_slice(&bytes[escape_start..escape_start + 6]);
                index = escape_start + 6;
                continue;
            }

            output.push(b'\\');
            continue;
        }

        if in_string && byte < 0x20 {
            match byte {
                b'\n' => output.extend_from_slice(br"\n"),
                b'\r' => output.extend_from_slice(br"\r"),
                b'\t' => output.extend_from_slice(br"\t"),
                _ => output.extend_from_slice(format!(r"\u{byte:04X}").as_bytes()),
            }
            index += 1;
            continue;
        }

        if !in_string && byte == 0 {
            index += 1;
            continue;
        }

        output.push(byte);
        index += 1;
    }

    String::from_utf8(output).expect("lossy UTF-8 input must remain valid UTF-8")
}

fn unicode_escape_at(bytes: &[u8], start: usize) -> Option<u16> {
    if start + 6 > bytes.len() || bytes[start] != b'\\' || bytes[start + 1] != b'u' {
        return None;
    }
    let digits = std::str::from_utf8(&bytes[start + 2..start + 6]).ok()?;
    u16::from_str_radix(digits, 16).ok()
}

fn normalize(value: WhisperJson, duration_ms: u64) -> Result<WhisperTranscript, AppError> {
    let mut previous_end = 0_u64;
    let mut segments = Vec::with_capacity(value.transcription.len());
    for segment in value.transcription {
        let text = segment.text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        let start = segment.offsets.from.max(previous_end).min(duration_ms);
        let end = segment.offsets.to.min(duration_ms.saturating_add(2_000));
        if end <= start {
            continue;
        }
        previous_end = end;
        segments.push(SubtitleSegment {
            id: Uuid::new_v4().to_string(),
            start_ms: start,
            end_ms: end,
            source_text: text,
            translated_text: None,
        });
    }
    if segments.is_empty() {
        return Err(AppError::user(
            "Whisper 没有识别到可用语音。",
            "No transcription segments",
        ));
    }
    Ok(WhisperTranscript {
        language: if value.result.language.trim().is_empty() {
            "und".to_string()
        } else {
            value.result.language
        },
        segments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes_whisper_json() {
        let value: WhisperJson = serde_json::from_str(
            r#"{
                "result": {"language": "ja"},
                "transcription": [
                    {"offsets": {"from": 0, "to": 1200}, "text": " こんにちは "},
                    {"offsets": {"from": 1100, "to": 2500}, "text": "世界"}
                ]
            }"#,
        )
        .unwrap();
        let result = normalize(value, 3_000).unwrap();
        assert_eq!(result.language, "ja");
        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[1].start_ms, 1_200);
    }

    #[test]
    fn repairs_unpaired_unicode_surrogates_without_losing_the_transcript() {
        let value = parse_whisper_json(
            br#"{
                "result": {"language": "ko"},
                "transcription": [
                    {"offsets": {"from": 0, "to": 900}, "text": "hello \uD800 world"},
                    {"offsets": {"from": 900, "to": 1800}, "text": "\uDC00 next"}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(value.transcription.len(), 2);
        assert_eq!(value.transcription[0].text, "hello \u{fffd} world");
        assert_eq!(value.transcription[1].text, "\u{fffd} next");
    }

    #[test]
    fn preserves_valid_unicode_pairs_and_escaped_unicode_text() {
        let value = parse_whisper_json(
            br#"{
                "transcription": [
                    {"offsets": {"from": 0, "to": 900}, "text": "\uD83D\uDE00"},
                    {"offsets": {"from": 900, "to": 1800}, "text": "literal \\uD800"}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(value.transcription[0].text, "😀");
        assert_eq!(value.transcription[1].text, r"literal \uD800");
    }

    #[test]
    fn repairs_invalid_utf8_and_raw_control_characters() {
        let mut bytes =
            br#"{"transcription":[{"offsets":{"from":0,"to":900},"text":"bad "#.to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice(b" line\nbreak\"}]}");

        let value = parse_whisper_json(&bytes).unwrap();
        assert_eq!(value.transcription.len(), 1);
        assert_eq!(value.transcription[0].text, "bad \u{fffd} line\nbreak");
    }
}
