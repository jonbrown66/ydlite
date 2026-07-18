use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::errors::AppError;
use crate::subtitle_commands::SubtitleTaskState;
use crate::subtitle_store;

const HISTORY_FILE: &str = "cleanup-history.json";
const HISTORY_LIMIT: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRecord {
    pub id: String,
    pub cleaned_at: u64,
    pub released_bytes: u64,
    pub temporary_bytes: u64,
    pub webview_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    pub cache_bytes: u64,
    pub temporary_bytes: u64,
    pub webview_bytes: u64,
    pub model_bytes: u64,
    pub project_bytes: u64,
    pub records: Vec<CleanupRecord>,
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    app.path()
        .app_local_data_dir()
        .map_err(|error| AppError::user("无法定位应用缓存目录。", error.to_string()))
}

fn directory_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => directory_size(&path),
                Ok(kind) if kind.is_file() => {
                    entry.metadata().map(|value| value.len()).unwrap_or(0)
                }
                _ => 0,
            }
        })
        .sum()
}

fn read_records(dir: &Path) -> Vec<CleanupRecord> {
    fs::read(dir.join(HISTORY_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_records(dir: &Path, records: &[CleanupRecord]) -> Result<(), AppError> {
    fs::create_dir_all(dir)?;
    let path = dir.join(HISTORY_FILE);
    let temporary = dir.join(format!("{HISTORY_FILE}.tmp"));
    fs::write(&temporary, serde_json::to_vec_pretty(records)?)?;
    if path.is_file() {
        fs::remove_file(&path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

fn status(app: &AppHandle) -> Result<CacheStatus, AppError> {
    let dir = data_dir(app)?;
    let temporary_bytes = directory_size(&dir.join("subtitle-temp"));
    let webview_bytes = directory_size(&dir.join("EBWebView"));
    Ok(CacheStatus {
        cache_bytes: temporary_bytes + webview_bytes,
        temporary_bytes,
        webview_bytes,
        model_bytes: directory_size(&dir.join("whisper")),
        project_bytes: directory_size(&dir.join("subtitle-projects")),
        records: read_records(&dir),
    })
}

#[tauri::command]
pub fn get_cache_status(app: AppHandle) -> Result<CacheStatus, AppError> {
    status(&app)
}

#[tauri::command]
pub fn clear_app_cache(
    app: AppHandle,
    tasks: State<'_, SubtitleTaskState>,
) -> Result<CacheStatus, AppError> {
    if tasks.has_active_tasks() {
        return Err(AppError::user(
            "字幕任务运行期间不能清理缓存。",
            "Cancel or finish the active task first",
        ));
    }
    let before = status(&app)?;
    subtitle_store::clean_all_temp_now(&app)?;
    if let Some(window) = app.get_webview_window("main") {
        window
            .clear_all_browsing_data()
            .map_err(|error| AppError::user("界面缓存清理失败。", error.to_string()))?;
    }
    let dir = data_dir(&app)?;
    let mut after = status(&app)?;
    let record = CleanupRecord {
        id: Uuid::new_v4().to_string(),
        cleaned_at: subtitle_store::now_epoch(),
        released_bytes: before.cache_bytes.saturating_sub(after.cache_bytes),
        temporary_bytes: before.temporary_bytes,
        webview_bytes: before.webview_bytes,
    };
    let mut records = read_records(&dir);
    records.insert(0, record);
    records.truncate(HISTORY_LIMIT);
    write_records(&dir, &records)?;
    after.records = records;
    Ok(after)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_directory_has_zero_size() {
        assert_eq!(
            directory_size(Path::new("Z:\\ydlite-cache-path-that-does-not-exist")),
            0
        );
    }
}
