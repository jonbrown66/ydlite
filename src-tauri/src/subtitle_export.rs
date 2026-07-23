use std::fs;
use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio_util::sync::CancellationToken;

use crate::errors::AppError;
use crate::process_utils::hidden_command;
use crate::subtitle_types::{validate_segments, SubtitleProject, SubtitleSegment, SubtitleStyle};
use crate::tool_paths;

fn timestamp_srt(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

fn timestamp_vtt(ms: u64) -> String {
    timestamp_srt(ms).replace(',', ".")
}

fn segment_text(segment: &SubtitleSegment, content: &str) -> String {
    match content {
        "translated" => segment
            .translated_text
            .clone()
            .unwrap_or_else(|| segment.source_text.clone()),
        "bilingual" => {
            let translated = segment
                .translated_text
                .as_deref()
                .unwrap_or_default()
                .trim();
            if translated.is_empty() {
                segment.source_text.clone()
            } else {
                format!("{}\n{translated}", segment.source_text)
            }
        }
        _ => segment.source_text.clone(),
    }
}

pub fn render_srt(project: &SubtitleProject, content: &str) -> Result<String, AppError> {
    validate_segments(
        &project.segments,
        project.duration_ms,
        matches!(content, "translated" | "bilingual"),
    )
    .map_err(|error| AppError::user("字幕尚未通过导出检查。", error))?;
    Ok(project
        .segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            format!(
                "{}\n{} --> {}\n{}\n",
                index + 1,
                timestamp_srt(segment.start_ms),
                timestamp_srt(segment.end_ms),
                segment_text(segment, content)
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

pub fn render_vtt(project: &SubtitleProject, content: &str) -> Result<String, AppError> {
    validate_segments(
        &project.segments,
        project.duration_ms,
        matches!(content, "translated" | "bilingual"),
    )
    .map_err(|error| AppError::user("字幕尚未通过导出检查。", error))?;
    let body = project
        .segments
        .iter()
        .map(|segment| {
            format!(
                "{} --> {}\n{}\n",
                timestamp_vtt(segment.start_ms),
                timestamp_vtt(segment.end_ms),
                segment_text(segment, content)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!("WEBVTT\n\n{body}"))
}

pub fn render_text(project: &SubtitleProject, content: &str, markdown: bool) -> String {
    if markdown {
        let mut output = format!("# {}\n\n", project.title);
        for segment in &project.segments {
            output.push_str(&format!(
                "- `{}` {}\n",
                timestamp_vtt(segment.start_ms),
                segment_text(segment, content).replace('\n', " / ")
            ));
        }
        output
    } else {
        project
            .segments
            .iter()
            .map(|segment| segment_text(segment, content))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn export(
    project: &SubtitleProject,
    output_path: &Path,
    format: &str,
    content: &str,
) -> Result<(), AppError> {
    let rendered = match format {
        "srt" => render_srt(project, content)?,
        "vtt" => render_vtt(project, content)?,
        "txt" => render_text(project, content, false),
        "md" | "markdown" => render_text(project, content, true),
        _ => {
            return Err(AppError::user(
                "不支持该字幕格式。",
                format!("Unsupported format: {format}"),
            ))
        }
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let part_path = partial_output_path(output_path);
    if let Err(error) = fs::write(&part_path, rendered) {
        let _ = fs::remove_file(&part_path);
        return Err(error.into());
    }
    if let Err(error) = fs::rename(&part_path, output_path) {
        let _ = fs::remove_file(&part_path);
        return Err(error.into());
    }
    Ok(())
}

fn partial_output_path(output_path: &Path) -> std::path::PathBuf {
    let parent = output_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    match output_path.extension().and_then(|value| value.to_str()) {
        Some(extension) => parent.join(format!("{stem}.part.{extension}")),
        None => parent.join(format!("{stem}.part")),
    }
}

fn escape_ass(value: &str) -> String {
    value
        .replace('\\', r"\\")
        .replace('{', r"\{")
        .replace('}', r"\}")
        .replace('\n', r"\N")
}

fn timestamp_ass(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let centis = (ms % 1_000) / 10;
    format!("{hours}:{minutes:02}:{seconds:02}.{centis:02}")
}

fn validate_style(style: &SubtitleStyle) -> Result<(), AppError> {
    let valid_font = !style.font_family.trim().is_empty()
        && style.font_family.len() <= 64
        && !style
            .font_family
            .chars()
            .any(|character| matches!(character, ',' | '\n' | '\r' | '{' | '}'));
    let valid_color = |value: &str| {
        value.len() == 7
            && value.starts_with('#')
            && value[1..]
                .chars()
                .all(|character| character.is_ascii_hexdigit())
    };
    if !valid_font {
        return Err(AppError::user("字幕字体无效。", "Invalid ASS font family"));
    }
    if !(16..=120).contains(&style.font_size) || !(16..=120).contains(&style.translated_font_size) {
        return Err(AppError::user(
            "字幕字号需要在 16 到 120 之间。",
            "Invalid subtitle font size",
        ));
    }
    if !valid_color(&style.primary_color)
        || !valid_color(&style.translated_color)
        || !valid_color(&style.outline_color)
        || !valid_color(&style.background_color)
    {
        return Err(AppError::user(
            "字幕颜色格式无效。",
            "Expected #RRGGBB subtitle color",
        ));
    }
    if !(0.0..=8.0).contains(&style.outline_width) || !(0.0..=8.0).contains(&style.shadow) {
        return Err(AppError::user(
            "字幕描边或阴影超出允许范围。",
            "Invalid ASS outline or shadow",
        ));
    }
    if style.margin_vertical > 400
        || !matches!(style.position.as_str(), "top" | "middle" | "bottom")
    {
        return Err(AppError::user(
            "字幕位置设置无效。",
            "Invalid subtitle position",
        ));
    }
    Ok(())
}

fn ass_color(value: &str, alpha: u8) -> String {
    let red = &value[1..3];
    let green = &value[3..5];
    let blue = &value[5..7];
    format!("&H{alpha:02X}{blue}{green}{red}")
}

fn style_line(name: &str, style: &SubtitleStyle, translated: bool) -> String {
    let font_size = if translated {
        style.translated_font_size
    } else {
        style.font_size
    };
    let primary_color = if translated {
        &style.translated_color
    } else {
        &style.primary_color
    };
    let alignment = match style.position.as_str() {
        "top" => 8,
        "middle" => 5,
        _ => 2,
    };
    let border_style = if style.boxed { 3 } else { 1 };
    let background_alpha =
        255_u8.saturating_sub(((style.background_opacity as u16 * 255) / 100) as u8);
    format!(
        "Style: {name},{},{font_size},{},&H000000FF,{},{},{},0,0,0,100,100,0,0,{border_style},{},{},{alignment},80,80,{},1\n",
        style.font_family,
        ass_color(primary_color, 0),
        ass_color(&style.outline_color, 0),
        ass_color(&style.background_color, background_alpha),
        if style.bold { -1 } else { 0 },
        style.outline_width,
        style.shadow,
        style.margin_vertical,
    )
}

fn ass_segment_text(segment: &SubtitleSegment, content: &str, style: &SubtitleStyle) -> String {
    let source = escape_ass(&segment.source_text);
    let translated = escape_ass(
        segment
            .translated_text
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&segment.source_text),
    );
    match content {
        "translated" => format!(r"{{\rTranslated}}{translated}"),
        "bilingual" if style.translated_first => {
            format!(r"{{\rTranslated}}{translated}\N{{\rSource}}{source}")
        }
        "bilingual" => format!(r"{{\rSource}}{source}\N{{\rTranslated}}{translated}"),
        _ => format!(r"{{\rSource}}{source}"),
    }
}

pub fn render_ass(
    project: &SubtitleProject,
    content: &str,
    style: &SubtitleStyle,
) -> Result<String, AppError> {
    validate_segments(
        &project.segments,
        project.duration_ms,
        matches!(content, "translated" | "bilingual"),
    )
    .map_err(|error| AppError::user("字幕尚未通过烧录检查。", error))?;
    validate_style(style)?;
    let mut output = String::from(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: 1920\nPlayResY: 1080\nWrapStyle: 0\nScaledBorderAndShadow: yes\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n",
    );
    output.push_str(&style_line("Source", style, false));
    output.push_str(&style_line("Translated", style, true));
    output.push_str("\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n");
    for segment in &project.segments {
        output.push_str(&format!(
            "Dialogue: 0,{},{},Source,,0,0,0,,{}\n",
            timestamp_ass(segment.start_ms),
            timestamp_ass(segment.end_ms),
            ass_segment_text(segment, content, style)
        ));
    }
    Ok(output)
}

#[derive(Debug, Clone)]
pub struct BurnResult {
    pub encoder: String,
    pub output_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum VideoEncoder {
    Nvenc,
    Qsv,
    Amf,
    Cpu,
}

impl VideoEncoder {
    fn label(self) -> &'static str {
        match self {
            Self::Nvenc => "NVIDIA GPU",
            Self::Qsv => "Intel GPU",
            Self::Amf => "AMD GPU",
            Self::Cpu => "CPU",
        }
    }
}

pub async fn burn(
    project: &SubtitleProject,
    output_path: &Path,
    content: &str,
    style: &SubtitleStyle,
    ass_path: &Path,
    cancel: &CancellationToken,
    mut on_progress: impl FnMut(f32, &str) + Send,
) -> Result<BurnResult, AppError> {
    fs::write(ass_path, render_ass(project, content, style)?)?;
    let bitrate = source_video_bitrate(Path::new(&project.source_path), project.duration_ms).await;
    let escaped = ass_path
        .to_string_lossy()
        .replace('\\', "/")
        .replace(':', "\\:")
        .replace('\'', "\\'");
    let part_path = partial_output_path(output_path);
    let preferred = detect_encoder().await;
    let first = run_burn_attempt(
        project,
        &part_path,
        &escaped,
        bitrate,
        preferred,
        cancel,
        &mut on_progress,
    )
    .await;
    let encoder = match first {
        Ok(()) => preferred,
        Err(error) if preferred != VideoEncoder::Cpu && !cancel.is_cancelled() => {
            let _ = fs::remove_file(&part_path);
            on_progress(0.0, "GPU 不可用，已切换 CPU");
            run_burn_attempt(
                project,
                &part_path,
                &escaped,
                bitrate,
                VideoEncoder::Cpu,
                cancel,
                &mut on_progress,
            )
            .await
            .map_err(|cpu_error| {
                AppError::user(
                    "字幕视频生成失败。",
                    format!("GPU: {error}\nCPU fallback: {cpu_error}"),
                )
            })?;
            VideoEncoder::Cpu
        }
        Err(error) => {
            let _ = fs::remove_file(&part_path);
            let _ = fs::remove_file(ass_path);
            return Err(error);
        }
    };
    let validation = validate_burn_output(&part_path, project.duration_ms).await;
    let _ = fs::remove_file(ass_path);
    if let Err(error) = validation {
        let _ = fs::remove_file(&part_path);
        return Err(error);
    }
    if let Err(error) = fs::rename(&part_path, output_path) {
        let _ = fs::remove_file(&part_path);
        return Err(AppError::user(
            "无法保存字幕视频。",
            format!("{}: {error}", output_path.display()),
        ));
    }
    let output_bytes = fs::metadata(output_path)?.len();
    Ok(BurnResult {
        encoder: encoder.label().into(),
        output_bytes,
    })
}

async fn detect_encoder() -> VideoEncoder {
    let output = hidden_command(tool_paths::ffmpeg())
        .args(["-hide_banner", "-encoders"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;
    let encoders = output
        .ok()
        .filter(|result| result.status.success())
        .map(|result| String::from_utf8_lossy(&result.stdout).to_string())
        .unwrap_or_default();
    if encoders.contains("h264_nvenc") {
        VideoEncoder::Nvenc
    } else if encoders.contains("h264_qsv") {
        VideoEncoder::Qsv
    } else if encoders.contains("h264_amf") {
        VideoEncoder::Amf
    } else {
        VideoEncoder::Cpu
    }
}

async fn run_burn_attempt(
    project: &SubtitleProject,
    output_path: &Path,
    escaped_ass_path: &str,
    bitrate: Option<u64>,
    encoder: VideoEncoder,
    cancel: &CancellationToken,
    on_progress: &mut (dyn FnMut(f32, &str) + Send),
) -> Result<(), AppError> {
    let mut command = hidden_command(tool_paths::ffmpeg());
    command
        .args(["-y", "-i", &project.source_path, "-vf"])
        .arg(format!("ass='{escaped_ass_path}'"));
    match encoder {
        VideoEncoder::Nvenc => {
            command.args([
                "-c:v",
                "h264_nvenc",
                "-preset",
                "p5",
                "-cq",
                "22",
                "-b:v",
                "0",
                "-pix_fmt",
                "yuv420p",
            ]);
        }
        VideoEncoder::Qsv => {
            command.args([
                "-c:v",
                "h264_qsv",
                "-preset",
                "medium",
                "-global_quality",
                "22",
                "-pix_fmt",
                "yuv420p",
            ]);
        }
        VideoEncoder::Amf => {
            command.args([
                "-c:v", "h264_amf", "-quality", "balanced", "-rc", "vbr_peak", "-pix_fmt",
                "yuv420p",
            ]);
            if let Some(bits_per_second) = bitrate {
                command.args(["-b:v", &bits_per_second.to_string()]);
            }
        }
        VideoEncoder::Cpu => {
            command.args([
                "-c:v", "libx264", "-preset", "medium", "-crf", "22", "-pix_fmt", "yuv420p",
            ]);
        }
    }
    if let Some(bits_per_second) = bitrate {
        let (max_rate, buffer_size) = bitrate_limits(bits_per_second);
        command.args(["-maxrate", &max_rate, "-bufsize", &buffer_size]);
    }
    command
        .args([
            "-c:a",
            "copy",
            "-movflags",
            "+faststart",
            "-progress",
            "pipe:1",
            "-nostats",
        ])
        .arg(output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| AppError::user("无法启动字幕视频生成。", error.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::user("无法读取视频生成进度。", "Missing FFmpeg stdout"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::user("无法读取视频生成日志。", "Missing FFmpeg stderr"))?;
    let stderr_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes).await;
        bytes
    });
    let mut lines = BufReader::new(stdout).lines();
    on_progress(0.0, encoder.label());
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                let _ = stderr_task.await;
                let _ = fs::remove_file(output_path);
                return Err(AppError::user("字幕视频生成已取消。", "cancelled"));
            }
            line = lines.next_line() => {
                match line? {
                    Some(line) => {
                        if let Some(percent) = parse_burn_progress(&line, project.duration_ms) {
                            on_progress(percent, encoder.label());
                        }
                    }
                    None => break,
                }
            }
        }
    }
    let status = child.wait().await?;
    let stderr = stderr_task.await.unwrap_or_default();
    if !status.success() {
        let _ = fs::remove_file(output_path);
        return Err(AppError::user(
            "字幕视频生成失败。",
            String::from_utf8_lossy(&stderr).to_string(),
        ));
    }
    Ok(())
}

async fn validate_burn_output(path: &Path, expected_duration_ms: u64) -> Result<(), AppError> {
    let output = hidden_command(tool_paths::ffprobe())
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|error| AppError::user("无法校验字幕视频。", error.to_string()))?;
    let duration_ms = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .ok()
        .map(|seconds| (seconds * 1_000.0).round() as u64)
        .filter(|duration| *duration > 0)
        .ok_or_else(|| {
            AppError::user(
                "生成的视频无法读取，未标记为完成。",
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
        })?;
    let difference = duration_ms.abs_diff(expected_duration_ms);
    if difference > 1_000 {
        return Err(AppError::user(
            "生成的视频时长校验失败，未标记为完成。",
            format!(
                "expected={expected_duration_ms}ms, actual={duration_ms}ms, diff={difference}ms"
            ),
        ));
    }
    Ok(())
}

async fn source_video_bitrate(path: &Path, duration_ms: u64) -> Option<u64> {
    let output = hidden_command(tool_paths::ffprobe())
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=bit_rate",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    let probed = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|value| *value >= 100_000);
    probed.or_else(|| {
        let bytes = fs::metadata(path).ok()?.len();
        (duration_ms > 0).then(|| {
            ((bytes as u128 * 8 * 1_000 / duration_ms as u128) as u64).saturating_mul(95) / 100
        })
    })
}

fn bitrate_limits(source_bits_per_second: u64) -> (String, String) {
    let max_rate_kbps = source_bits_per_second.saturating_mul(105).div_ceil(100_000);
    (
        format!("{max_rate_kbps}k"),
        format!("{}k", max_rate_kbps.saturating_mul(2)),
    )
}

fn parse_burn_progress(line: &str, duration_ms: u64) -> Option<f32> {
    let micros = line.strip_prefix("out_time_us=")?.parse::<u64>().ok()?;
    if duration_ms == 0 {
        return None;
    }
    Some(((micros as f64 / 1_000.0 / duration_ms as f64) * 100.0).clamp(0.0, 99.0) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subtitle_types::SubtitleUsage;

    fn project() -> SubtitleProject {
        SubtitleProject {
            id: "p".into(),
            title: "Demo".into(),
            source_path: "demo.mp4".into(),
            source_fingerprint: "1".into(),
            duration_ms: 2_000,
            model: "gemini-3.1-flash-lite".into(),
            source_language: None,
            target_language: None,
            completed_chunks: vec![],
            segments: vec![SubtitleSegment {
                id: "s".into(),
                start_ms: 0,
                end_ms: 1_500,
                source_text: "Hello {world}".into(),
                translated_text: Some("你好".into()),
            }],
            usage: SubtitleUsage::default(),
            status: "ready".into(),
            created_at: 0,
            updated_at: 0,
            last_error: None,
            asr_provider: None,
            translation_provider: None,
            artifacts: vec![],
            performance: Default::default(),
        }
    }

    #[test]
    fn renders_srt_and_ass_safely() {
        assert!(render_srt(&project(), "bilingual")
            .unwrap()
            .contains("00:00:00,000 --> 00:00:01,500"));
        assert!(render_ass(&project(), "source", &SubtitleStyle::default())
            .unwrap()
            .contains(r"Hello \{world\}"));
    }

    #[test]
    fn renders_linked_bilingual_styles() {
        let style = SubtitleStyle {
            font_size: 52,
            translated_font_size: 50,
            primary_color: "#F8F8F8".into(),
            translated_color: "#FFD95A".into(),
            translated_first: true,
            ..SubtitleStyle::default()
        };
        let rendered = render_ass(&project(), "bilingual", &style).unwrap();
        assert!(rendered.contains("Style: Source,Microsoft YaHei,52"));
        assert!(rendered.contains("Style: Translated,Microsoft YaHei,50"));
        assert!(rendered.contains(r"{\rTranslated}你好\N{\rSource}Hello \{world\}"));

        let source_style = rendered
            .lines()
            .find(|line| line.starts_with("Style: Source,"))
            .unwrap()
            .split(',')
            .collect::<Vec<_>>();
        assert_eq!(source_style.len(), 23);
        assert_eq!(source_style[11], "100", "ScaleX must remain visible");
        assert_eq!(source_style[15], "1", "BorderStyle field must not shift");
        assert_eq!(source_style[18], "2", "Alignment field must not shift");
    }

    #[test]
    fn parses_ffmpeg_burn_progress() {
        let progress = parse_burn_progress("out_time_us=5000000", 10_000).unwrap();
        assert!((progress - 50.0).abs() < 0.01);
        assert!(parse_burn_progress("progress=continue", 10_000).is_none());
    }

    #[test]
    fn keeps_burn_bitrate_close_to_source() {
        let (max_rate, buffer_size) = bitrate_limits(3_744_505);
        assert_eq!(max_rate, "3932k");
        assert_eq!(buffer_size, "7864k");
    }

    #[test]
    fn partial_output_keeps_media_extension() {
        assert_eq!(
            partial_output_path(Path::new(r"E:\video\demo.mp4")),
            Path::new(r"E:\video\demo.part.mp4")
        );
        assert_eq!(
            partial_output_path(Path::new(r"E:\video\demo.srt")),
            Path::new(r"E:\video\demo.part.srt")
        );
    }
}
