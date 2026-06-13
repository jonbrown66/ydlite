use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::process::Command;
use url::Url;

use crate::downloader::{run_download, DownloadState};
use crate::errors::AppError;
use crate::tool_paths;
use crate::ytdlp;

#[derive(Debug, Serialize)]
pub struct DependencyStatus {
    pub ytdlp_ok: bool,
    pub ytdlp_version: Option<String>,
    pub ytdlp_latest_version: Option<String>,
    pub ytdlp_update_available: bool,
    pub ytdlp_path: String,
    pub ffmpeg_ok: bool,
    pub ffmpeg_version: Option<String>,
    pub ffmpeg_path: String,
}

#[derive(Debug, Serialize)]
pub struct VideoFormat {
    #[serde(rename = "formatId")]
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    pub height: Option<u32>,
    pub note: Option<String>,
    pub filesize: Option<u64>,
    #[serde(rename = "filesizeApprox")]
    pub filesize_approx: Option<u64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlaylistEntry {
    pub id: String,
    pub title: String,
    pub url: String,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VideoInfo {
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub extractor: Option<String>,
    #[serde(rename = "originalUrl")]
    pub original_url: String,
    #[serde(rename = "resolvedUrl")]
    pub resolved_url: String,
    pub site: String,
    #[serde(rename = "parseStrategy")]
    pub parse_strategy: String,
    #[serde(rename = "isPlaylist")]
    pub is_playlist: bool,
    pub entries: Option<Vec<PlaylistEntry>>,
    pub formats: Option<Vec<VideoFormat>>,
    #[serde(rename = "cookieSource")]
    pub cookie_source: CookieSource,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    pub dir: String,
    pub mode: DownloadMode,
    pub format_id: Option<String>,
    #[serde(default)]
    pub options: ParseOptions,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ParseVideoRequest {
    pub url: String,
    #[serde(default)]
    pub options: ParseOptions,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ParseOptions {
    #[serde(default)]
    pub cookie_source: CookieSource,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            cookie_source: CookieSource::None,
        }
    }
}

impl ParseOptions {
    pub fn to_ytdlp_options(&self, site: ytdlp::SiteProfile) -> ytdlp::YtdlpOptions {
        let mut options = ytdlp::YtdlpOptions::default();
        match &self.cookie_source {
            CookieSource::None => {}
            CookieSource::Browser { browser } => {
                options.cookies_from_browser = Some(browser.to_ytdlp_name().to_string());
            }
            CookieSource::File { path } => {
                let trimmed = path.trim();
                if !trimmed.is_empty() {
                    options.cookies_file = Some(trimmed.to_string());
                }
            }
        }
        if site == ytdlp::SiteProfile::Bilibili {
            options.headers.push((
                "Referer".to_string(),
                "https://www.bilibili.com/".to_string(),
            ));
            options
                .headers
                .push(("Origin".to_string(), "https://www.bilibili.com".to_string()));
        }
        options
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CookieSource {
    #[default]
    None,
    Browser {
        browser: BrowserCookieSource,
    },
    File {
        path: String,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum BrowserCookieSource {
    Firefox,
    Chrome,
    Edge,
}

impl BrowserCookieSource {
    fn to_ytdlp_name(&self) -> &'static str {
        match self {
            BrowserCookieSource::Firefox => "firefox",
            BrowserCookieSource::Chrome => "chrome",
            BrowserCookieSource::Edge => "edge",
        }
    }
}

#[derive(Debug)]
struct ParseAttempt {
    strategy: String,
    url: String,
    stderr: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum DownloadMode {
    Best,
    P720,
    Mp3,
    Custom,
}

#[tauri::command]
pub async fn check_dependencies() -> Result<DependencyStatus, AppError> {
    let ytdlp_path = tool_paths::ytdlp();
    let ffmpeg_path = tool_paths::ffmpeg();
    let ytdlp_version = command_version(ytdlp_path.clone(), "--version").await;
    let ffmpeg_version = command_version(ffmpeg_path.clone(), "-version").await;

    Ok(DependencyStatus {
        ytdlp_ok: ytdlp_version.is_some(),
        ytdlp_version,
        ytdlp_latest_version: None,
        ytdlp_update_available: false,
        ytdlp_path: ytdlp_path.to_string_lossy().to_string(),
        ffmpeg_ok: ffmpeg_version.is_some(),
        ffmpeg_version,
        ffmpeg_path: ffmpeg_path.to_string_lossy().to_string(),
    })
}

#[derive(Debug, Serialize)]
pub struct YtdlpUpdateStatus {
    pub ytdlp_latest_version: Option<String>,
    pub ytdlp_update_available: bool,
}

#[tauri::command]
pub async fn check_ytdlp_update() -> Result<YtdlpUpdateStatus, AppError> {
    let current = command_version(tool_paths::ytdlp(), "--version").await;
    let latest = latest_ytdlp_version().await;
    let update_available = match (&current, &latest) {
        (Some(current), Some(latest)) => version_is_newer(latest, current),
        _ => false,
    };

    Ok(YtdlpUpdateStatus {
        ytdlp_latest_version: latest,
        ytdlp_update_available: update_available,
    })
}

async fn latest_ytdlp_version() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .ok()?;
    let response = client
        .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
        .header(reqwest::header::USER_AGENT, "YDLite")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&response.text().await.ok()?).ok()?;
    value
        .get("tag_name")
        .and_then(|tag| tag.as_str())
        .map(normalize_version)
}

fn normalize_version(version: &str) -> String {
    version.trim().trim_start_matches('v').to_string()
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    normalize_version(latest) > normalize_version(current)
}

async fn resolve_redirects(url: &str) -> Option<String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return None;
    }

    let lower = url.to_ascii_lowercase();
    let is_short_url = lower.contains("b23.tv")
        || lower.contains("v.douyin.com")
        || lower.contains("xhslink.com")
        || lower.contains("kuaishou.com")
        || lower.contains("t.co")
        || lower.contains("dwz.cn");

    if !is_short_url {
        return Some(url.to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .ok()?;

    if let Ok(response) = client.head(url).send().await {
        return Some(response.url().to_string());
    }

    if let Ok(response) = client.get(url).send().await {
        return Some(response.url().to_string());
    }

    None
}

fn remove_tracking_params(url: &mut Url) {
    let tracking_keys = [
        "utm_source",
        "utm_medium",
        "utm_campaign",
        "utm_term",
        "utm_content",
        "spm_id_from",
        "vd_source",
        "click_id",
        "fbclid",
        "gclid",
        "msclkid",
        "_from",
        "callback",
    ];
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| !tracking_keys.iter().any(|&tk| key.as_ref() == tk))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();

    url.set_query(None);
    if !pairs.is_empty() {
        let mut query = url.query_pairs_mut();
        for (key, value) in pairs {
            query.append_pair(&key, &value);
        }
    }
}

#[tauri::command]
pub async fn parse_video(request: ParseVideoRequest) -> Result<VideoInfo, AppError> {
    let original_url = validate_url(&request.url)?.to_string();

    let resolved_url = resolve_redirects(&original_url)
        .await
        .unwrap_or_else(|| original_url.clone());
    let site = ytdlp::site_profile(&resolved_url);
    let cleaned_url = clean_url_for_site(&resolved_url, site);

    let mut attempts = Vec::new();
    let mut attempts_queue = Vec::new();

    let browsers = [
        (BrowserCookieSource::Chrome, "chrome"),
        (BrowserCookieSource::Edge, "edge"),
        (BrowserCookieSource::Firefox, "firefox"),
    ];

    if !matches!(request.options.cookie_source, CookieSource::None) {
        attempts_queue.push((
            cleaned_url.clone(),
            request.options.cookie_source.clone(),
            "clean URL + selected cookies".to_string(),
        ));
        if cleaned_url != original_url {
            attempts_queue.push((
                original_url.clone(),
                request.options.cookie_source.clone(),
                "original URL + selected cookies".to_string(),
            ));
        }
    }

    attempts_queue.push((
        cleaned_url.clone(),
        CookieSource::None,
        "clean URL + no cookies".to_string(),
    ));
    if cleaned_url != original_url {
        attempts_queue.push((
            original_url.clone(),
            CookieSource::None,
            "original URL + no cookies".to_string(),
        ));
    }

    for (browser_enum, browser_name) in browsers {
        attempts_queue.push((
            cleaned_url.clone(),
            CookieSource::Browser {
                browser: browser_enum.clone(),
            },
            format!("clean URL + {} cookies", browser_name),
        ));
        if cleaned_url != original_url {
            attempts_queue.push((
                original_url.clone(),
                CookieSource::Browser {
                    browser: browser_enum.clone(),
                },
                format!("original URL + {} cookies", browser_name),
            ));
        }
    }

    for (candidate_url, cookie_src, strategy) in attempts_queue {
        let options = ParseOptions {
            cookie_source: cookie_src.clone(),
        };

        match run_parse_attempt(&candidate_url, &options, site, &strategy, true).await {
            Ok(value) => {
                let is_playlist = value.get("_type").and_then(|t| t.as_str()) == Some("playlist")
                    || value.get("entries").is_some();

                if is_playlist {
                    return video_info_from_value(
                        value,
                        original_url,
                        candidate_url,
                        site,
                        strategy,
                        true,
                        cookie_src,
                    );
                } else {
                    match run_parse_attempt(&candidate_url, &options, site, &strategy, false).await
                    {
                        Ok(deep_value) => {
                            return video_info_from_value(
                                deep_value,
                                original_url,
                                candidate_url,
                                site,
                                strategy,
                                false,
                                cookie_src,
                            );
                        }
                        Err(_) => {
                            return video_info_from_value(
                                value,
                                original_url,
                                candidate_url,
                                site,
                                strategy,
                                false,
                                cookie_src,
                            );
                        }
                    }
                }
            }
            Err(attempt) => attempts.push(attempt),
        }
    }

    Err(AppError::user(
        "Could not parse this link. If it needs login, make sure the browser is signed in and not locked.",
        format_attempts(&attempts),
    ))
}

async fn run_parse_attempt(
    url: &str,
    options: &ParseOptions,
    site: ytdlp::SiteProfile,
    strategy: &str,
    flat_playlist: bool,
) -> Result<serde_json::Value, ParseAttempt> {
    let ytdlp_options = options.to_ytdlp_options(site);
    let output = Command::new(tool_paths::ytdlp())
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .args(ytdlp::parse_args(url, flat_playlist, &ytdlp_options))
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| ParseAttempt {
            strategy: strategy.to_string(),
            url: url.to_string(),
            stderr: error.to_string(),
        })?;

    if !output.status.success() {
        return Err(ParseAttempt {
            strategy: strategy.to_string(),
            url: url.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    serde_json::from_slice(&output.stdout).map_err(|error| ParseAttempt {
        strategy: strategy.to_string(),
        url: url.to_string(),
        stderr: error.to_string(),
    })
}

fn video_info_from_value(
    value: serde_json::Value,
    original_url: String,
    resolved_url: String,
    site: ytdlp::SiteProfile,
    parse_strategy: String,
    is_playlist: bool,
    cookie_source: CookieSource,
) -> Result<VideoInfo, AppError> {
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名视频")
        .to_string();

    let uploader = value
        .get("uploader")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let duration = value.get("duration").and_then(|v| v.as_f64());
    let thumbnail = value
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let extractor = value
        .get("extractor")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let entries = if is_playlist {
        if let Some(entries_value) = value.get("entries").and_then(|e| e.as_array()) {
            let mut list = Vec::new();
            for entry in entries_value {
                let id = entry
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let title = entry
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("未命名分集")
                    .to_string();
                let url = entry
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let resolved_entry_url = if url.is_empty() && !id.is_empty() {
                    if original_url.contains("bilibili.com") {
                        format!("https://www.bilibili.com/video/{}", id)
                    } else if original_url.contains("youtube.com")
                        || original_url.contains("youtu.be")
                    {
                        format!("https://www.youtube.com/watch?v={}", id)
                    } else {
                        id.clone()
                    }
                } else {
                    url
                };

                list.push(PlaylistEntry {
                    id,
                    title,
                    url: resolved_entry_url,
                    duration: entry.get("duration").and_then(|v| v.as_f64()),
                    uploader: entry
                        .get("uploader")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                });
            }
            Some(list)
        } else {
            None
        }
    } else {
        None
    };

    let formats = if !is_playlist {
        if let Some(formats_value) = value.get("formats").and_then(|f| f.as_array()) {
            let mut list = Vec::new();
            for fmt in formats_value {
                let format_id = fmt
                    .get("format_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if format_id.is_empty() {
                    continue;
                }
                let ext = fmt
                    .get("ext")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let resolution = fmt
                    .get("resolution")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                let height = fmt.get("height").and_then(|v| v.as_u64()).map(|v| v as u32);
                let note = fmt
                    .get("format_note")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                let filesize = fmt.get("filesize").and_then(|v| v.as_u64());
                let filesize_approx = fmt.get("filesize_approx").and_then(|v| v.as_u64());
                let vcodec = fmt
                    .get("vcodec")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                let acodec = fmt
                    .get("acodec")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                if vcodec.as_deref() == Some("none") && acodec.as_deref() == Some("none") {
                    continue;
                }

                list.push(VideoFormat {
                    format_id,
                    ext,
                    resolution,
                    height,
                    note,
                    filesize,
                    filesize_approx,
                    vcodec,
                    acodec,
                });
            }
            Some(list)
        } else {
            None
        }
    } else {
        None
    };

    Ok(VideoInfo {
        title,
        uploader,
        duration,
        thumbnail,
        extractor,
        original_url,
        resolved_url,
        site: site_label(site).to_string(),
        parse_strategy,
        is_playlist,
        entries,
        formats,
        cookie_source,
    })
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, DownloadState>,
    request: DownloadRequest,
) -> Result<(), AppError> {
    validate_url(&request.url)?;
    let dir = PathBuf::from(request.dir.trim());
    if !dir.is_dir() {
        return Err(AppError::user(
            "保存目录无效，请重新选择一个可用文件夹。",
            format!("Invalid directory: {}", dir.display()),
        ));
    }

    run_download(app, state.inner().clone(), request, dir).await
}

fn clean_url_for_site(input: &str, site: ytdlp::SiteProfile) -> String {
    let Ok(mut url) = Url::parse(input) else {
        return input.to_string();
    };

    match site {
        ytdlp::SiteProfile::Bilibili => retain_query_params(&mut url, &["p", "page"]),
        ytdlp::SiteProfile::Youtube => {
            retain_query_params(&mut url, &["v", "list", "index", "t", "start"])
        }
        ytdlp::SiteProfile::Twitter | ytdlp::SiteProfile::Instagram => {
            url.set_query(None);
        }
        ytdlp::SiteProfile::Generic => {
            remove_tracking_params(&mut url);
        }
    }

    url.to_string()
}

fn retain_query_params(url: &mut Url, allowed: &[&str]) {
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| {
            allowed
                .iter()
                .any(|allowed_key| key.as_ref() == *allowed_key)
        })
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();

    url.set_query(None);
    if !pairs.is_empty() {
        let mut query = url.query_pairs_mut();
        for (key, value) in pairs {
            query.append_pair(&key, &value);
        }
    }
}

fn format_attempts(attempts: &[ParseAttempt]) -> String {
    if attempts.is_empty() {
        return "No parse attempts were executed".to_string();
    }

    attempts
        .iter()
        .enumerate()
        .map(|(index, attempt)| {
            format!(
                "Attempt {}: {}\nURL: {}\n{}",
                index + 1,
                attempt.strategy,
                attempt.url,
                attempt.stderr.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn site_label(site: ytdlp::SiteProfile) -> &'static str {
    match site {
        ytdlp::SiteProfile::Bilibili => "bilibili",
        ytdlp::SiteProfile::Youtube => "youtube",
        ytdlp::SiteProfile::Twitter => "twitter",
        ytdlp::SiteProfile::Instagram => "instagram",
        ytdlp::SiteProfile::Generic => "generic",
    }
}

#[tauri::command]
pub async fn cancel_download(state: State<'_, DownloadState>) -> Result<(), AppError> {
    state.cancel_current().await
}

#[tauri::command]
pub async fn open_path(app: AppHandle, path: String) -> Result<(), AppError> {
    let target = PathBuf::from(path.trim());
    if !target.exists() {
        return Err(AppError::user(
            "无法打开文件，目标文件不存在。",
            format!("Missing path: {}", target.display()),
        ));
    }

    app.opener()
        .open_path(target.to_string_lossy().to_string(), None::<&str>)
        .map_err(|error| AppError::user("打开文件失败。", error.to_string()))
}

#[tauri::command]
pub async fn open_parent_folder(app: AppHandle, path: String) -> Result<(), AppError> {
    let target = PathBuf::from(path.trim());
    let Some(parent) = target.parent() else {
        return Err(AppError::user(
            "无法打开所在文件夹，路径无效。",
            format!("Missing parent for path: {}", target.display()),
        ));
    };
    if !parent.is_dir() {
        return Err(AppError::user(
            "无法打开所在文件夹，目录不存在。",
            format!("Missing directory: {}", parent.display()),
        ));
    }

    app.opener()
        .open_path(parent.to_string_lossy().to_string(), None::<&str>)
        .map_err(|error| AppError::user("打开文件夹失败。", error.to_string()))
}

async fn command_version(program: PathBuf, arg: &str) -> Option<String> {
    let output = Command::new(program)
        .arg(arg)
        .stdin(Stdio::null())
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    text.lines().next().map(|line| line.trim().to_string())
}

fn validate_url(url: &str) -> Result<&str, AppError> {
    let clean = url.trim();
    if clean.is_empty() {
        return Err(AppError::user("请输入视频链接。", "URL is empty"));
    }
    if !(clean.starts_with("http://") || clean.starts_with("https://")) {
        return Err(AppError::user(
            "链接格式不正确，请输入以 http:// 或 https:// 开头的地址。",
            format!("Invalid URL: {clean}"),
        ));
    }
    Ok(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare_only_marks_newer_releases() {
        assert!(version_is_newer("2026.06.09", "2026.02.21"));
        assert!(!version_is_newer("2026.06.09", "2026.06.09"));
        assert!(!version_is_newer("v2026.06.09", "2026.06.09"));
    }

    #[test]
    fn clean_bilibili_url_keeps_page_only() {
        let cleaned = clean_url_for_site(
            "https://www.bilibili.com/video/BV1R541157SQ?spm_id_from=333&p=39&vd_source=abc",
            ytdlp::SiteProfile::Bilibili,
        );

        assert_eq!(cleaned, "https://www.bilibili.com/video/BV1R541157SQ?p=39");
    }

    #[test]
    fn clean_youtube_url_keeps_video_identity() {
        let cleaned = clean_url_for_site(
            "https://www.youtube.com/watch?v=abc&t=30s&si=tracking",
            ytdlp::SiteProfile::Youtube,
        );

        assert_eq!(cleaned, "https://www.youtube.com/watch?v=abc&t=30s");
    }

    #[test]
    fn clean_twitter_url_removes_tracking_query() {
        let cleaned = clean_url_for_site(
            "https://x.com/example/status/123?s=20&t=tracking",
            ytdlp::SiteProfile::Twitter,
        );

        assert_eq!(cleaned, "https://x.com/example/status/123");
    }
}
