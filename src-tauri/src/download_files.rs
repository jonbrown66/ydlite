use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::AppError;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadOptions {
    pub naming: Naming,
    pub conflict: Conflict,
    pub quality: Quality,
    pub audio_format_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Naming {
    #[default]
    Title,
    AuthorTitle,
    DateTitle,
}
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Conflict {
    #[default]
    Skip,
    Number,
    Overwrite,
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Quality {
    #[default]
    Compatible,
    Quality,
}

pub struct OutputPlan {
    pub template: String,
    pub existing: Option<PathBuf>,
}

// Include media identity and requested streams, so unrelated titles and different
// output formats never share a destination. No URL or credential is persisted.
fn fingerprint(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn safe_name(value: &str) -> String {
    let mut clean: String = value
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*%".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    while clean.len() > 150 {
        clean.pop();
    }
    let clean = clean.trim().trim_end_matches(['.', ' ']);
    // Prefixing also avoids Windows reserved device names such as CON and NUL.
    format!("{}", if clean.is_empty() { "Media" } else { clean })
}

pub fn plan_output(
    dir: &Path,
    info: &Value,
    profile: &str,
    options: &DownloadOptions,
) -> Result<OutputPlan, AppError> {
    let title = info["title"].as_str().unwrap_or("Media");
    let extractor = info["extractor_key"]
        .as_str()
        .or(info["extractor"].as_str())
        .unwrap_or("generic");
    let generic_source = if ["generic", "html5"]
        .iter()
        .any(|name| extractor.to_ascii_lowercase().contains(name))
    {
        info["webpage_url"]
            .as_str()
            .or(info["original_url"].as_str())
            .unwrap_or("")
    } else {
        ""
    };
    let identity = format!(
        "{}:{}:{}",
        extractor,
        info["id"].as_str().unwrap_or(title),
        generic_source
    );
    let tag = format!(
        "[{}-{}]",
        fingerprint(&identity),
        &fingerprint(profile)[..8]
    );
    let display = match options.naming {
        Naming::Title => title.to_string(),
        Naming::AuthorTitle => format!(
            "{} - {title}",
            info["uploader"].as_str().unwrap_or("Unknown author")
        ),
        Naming::DateTitle => format!(
            "{} - {title}",
            info["upload_date"].as_str().unwrap_or("Unknown date")
        ),
    };
    let base = format!("{} {tag}", safe_name(&display));
    let mut matches = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() || entry.metadata()?.len() == 0 {
            continue;
        }
        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ![
            "mp4", "mkv", "webm", "mov", "avi", "flv", "mp3", "m4a", "ogg", "opus", "wav", "aac",
            "ts", "m4v",
        ]
        .contains(&extension.as_str())
        {
            continue;
        }
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let identity_stem = stem
            .rsplit_once(" (")
            .filter(|(_, suffix)| {
                suffix
                    .strip_suffix(')')
                    .is_some_and(|n| n.parse::<u32>().is_ok())
            })
            .map(|(stem, _)| stem)
            .unwrap_or(&stem);
        if identity_stem.ends_with(&tag) {
            matches.push(path);
        }
    }
    // The original is preferred to numbered copies, irrespective of lexical order.
    matches.sort_by_key(|path| {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let number = stem
            .rsplit_once(" (")
            .and_then(|(_, suffix)| suffix.strip_suffix(')'))
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(1);
        (number, path.clone())
    });
    let mut stem = base.clone();
    let existing = matches.first().cloned();
    match options.conflict {
        Conflict::Skip => {}
        Conflict::Overwrite => {
            if let Some(path) = &existing {
                stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
            }
        }
        Conflict::Number => {
            let mut number = 2;
            while matches
                .iter()
                .any(|p| p.file_stem().unwrap_or_default().to_string_lossy() == stem)
            {
                stem = format!("{base} ({number})");
                number += 1;
            }
        }
    }
    Ok(OutputPlan {
        template: format!("{}.%(ext)s", stem.replace('%', "%%")),
        existing: if options.conflict == Conflict::Skip {
            existing
        } else {
            None
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generic_media_ids_are_scoped_to_the_source_and_percent_names_are_literal() {
        let dir = std::env::temp_dir().join(format!("ydlite-generic-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let mut info = serde_json::json!({"id":"video","title":"Same title","extractor_key":"Generic","webpage_url":"https://one.example/video.mp4"});
        let mut options = DownloadOptions::default();
        let first = plan_output(&dir, &info, "best", &options).unwrap();
        let filename = first.template.replace("%(ext)s", "mp4");
        std::fs::write(dir.join(&filename), b"complete").unwrap();
        info["webpage_url"] = "https://two.example/video.mp4".into();
        assert!(plan_output(&dir, &info, "best", &options)
            .unwrap()
            .existing
            .is_none());
        info["webpage_url"] = "https://one.example/video.mp4".into();
        let renamed = filename.replace("Same title", "100% title");
        std::fs::rename(dir.join(filename), dir.join(&renamed)).unwrap();
        options.conflict = Conflict::Overwrite;
        let replacement = plan_output(&dir, &info, "best", &options).unwrap();
        assert!(replacement.template.contains("100%% title"));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn names_preserve_unicode_and_do_not_collide_or_destroy_partial_files() {
        let dir = std::env::temp_dir().join(format!("ydlite-names-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let info = serde_json::json!({"id":"first","title":"中文标题 / CON: 测试","uploader":"作者","upload_date":"20261012","extractor_key":"test"});
        let mut options = DownloadOptions::default();
        let plan = plan_output(&dir, &info, "best", &options).unwrap();
        assert!(plan.template.contains("中文标题"));
        assert!(!plan.template.contains('/'));
        let path = dir.join(plan.template.replace("%(ext)s", "mp4"));
        std::fs::write(&path, b"original").unwrap();
        assert_eq!(
            plan_output(&dir, &info, "best", &options).unwrap().existing,
            Some(path.clone())
        );
        options.naming = Naming::AuthorTitle;
        assert_eq!(
            plan_output(&dir, &info, "best", &options).unwrap().existing,
            Some(path.clone()),
            "renaming presets do not defeat duplicate detection"
        );
        options.naming = Naming::Title;
        let mut other = info.clone();
        other["id"] = "second".into();
        assert!(plan_output(&dir, &other, "best", &options)
            .unwrap()
            .existing
            .is_none());
        assert!(plan_output(&dir, &info, "mp3", &options)
            .unwrap()
            .existing
            .is_none());
        options.conflict = Conflict::Number;
        let numbered = plan_output(&dir, &info, "best", &options).unwrap();
        assert!(numbered.template.contains(" (2)."));
        std::fs::write(
            dir.join(numbered.template.replace("%(ext)s", "mp4.part")),
            b"partial",
        )
        .unwrap();
        assert_eq!(
            plan_output(&dir, &info, "best", &options).unwrap().template,
            numbered.template
        );
        std::fs::write(
            dir.join(numbered.template.replace("%(ext)s", "mp4")),
            b"numbered copy",
        )
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        options.conflict = Conflict::Overwrite;
        assert_eq!(
            plan_output(&dir, &info, "best", &options).unwrap().template,
            plan.template
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
