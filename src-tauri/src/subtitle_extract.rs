use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use regex::Regex;
use serde::Deserialize;
use tokio::fs;
use uuid::Uuid;

use crate::commands::{validate_url, ParseOptions};
use crate::errors::AppError;
use crate::process_utils::hidden_command;
use crate::tool_paths;
use crate::ytdlp;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtractSubtitlesRequest {
    pub url: String,
    pub dir: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub options: ParseOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cue {
    start_ms: u64,
    end_ms: u64,
    text: String,
}

#[derive(Debug)]
struct SubtitleTrack {
    language: String,
    cues: Vec<Cue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Json3Document {
    #[serde(default)]
    events: Vec<Json3Event>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Json3Event {
    t_start_ms: Option<f64>,
    d_duration_ms: Option<f64>,
    #[serde(default)]
    segs: Vec<Json3Segment>,
}

#[derive(Debug, Deserialize)]
struct Json3Segment {
    #[serde(default)]
    utf8: String,
}

#[tauri::command]
pub async fn extract_subtitles(request: ExtractSubtitlesRequest) -> Result<String, AppError> {
    let url = validate_url(&request.url)?.to_string();
    let dir = PathBuf::from(request.dir.trim());
    if !dir.is_dir() {
        return Err(AppError::user(
            "保存目录无效，请重新选择一个可用文件夹。",
            format!("Invalid directory: {}", dir.display()),
        ));
    }

    let temp_dir = std::env::temp_dir().join(format!("ydlite-subtitles-{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).await?;
    let result = extract_subtitles_inner(
        &url,
        &dir,
        request.title.trim(),
        &request.options,
        &temp_dir,
    )
    .await;
    let _ = fs::remove_dir_all(&temp_dir).await;
    result
}

async fn extract_subtitles_inner(
    url: &str,
    output_dir: &Path,
    title: &str,
    options: &ParseOptions,
    temp_dir: &Path,
) -> Result<String, AppError> {
    let output_template = temp_dir.join("%(title).200B.%(language)s.%(ext)s");
    let ytdlp_options = options.to_ytdlp_options(ytdlp::site_profile(url));
    let output = hidden_command(tool_paths::ytdlp())
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .args(subtitle_args(
            url,
            &output_template.to_string_lossy(),
            &ytdlp_options,
        ))
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| {
            AppError::user(
                "未检测到 yt-dlp。请先安装 yt-dlp，并确保它可以在命令行中直接运行。",
                error.to_string(),
            )
        })?;

    if !output.status.success() {
        return Err(AppError::user(
            "字幕提取失败，请查看详细日志。",
            command_detail(&output.stdout, &output.stderr),
        ));
    }

    let files = subtitle_files(temp_dir).await?;
    if files.is_empty() {
        return Err(AppError::user(
            "此视频没有可提取的站点字幕。",
            format!(
                "{}\n\n该视频可能没有提供人工字幕或自动字幕。下载视频后，可点击“创建字幕”使用语音识别生成字幕。",
                command_detail(&output.stdout, &output.stderr)
            ),
        ));
    }

    let mut tracks = Vec::new();
    for path in files {
        let bytes = fs::read(&path).await?;
        let content = String::from_utf8_lossy(&bytes);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let cues = parse_subtitle_file(&content, extension);
        if !cues.is_empty() {
            tracks.push(SubtitleTrack {
                language: subtitle_language(&path),
                cues,
            });
        }
    }

    if tracks.is_empty() {
        return Err(AppError::user(
            "字幕文件无法读取。",
            "The downloaded subtitle files did not contain readable cues.",
        ));
    }

    tracks.sort_by(|left, right| left.language.cmp(&right.language));
    let markdown = build_markdown(title, url, &tracks);
    let output_path = next_output_path(output_dir, title)?;
    fs::write(&output_path, markdown).await?;
    Ok(output_path.to_string_lossy().to_string())
}

fn subtitle_args(url: &str, output_template: &str, options: &ytdlp::YtdlpOptions) -> Vec<String> {
    let mut args = vec![
        "--skip-download".to_string(),
        "--write-subs".to_string(),
        "--write-auto-subs".to_string(),
        "--sub-langs".to_string(),
        "all".to_string(),
        "--sub-format".to_string(),
        "srt/vtt/ass/ssa/ttml/best".to_string(),
        "--no-playlist".to_string(),
        "--windows-filenames".to_string(),
        "--restrict-filenames".to_string(),
        "--encoding".to_string(),
        "utf-8".to_string(),
        "-o".to_string(),
        output_template.to_string(),
    ];
    args.extend(
        ["--extractor-args", "generic:impersonate"]
            .into_iter()
            .map(str::to_string),
    );
    args.extend(option_args(options));
    args.push(url.to_string());
    args
}

fn option_args(options: &ytdlp::YtdlpOptions) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(browser) = &options.cookies_from_browser {
        args.push("--cookies-from-browser".to_string());
        args.push(browser.clone());
    }
    if let Some(file) = &options.cookies_file {
        args.push("--cookies".to_string());
        args.push(file.clone());
    }
    for (name, value) in &options.headers {
        args.push("--add-header".to_string());
        args.push(format!("{name}:{value}"));
    }
    args
}

async fn subtitle_files(dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut entries = fs::read_dir(dir).await?;
    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_file() {
            continue;
        }
        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(
            extension.as_str(),
            "vtt"
                | "srt"
                | "ass"
                | "ssa"
                | "lrc"
                | "ttml"
                | "dfxp"
                | "xml"
                | "json3"
                | "srv1"
                | "srv2"
                | "srv3"
        ) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn subtitle_language(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let language = stem
        .rsplit_once('.')
        .map(|(_, value)| value)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("未知语言");
    match language.to_ascii_lowercase().as_str() {
        "na" | "none" | "unknown" => "未知语言".to_string(),
        _ => language.to_string(),
    }
}

fn parse_subtitle_file(content: &str, extension: &str) -> Vec<Cue> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    match extension.to_ascii_lowercase().as_str() {
        "ass" | "ssa" => parse_ass_cues(&normalized),
        "json3" => parse_json3_cues(&normalized),
        "ttml" | "dfxp" | "xml" | "srv1" | "srv2" | "srv3" => parse_xml_cues(&normalized),
        _ => parse_timed_cues(&normalized),
    }
}

fn parse_timed_cues(content: &str) -> Vec<Cue> {
    let lines: Vec<&str> = content.lines().collect();
    let mut cues = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let Some((start_ms, end_ms)) = parse_time_range(lines[index].trim()) else {
            index += 1;
            continue;
        };

        index += 1;
        let mut text_lines = Vec::new();
        while index < lines.len() {
            let line = lines[index].trim();
            if line.is_empty() || parse_time_range(line).is_some() {
                break;
            }
            text_lines.push(lines[index]);
            index += 1;
        }
        if let Some(text) = clean_caption_text(&text_lines) {
            push_cue(&mut cues, start_ms, end_ms, text);
        }
    }

    cues
}

fn parse_ass_cues(content: &str) -> Vec<Cue> {
    let mut cues = Vec::new();
    for line in content.lines() {
        let Some(dialogue) = line
            .strip_prefix("Dialogue:")
            .or_else(|| line.strip_prefix("dialogue:"))
        else {
            continue;
        };
        let fields: Vec<&str> = dialogue.splitn(10, ',').collect();
        if fields.len() < 10 {
            continue;
        }
        let (Some(start_ms), Some(end_ms)) =
            (parse_timestamp(fields[1]), parse_timestamp(fields[2]))
        else {
            continue;
        };
        let text_lines = fields[9].split("\\N").collect::<Vec<_>>();
        if let Some(text) = clean_caption_text(&text_lines) {
            push_cue(&mut cues, start_ms, end_ms, text);
        }
    }
    cues
}

fn parse_json3_cues(content: &str) -> Vec<Cue> {
    let Ok(document) = serde_json::from_str::<Json3Document>(content) else {
        return Vec::new();
    };

    let mut cues = Vec::new();
    for event in document.events {
        let (Some(start_ms), Some(duration_ms)) = (
            event.t_start_ms.and_then(millis_from_number),
            event.d_duration_ms.and_then(millis_from_number),
        ) else {
            continue;
        };
        let Some(end_ms) = start_ms
            .checked_add(duration_ms)
            .filter(|end| *end > start_ms)
        else {
            continue;
        };
        let text = event
            .segs
            .iter()
            .map(|segment| segment.utf8.as_str())
            .collect::<String>();
        if let Some(text) = clean_caption_text(&[text.as_str()]) {
            push_cue(&mut cues, start_ms, end_ms, text);
        }
    }

    cues
}

fn parse_xml_cues(content: &str) -> Vec<Cue> {
    let Ok(paragraphs) = Regex::new(r"(?is)<p\b(?P<attributes>[^>]*)>(?P<text>.*?)</p>") else {
        return Vec::new();
    };
    let Ok(line_breaks) = Regex::new(r"(?i)<br\s*/?>") else {
        return Vec::new();
    };

    let mut cues = Vec::new();
    for paragraph in paragraphs.captures_iter(content) {
        let attributes = paragraph
            .name("attributes")
            .map(|value| value.as_str())
            .unwrap_or_default();
        let Some(start_ms) = xml_attribute(attributes, "begin")
            .or_else(|| xml_attribute(attributes, "start"))
            .as_deref()
            .and_then(parse_xml_timestamp)
        else {
            continue;
        };
        let end_ms = xml_attribute(attributes, "end")
            .as_deref()
            .and_then(parse_xml_timestamp)
            .or_else(|| {
                xml_attribute(attributes, "dur")
                    .as_deref()
                    .and_then(parse_xml_timestamp)
                    .and_then(|duration| start_ms.checked_add(duration))
            });
        let Some(end_ms) = end_ms.filter(|end| *end > start_ms) else {
            continue;
        };
        let body = paragraph
            .name("text")
            .map(|value| line_breaks.replace_all(value.as_str(), "\n").into_owned())
            .unwrap_or_default();
        if let Some(text) = clean_caption_text(&[body.as_str()]) {
            push_cue(&mut cues, start_ms, end_ms, text);
        }
    }

    cues
}

fn xml_attribute(attributes: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"(?i)\b{name}\s*=\s*[\"'](?P<value>[^\"']+)[\"']"#);
    Regex::new(&pattern)
        .ok()?
        .captures(attributes)?
        .name("value")
        .map(|value| value.as_str().to_string())
}

fn parse_xml_timestamp(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Some(milliseconds) = value.strip_suffix("ms") {
        return milliseconds
            .parse::<f64>()
            .ok()
            .and_then(millis_from_number);
    }
    if let Some(seconds) = value.strip_suffix('s') {
        return seconds
            .parse::<f64>()
            .ok()
            .and_then(|seconds| millis_from_number(seconds * 1000.0));
    }
    if let Ok(seconds) = value.parse::<f64>() {
        return millis_from_number(seconds * 1000.0);
    }
    parse_timestamp(value)
}

fn millis_from_number(value: f64) -> Option<u64> {
    if value.is_finite() && value >= 0.0 && value <= u64::MAX as f64 {
        Some(value.round() as u64)
    } else {
        None
    }
}

fn parse_time_range(line: &str) -> Option<(u64, u64)> {
    let (start, end) = line.split_once("-->")?;
    Some((
        parse_timestamp(start.trim().split_whitespace().next()?)?,
        parse_timestamp(end.trim().split_whitespace().next()?)?,
    ))
}

fn parse_timestamp(value: &str) -> Option<u64> {
    let normalized = value.trim().replace(',', ".");
    let parts: Vec<&str> = normalized.split(':').collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [minutes, seconds] => (
            0.0,
            minutes.parse::<f64>().ok()?,
            seconds.parse::<f64>().ok()?,
        ),
        [hours, minutes, seconds] => (
            hours.parse::<f64>().ok()?,
            minutes.parse::<f64>().ok()?,
            seconds.parse::<f64>().ok()?,
        ),
        _ => return None,
    };
    if hours < 0.0 || minutes < 0.0 || seconds < 0.0 {
        return None;
    }
    Some(((hours * 3600.0 + minutes * 60.0 + seconds) * 1000.0).round() as u64)
}

fn clean_caption_text(lines: &[&str]) -> Option<String> {
    let text = lines
        .iter()
        .map(|line| decode_html_entities(&strip_markup(line)).replace("\\N", "\n"))
        .flat_map(|line| {
            line.lines()
                .map(str::trim)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if text.is_empty() {
        None
    } else {
        Some(text.join(" / "))
    }
}

fn strip_markup(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => result.push(character),
            _ => {}
        }
    }
    result
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn push_cue(cues: &mut Vec<Cue>, start_ms: u64, end_ms: u64, text: String) {
    let cue = Cue {
        start_ms,
        end_ms,
        text,
    };
    if cues.last() != Some(&cue) {
        cues.push(cue);
    }
}

fn build_markdown(title: &str, source_url: &str, tracks: &[SubtitleTrack]) -> String {
    let mut markdown = String::new();
    let title = single_line(if title.trim().is_empty() {
        "视频字幕"
    } else {
        title
    });
    writeln!(&mut markdown, "# {title}\n").expect("writing to a String cannot fail");
    writeln!(&mut markdown, "> 来源：{}\n", single_line(source_url))
        .expect("writing to a String cannot fail");

    for track in tracks {
        let language = single_line(&track.language);
        writeln!(&mut markdown, "## {language}\n").expect("writing to a String cannot fail");
        for cue in &track.cues {
            writeln!(
                &mut markdown,
                "- **{}** {}",
                format_timestamp(cue.start_ms),
                escape_markdown(&cue.text)
            )
            .expect("writing to a String cannot fail");
        }
        markdown.push('\n');
    }

    markdown
}

fn format_timestamp(milliseconds: u64) -> String {
    let total_seconds = milliseconds / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let millis = milliseconds % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

fn escape_markdown(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sanitize_filename(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches([' ', '.']);
    let base = if trimmed.is_empty() {
        "视频字幕"
    } else {
        trimmed
    };
    base.chars().take(120).collect()
}

fn next_output_path(output_dir: &Path, title: &str) -> Result<PathBuf, AppError> {
    let base_name = sanitize_filename(title);
    let first = output_dir.join(format!("{base_name}.md"));
    if !first.exists() {
        return Ok(first);
    }

    for index in 2..10_000 {
        let candidate = output_dir.join(format!("{base_name} ({index}).md"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(AppError::user(
        "无法创建字幕文件。",
        format!("Too many existing files named {base_name}.md"),
    ))
}

fn command_detail(stdout: &[u8], stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    if stdout.is_empty() {
        "yt-dlp exited without a detailed error.".to_string()
    } else {
        stdout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vtt_cues_and_removes_markup() {
        let cues = parse_subtitle_file(
            "WEBVTT\n\n00:00.000 --> 00:01.500\n<c.white>Hello &amp; welcome</c>\n\n",
            "vtt",
        );

        assert_eq!(
            cues,
            vec![Cue {
                start_ms: 0,
                end_ms: 1_500,
                text: "Hello & welcome".to_string(),
            }]
        );
    }

    #[test]
    fn parses_ass_dialogue_text() {
        let cues = parse_subtitle_file(
            "[Events]\nDialogue: 0,0:00:01.00,0:00:02.50,Default,Name,0,0,0,0,Hello\\NWorld\n",
            "ass",
        );

        assert_eq!(cues[0].start_ms, 1_000);
        assert_eq!(cues[0].end_ms, 2_500);
        assert_eq!(cues[0].text, "Hello / World");
    }

    #[test]
    fn parses_json3_cues() {
        let cues = parse_subtitle_file(
            r#"{"events":[{"tStartMs":1000,"dDurationMs":1500,"segs":[{"utf8":"Hello <b>world</b>"}]}]}"#,
            "json3",
        );

        assert_eq!(
            cues,
            vec![Cue {
                start_ms: 1_000,
                end_ms: 2_500,
                text: "Hello world".to_string(),
            }]
        );
    }

    #[test]
    fn parses_ttml_cues() {
        let cues = parse_subtitle_file(
            r#"<tt><body><div><p begin="00:00:01.000" end="00:00:02.250">Hello<br/>world</p></div></body></tt>"#,
            "ttml",
        );

        assert_eq!(
            cues,
            vec![Cue {
                start_ms: 1_000,
                end_ms: 2_250,
                text: "Hello / world".to_string(),
            }]
        );
    }

    #[test]
    fn creates_unique_safe_markdown_names() {
        assert_eq!(sanitize_filename("a:b?.mp4"), "a_b_.mp4");
    }

    #[test]
    fn subtitle_args_request_subtitles_without_video() {
        let args = subtitle_args(
            "https://example.com/video",
            "C:\\Temp\\%(title)s.%(language)s.%(ext)s",
            &ytdlp::YtdlpOptions::default(),
        );

        assert!(args.iter().any(|arg| arg == "--skip-download"));
        assert!(args.iter().any(|arg| arg == "--write-subs"));
        assert!(args.iter().any(|arg| arg == "--write-auto-subs"));
        assert!(args.windows(2).any(|pair| pair == ["--sub-langs", "all"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--sub-format", "srt/vtt/ass/ssa/ttml/best"]));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://example.com/video")
        );
    }
}
