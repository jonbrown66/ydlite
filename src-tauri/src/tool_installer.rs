use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::Path;

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

use crate::errors::AppError;
use crate::tool_paths;

const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInstallEvent {
    pub tool: String,
    pub status: String,
    pub percent: Option<f32>,
    pub message: Option<String>,
}

#[tauri::command]
pub fn get_tools_directory() -> String {
    tool_paths::tools_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub async fn install_missing_tools(app: AppHandle) -> Result<(), AppError> {
    fs::create_dir_all(tool_paths::tools_dir())?;

    if !command_available(tool_paths::ytdlp(), "--version").await {
        download_file(&app, "yt-dlp", YTDLP_URL, &tool_paths::ytdlp_install_path()).await?;
        emit(&app, "yt-dlp", "installed", Some(100.0), "yt-dlp 安装完成");
    } else {
        emit(&app, "yt-dlp", "installed", Some(100.0), "yt-dlp 已安装");
    }

    if !command_available(tool_paths::ffmpeg(), "-version").await {
        let archive = tool_paths::tools_dir().join("ffmpeg.zip");
        download_file(&app, "ffmpeg", FFMPEG_URL, &archive).await?;
        emit(&app, "ffmpeg", "extracting", None, "正在解压 ffmpeg");
        extract_ffmpeg(&archive, &tool_paths::ffmpeg_install_path())?;
        let _ = fs::remove_file(&archive);
        emit(&app, "ffmpeg", "installed", Some(100.0), "ffmpeg 安装完成");
    } else {
        emit(&app, "ffmpeg", "installed", Some(100.0), "ffmpeg 已安装");
    }

    Ok(())
}

#[tauri::command]
pub async fn update_ytdlp(app: AppHandle) -> Result<(), AppError> {
    fs::create_dir_all(tool_paths::tools_dir())?;
    download_file(&app, "yt-dlp", YTDLP_URL, &tool_paths::ytdlp_install_path()).await?;
    emit(&app, "yt-dlp", "installed", Some(100.0), "yt-dlp 已更新");
    Ok(())
}

async fn command_available(program: std::path::PathBuf, version_arg: &str) -> bool {
    Command::new(program)
        .arg(version_arg)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

async fn download_file(
    app: &AppHandle,
    tool: &str,
    url: &str,
    destination: &Path,
) -> Result<(), AppError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    emit(app, tool, "downloading", Some(0.0), "开始下载");

    let response = reqwest::get(url).await.map_err(|error| {
        AppError::user(format!("{tool} 下载失败，请检查网络。"), error.to_string())
    })?;
    if !response.status().is_success() {
        return Err(AppError::user(
            format!("{tool} 下载失败，请稍后重试。"),
            format!("HTTP status: {}", response.status()),
        ));
    }

    let total = response.content_length();
    let mut downloaded = 0_u64;
    let mut file = File::create(destination)?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            AppError::user(format!("{tool} 下载中断，请重试。"), error.to_string())
        })?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        if let Some(total) = total {
            if total > 0 {
                let percent = downloaded as f32 / total as f32 * 100.0;
                emit(app, tool, "downloading", Some(percent), "下载中");
            }
        }
    }

    Ok(())
}

fn extract_ffmpeg(archive: &Path, destination: &Path) -> Result<(), AppError> {
    let mut bytes = Vec::new();
    File::open(archive)?.read_to_end(&mut bytes)?;
    let cursor = Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|error| AppError::user("ffmpeg 解压失败。", error.to_string()))?;

    for index in 0..zip.len() {
        let mut file = zip
            .by_index(index)
            .map_err(|error| AppError::user("ffmpeg 解压失败。", error.to_string()))?;
        let name = file.name().replace('\\', "/");
        if name.ends_with("/bin/ffmpeg.exe") || name == "ffmpeg.exe" {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = File::create(destination)?;
            std::io::copy(&mut file, &mut out)?;
            return Ok(());
        }
    }

    Err(AppError::user(
        "ffmpeg 解压失败，压缩包中未找到 ffmpeg.exe。",
        format!("Archive: {}", archive.display()),
    ))
}

fn emit(
    app: &AppHandle,
    tool: &str,
    status: &str,
    percent: Option<f32>,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "tools://install-progress",
        ToolInstallEvent {
            tool: tool.to_string(),
            status: status.to_string(),
            percent,
            message: Some(message.into()),
        },
    );
}
