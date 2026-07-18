use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::errors::AppError;
use crate::subtitle_types::{GeminiSettings, SubtitleProject};

const SETTINGS_FILE: &str = "gemini-settings.json";

pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    app.path()
        .app_local_data_dir()
        .map_err(|error| AppError::user("无法定位应用数据目录。", error.to_string()))
}

pub fn projects_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app_data_dir(app)?.join("subtitle-projects"))
}

pub fn temp_dir(app: &AppHandle, project_id: &str) -> Result<PathBuf, AppError> {
    Ok(app_data_dir(app)?.join("subtitle-temp").join(project_id))
}

fn project_dir(app: &AppHandle, project_id: &str) -> Result<PathBuf, AppError> {
    Ok(projects_dir(app)?.join(project_id))
}

fn project_file(app: &AppHandle, project_id: &str) -> Result<PathBuf, AppError> {
    Ok(project_dir(app, project_id)?.join("project.json"))
}

pub fn read_project(app: &AppHandle, project_id: &str) -> Result<SubtitleProject, AppError> {
    let path = project_file(app, project_id)?;
    let bytes = fs::read(&path).map_err(|error| {
        AppError::user(
            "字幕项目不存在或无法读取。",
            format!("{}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(AppError::from)
}

pub fn write_project(app: &AppHandle, project: &SubtitleProject) -> Result<(), AppError> {
    let dir = project_dir(app, &project.id)?;
    fs::create_dir_all(&dir)?;
    let path = dir.join("project.json");
    let tmp = dir.join("project.json.tmp");
    let backup = dir.join("project.previous.json");
    let bytes = serde_json::to_vec_pretty(project)?;
    fs::write(&tmp, bytes)?;
    if path.is_file() {
        let _ = fs::copy(&path, &backup);
        fs::remove_file(&path)?;
    }
    fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn list_projects(app: &AppHandle) -> Result<Vec<SubtitleProject>, AppError> {
    let dir = projects_dir(app)?;
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut projects = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("project.json");
        if let Ok(bytes) = fs::read(&path) {
            if let Ok(project) = serde_json::from_slice::<SubtitleProject>(&bytes) {
                projects.push(project);
            }
        }
    }
    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(projects)
}

fn validate_project_id(project_id: &str) -> Result<(), AppError> {
    Uuid::parse_str(project_id)
        .map(|_| ())
        .map_err(|_| AppError::user("字幕项目 ID 无效。", project_id.to_string()))
}

pub fn delete_project(app: &AppHandle, project_id: &str) -> Result<bool, AppError> {
    validate_project_id(project_id)?;
    let path = project_dir(app, project_id)?;
    if !path.is_dir() {
        return Ok(false);
    }
    fs::remove_dir_all(path)?;
    clean_temp(app, project_id);
    Ok(true)
}

pub fn clear_projects(app: &AppHandle) -> Result<usize, AppError> {
    let projects = list_projects(app)?;
    let mut removed = 0;
    for project in projects {
        if delete_project(app, &project.id)? {
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn read_settings(app: &AppHandle) -> Result<GeminiSettings, AppError> {
    let path = app_data_dir(app)?.join(SETTINGS_FILE);
    if !path.is_file() {
        return Ok(GeminiSettings::default());
    }
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(AppError::from)
}

pub fn write_settings(app: &AppHandle, settings: &GeminiSettings) -> Result<(), AppError> {
    let dir = app_data_dir(app)?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(SETTINGS_FILE);
    let tmp = dir.join(format!("{SETTINGS_FILE}.tmp"));
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    if path.is_file() {
        fs::remove_file(&path)?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn fingerprint(path: &Path) -> Result<String, AppError> {
    let metadata = fs::metadata(path).map_err(|error| {
        AppError::user("无法读取媒体文件。", format!("{}: {error}", path.display()))
    })?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    Ok(format!("{}-{modified}", metadata.len()))
}

pub fn clean_temp(app: &AppHandle, project_id: &str) {
    if let Ok(path) = temp_dir(app, project_id) {
        let _ = fs::remove_dir_all(path);
    }
}

pub fn clean_all_temp(app: &AppHandle) {
    if let Ok(dir) = app_data_dir(app) {
        let path = dir.join("subtitle-temp");
        let mut cleanup_paths = Vec::new();
        if path.is_dir() {
            let cleanup = path.with_file_name(format!(
                "subtitle-temp.cleanup-{}-{}",
                now_epoch(),
                std::process::id()
            ));
            if fs::rename(&path, &cleanup).is_ok() {
                cleanup_paths.push(cleanup);
            }
        }
        if let Ok(entries) = fs::read_dir(dir) {
            cleanup_paths.extend(entries.flatten().filter_map(|entry| {
                let name = entry.file_name();
                name.to_string_lossy()
                    .starts_with("subtitle-temp.cleanup-")
                    .then(|| entry.path())
            }));
        }
        cleanup_paths.sort();
        cleanup_paths.dedup();
        if !cleanup_paths.is_empty() {
            std::thread::spawn(move || {
                for cleanup in cleanup_paths {
                    let _ = fs::remove_dir_all(cleanup);
                }
            });
        }
    }
}

pub fn clean_all_temp_now(app: &AppHandle) -> Result<(), AppError> {
    let path = app_data_dir(app)?.join("subtitle-temp");
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub fn recover_interrupted_projects(app: &AppHandle) {
    for mut project in list_projects(app).unwrap_or_default() {
        if !matches!(
            project.status.as_str(),
            "processing" | "translate" | "polish" | "exporting" | "burning"
        ) {
            continue;
        }
        project.status = "paused".into();
        project.last_error = Some("应用上次退出时任务尚未完成，可重新开始以继续处理。".into());
        project.updated_at = now_epoch();
        if let Some(stage) = project
            .performance
            .stages
            .iter_mut()
            .rev()
            .find(|stage| stage.status == "running")
        {
            let finished = now_epoch_ms();
            stage.finished_at_ms = Some(finished);
            stage.duration_ms = Some(finished.saturating_sub(stage.started_at_ms));
            stage.status = "interrupted".into();
            stage.detail = Some("应用退出，任务已转为可恢复状态。".into());
        }
        let _ = write_project(app, &project);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_project_path_traversal() {
        assert!(validate_project_id("..\\subtitle-projects").is_err());
        assert!(validate_project_id("bd4ba448-9390-4e53-9cb1-177b2e261473").is_ok());
    }
}
