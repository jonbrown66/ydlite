use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::commands::{CookieSource, ParseOptions};
use crate::tool_paths;
use crate::tool_probe::FileIdentity;

const MAX_METADATA_BYTES: usize = 4 * 1024 * 1024;
const MAX_AGE: Duration = Duration::from_secs(120);

#[derive(PartialEq, Eq)]
struct CacheKey {
    url: String,
    tool: FileIdentity,
    cookies: Option<FileIdentity>,
}

impl CacheKey {
    fn read(url: &str, options: &ParseOptions) -> Option<Self> {
        let cookies = match &options.cookie_source {
            CookieSource::None => None,
            CookieSource::File { path } => {
                Some(FileIdentity::read(std::path::Path::new(path.trim()))?)
            }
            // Browser login changes are not observable through a file fingerprint.
            CookieSource::Browser { .. } => return None,
        };
        Some(Self {
            url: url.trim().into(),
            tool: FileIdentity::read_executable(&tool_paths::ytdlp())?,
            cookies,
        })
    }
}

struct CachedMetadata {
    key: CacheKey,
    created_at: Instant,
    json: Vec<u8>,
}

impl CachedMetadata {
    fn into_json(self, key: &CacheKey) -> Option<Vec<u8>> {
        (self.key == *key && self.created_at.elapsed() < MAX_AGE).then_some(self.json)
    }
}

fn cache() -> &'static Mutex<Option<CachedMetadata>> {
    static CACHE: OnceLock<Mutex<Option<CachedMetadata>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn has_downloadable_formats(value: &Value) -> bool {
    if value.get("entries").is_some()
        || !matches!(
            value.get("_type").and_then(Value::as_str),
            None | Some("video")
        )
    {
        return false;
    }
    value
        .get("formats")
        .and_then(Value::as_array)
        .is_some_and(|formats| {
            !formats.is_empty()
                && formats.iter().any(|format| {
                    let id = format
                        .get("format_id")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let url = format.get("url").and_then(Value::as_str).unwrap_or("");
                    let media = ["vcodec", "acodec"].iter().any(|key| {
                        format
                            .get(*key)
                            .and_then(Value::as_str)
                            .is_some_and(|codec| !codec.is_empty() && codec != "none")
                    });
                    // Generic HTML media may have no codec information even after
                    // full extraction. Accept its processed direct-media record,
                    // while a bare URL/format stub still requires deep extraction.
                    let direct_generic = matches!(
                        value.get("extractor").and_then(Value::as_str),
                        Some("generic" | "html5")
                    ) && ["id", "title"].iter().all(|key| {
                        value
                            .get(*key)
                            .and_then(Value::as_str)
                            .is_some_and(|text| !text.is_empty())
                    }) && format.get("vcodec").is_none_or(Value::is_null)
                        && format.get("acodec").is_none_or(Value::is_null)
                        && matches!(
                            format.get("protocol").and_then(Value::as_str),
                            Some("http" | "https")
                        )
                        && matches!(
                            format.get("ext").and_then(Value::as_str),
                            Some(
                                "mp4"
                                    | "webm"
                                    | "mkv"
                                    | "mov"
                                    | "mp3"
                                    | "m4a"
                                    | "aac"
                                    | "ogg"
                                    | "opus"
                                    | "flac"
                                    | "wav"
                            )
                        );
                    !id.is_empty()
                        && (media || direct_generic)
                        && (url.starts_with("https://") || url.starts_with("http://"))
                })
        })
}

fn cacheable(value: &Value) -> bool {
    has_downloadable_formats(value)
        && value.get("is_live").and_then(Value::as_bool) != Some(true)
        && !matches!(
            value.get("live_status").and_then(Value::as_str),
            Some("is_live" | "post_live" | "is_upcoming" | "was_live")
        )
}

pub(crate) fn remember(url: &str, options: &ParseOptions, value: &Value) {
    let next = (|| {
        if !cacheable(value) {
            return None;
        }
        let key = CacheKey::read(url, options)?;
        let mut value = value.clone();
        // yt-dlp can reselect quality and filename using the current request.
        let object = value.as_object_mut()?;
        object.insert("webpage_url".into(), Value::String(url.into()));
        for key in [
            "requested_downloads",
            "requested_formats",
            "filepath",
            "_filename",
            "filename",
        ] {
            object.remove(key);
        }
        let json = serde_json::to_vec(&value).ok()?;
        if json.len() > MAX_METADATA_BYTES {
            return None;
        }
        Some(CachedMetadata {
            key,
            created_at: Instant::now(),
            json,
        })
    })();
    if let Ok(mut cached) = cache().lock() {
        *cached = next;
    }
}

pub(crate) fn take(url: &str, options: &ParseOptions) -> Option<Vec<u8>> {
    let key = CacheKey::read(url, options)?;
    let value = cache().lock().ok()?.take()?;
    value.into_json(&key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    #[ignore = "requires yt-dlp installed on PATH"]
    fn path_installed_tool_supports_metadata_reuse() {
        assert!(
            FileIdentity::read_executable(std::path::Path::new(if cfg!(windows) {
                "yt-dlp.exe"
            } else {
                "yt-dlp"
            }))
            .is_some()
        );
        let url = "https://example.com/path-cache-fixture";
        let options = ParseOptions::default();
        let value = json!({"id":"fixture", "title":"Fixture", "formats":[{"format_id":"720","url":"https://example.com/media.mp4","vcodec":"avc1"}]});
        remember(url, &options, &value);
        assert!(
            take(url, &options).is_some(),
            "PATH tool must allow the actual remember/take pipeline"
        );
    }

    #[test]
    fn metadata_is_only_reused_for_the_same_request_before_expiry() {
        let tool = FileIdentity::read(&std::env::current_exe().unwrap()).unwrap();
        let make = || CachedMetadata {
            key: CacheKey {
                url: "https://example.com/a".into(),
                tool: tool.clone(),
                cookies: None,
            },
            created_at: Instant::now(),
            json: b"{}".to_vec(),
        };
        let mut key = CacheKey {
            url: "https://example.com/a".into(),
            tool: tool.clone(),
            cookies: None,
        };
        assert!(make().into_json(&key).is_some());
        let mut expired = make();
        expired.created_at -= MAX_AGE + Duration::from_secs(1);
        assert!(expired.into_json(&key).is_none());
        key.url = "https://example.com/b".into();
        assert!(make().into_json(&key).is_none());
        key.url = "https://example.com/a".into();
        key.cookies = Some(key.tool.clone());
        assert!(make().into_json(&key).is_none());
    }

    #[test]
    fn flat_or_incomplete_records_still_need_deep_extraction() {
        assert!(!has_downloadable_formats(
            &json!({"_type":"url", "url":"https://example.com/video"})
        ));
        assert!(!has_downloadable_formats(&json!({"formats":[]})));
        assert!(!has_downloadable_formats(
            &json!({"formats":[{"format_id":"720"}]})
        ));
        assert!(!has_downloadable_formats(
            &json!({"formats":[{"format_id":"720","url":"https://example.com/video"}]})
        ));
        assert!(!has_downloadable_formats(
            &json!({"entries":[], "formats":[{"format_id":"720","url":"https://example.com/video"}]})
        ));
        assert!(has_downloadable_formats(
            &json!({"formats":[{"format_id":"720","url":"https://example.com/video","vcodec":"avc1","acodec":"none"}]})
        ));
        assert!(!has_downloadable_formats(
            &json!({"formats":[{"format_id":"images","url":"https://example.com/images","vcodec":"none","acodec":"none"}]})
        ));
    }

    #[test]
    fn processed_generic_media_can_be_complete_without_codec_metadata() {
        let mut value = json!({"_type":"video", "extractor":"generic", "id":"fixture", "title":"Fixture",
            "formats":[{"format_id":"0", "url":"https://example.com/media.mp4", "ext":"mp4", "protocol":"https"}]});
        assert!(has_downloadable_formats(&value));
        value["extractor"] = json!("html5");
        assert!(has_downloadable_formats(&value));
        value["extractor"] = json!("incomplete");
        assert!(!has_downloadable_formats(&value));
        value["extractor"] = json!("html5");
        value["formats"][0]["protocol"] = json!("mhtml");
        assert!(!has_downloadable_formats(&value));
        value["formats"][0]["protocol"] = json!("https");
        value["title"] = json!(null);
        assert!(!has_downloadable_formats(&value));
    }

    #[test]
    fn live_metadata_must_not_be_reused_for_downloading() {
        let mut value = json!({"formats":[{"format_id":"720","url":"https://example.com/video","vcodec":"avc1"}]});
        assert!(cacheable(&value));
        for status in ["is_live", "post_live", "is_upcoming", "was_live"] {
            value["live_status"] = json!(status);
            assert!(!cacheable(&value));
        }
        value["live_status"] = json!("not_live");
        value["is_live"] = json!(true);
        assert!(!cacheable(&value));
    }
}
