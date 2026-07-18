use std::fs;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

use crate::errors::AppError;
use crate::subtitle_types::{WhisperModelInfo, WhisperRuntimeInfo};

const WHISPER_VERSION: &str = "1.9.1";

struct ModelSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    file_name: &'static str,
    url: &'static str,
    bytes: u64,
    size: &'static str,
}

struct RuntimeSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    url: &'static str,
    bytes: u64,
    size: &'static str,
}

const MODELS: &[ModelSpec] = &[
    ModelSpec {
        id: "small-q5",
        name: "Whisper Small Q5",
        description: "下载更快、占用更低，适合普通电脑和快速处理。",
        file_name: "ggml-small-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small-q5_1.bin",
        bytes: 190_085_487,
        size: "约 181 MB",
    },
    ModelSpec {
        id: "large-v3-turbo-q5",
        name: "Whisper Large V3 Turbo Q5",
        description: "多语言高质量模式，推荐日语、韩语、英语和中英混合视频。",
        file_name: "ggml-large-v3-turbo-q5_0.bin",
        url:
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-large-v3-turbo-q5_0.bin",
        bytes: 574_041_195,
        size: "约 548 MB",
    },
];

const RUNTIMES: &[RuntimeSpec] = &[
    RuntimeSpec {
        id: "cpu",
        name: "CPU 兼容运行组件",
        description: "体积小、兼容性最好，不需要 NVIDIA 显卡。",
        url: "https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.1/whisper-bin-x64.zip",
        bytes: 7_982_101,
        size: "约 7.6 MB",
    },
    RuntimeSpec {
        id: "cuda",
        name: "NVIDIA CUDA 加速组件",
        description: "适合 NVIDIA 显卡，包含完整 CUDA 12.4 运行库，下载较大但速度更快。",
        url: "https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.1/whisper-cublas-12.4.0-bin-x64.zip",
        bytes: 677_887_125,
        size: "约 647 MB",
    },
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperDownloadEvent {
    pub asset_type: String,
    pub id: String,
    pub status: String,
    pub percent: f32,
    pub message: String,
}

fn root(app: &AppHandle) -> Result<PathBuf, AppError> {
    app.path()
        .app_local_data_dir()
        .map(|path| path.join("whisper"))
        .map_err(|error| AppError::user("无法定位本地模型目录。", error.to_string()))
}

fn model_spec(id: &str) -> Result<&'static ModelSpec, AppError> {
    MODELS
        .iter()
        .find(|model| model.id == id)
        .ok_or_else(|| AppError::user("不支持该 Whisper 模型。", id))
}

pub fn is_supported_model_id(id: &str) -> bool {
    MODELS.iter().any(|model| model.id == id)
}

fn runtime_spec(id: &str) -> Result<&'static RuntimeSpec, AppError> {
    RUNTIMES
        .iter()
        .find(|runtime| runtime.id == id)
        .ok_or_else(|| AppError::user("不支持该 Whisper 运行组件。", id))
}

pub fn model_path(app: &AppHandle, id: &str) -> Result<PathBuf, AppError> {
    let spec = model_spec(id)?;
    Ok(root(app)?.join("models").join(spec.file_name))
}

pub fn runtime_dir(app: &AppHandle, id: &str) -> Result<PathBuf, AppError> {
    runtime_spec(id)?;
    Ok(root(app)?.join("runtime").join(id))
}

pub fn executable_path(app: &AppHandle, runtime: &str) -> Result<PathBuf, AppError> {
    let path = runtime_dir(app, runtime)?.join("whisper-cli.exe");
    if !runtime_is_installed(app, runtime)? {
        return Err(AppError::user(
            "尚未安装所选 Whisper 运行组件。",
            runtime.to_string(),
        ));
    }
    Ok(path)
}

pub fn ensure_model(app: &AppHandle, id: &str) -> Result<PathBuf, AppError> {
    let path = model_path(app, id)?;
    if !path.is_file() {
        return Err(AppError::user(
            "尚未下载所选 Whisper 模型。",
            format!("请先在设置中下载 {id}"),
        ));
    }
    Ok(path)
}

#[tauri::command]
pub fn list_whisper_models(app: AppHandle) -> Result<Vec<WhisperModelInfo>, AppError> {
    MODELS
        .iter()
        .map(|spec| {
            let path = model_path(&app, spec.id)?;
            let installed = path
                .metadata()
                .map(|metadata| metadata.len() >= spec.bytes.saturating_sub(1024))
                .unwrap_or(false);
            Ok(WhisperModelInfo {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                description: spec.description.to_string(),
                download_bytes: spec.bytes,
                download_size: spec.size.to_string(),
                installed,
                path: installed.then(|| path.to_string_lossy().to_string()),
            })
        })
        .collect()
}

#[tauri::command]
pub fn list_whisper_runtimes(app: AppHandle) -> Result<Vec<WhisperRuntimeInfo>, AppError> {
    RUNTIMES
        .iter()
        .map(|spec| {
            let installed = runtime_is_installed(&app, spec.id)?;
            Ok(WhisperRuntimeInfo {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                description: spec.description.to_string(),
                download_bytes: spec.bytes,
                download_size: spec.size.to_string(),
                installed,
            })
        })
        .collect()
}

#[tauri::command]
pub async fn download_whisper_model(
    app: AppHandle,
    request: crate::subtitle_types::WhisperAssetRequest,
) -> Result<(), AppError> {
    let spec = model_spec(&request.id)?;
    let destination = model_path(&app, spec.id)?;
    download(&app, "model", spec.id, spec.url, spec.bytes, &destination).await
}

#[tauri::command]
pub async fn download_whisper_runtime(
    app: AppHandle,
    request: crate::subtitle_types::WhisperAssetRequest,
) -> Result<(), AppError> {
    let spec = runtime_spec(&request.id)?;
    let dir = runtime_dir(&app, spec.id)?;
    let staging = dir.with_extension("installing");
    if staging.is_dir() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    let archive = root(&app)?.join(format!("runtime-{}-{WHISPER_VERSION}.zip", spec.id));
    download(&app, "runtime", spec.id, spec.url, spec.bytes, &archive).await?;
    emit(
        &app,
        "runtime",
        spec.id,
        "extracting",
        100.0,
        "正在安装运行组件…",
    );
    let archive_copy = archive.clone();
    let staging_copy = staging.clone();
    tokio::task::spawn_blocking(move || extract_runtime(&archive_copy, &staging_copy))
        .await
        .map_err(|error| AppError::user("Whisper 运行组件安装失败。", error.to_string()))??;
    if dir.is_dir() {
        fs::remove_dir_all(&dir)?;
    }
    fs::rename(&staging, &dir)?;
    let _ = fs::remove_file(archive);
    emit(
        &app,
        "runtime",
        spec.id,
        "installed",
        100.0,
        "运行组件安装完成",
    );
    Ok(())
}

fn runtime_is_installed(app: &AppHandle, id: &str) -> Result<bool, AppError> {
    let dir = runtime_dir(app, id)?;
    let common = [
        "whisper-cli.exe",
        "whisper.dll",
        "ggml.dll",
        "ggml-base.dll",
    ]
    .iter()
    .all(|name| dir.join(name).is_file());
    if !common {
        return Ok(false);
    }
    Ok(match id {
        "cuda" => ["ggml-cuda.dll", "cublas64_12.dll", "cublasLt64_12.dll"]
            .iter()
            .all(|name| dir.join(name).is_file()),
        _ => fs::read_dir(&dir)?
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().starts_with("ggml-cpu-")),
    })
}

#[tauri::command]
pub fn delete_whisper_model(
    app: AppHandle,
    request: crate::subtitle_types::WhisperAssetRequest,
) -> Result<(), AppError> {
    let path = model_path(&app, &request.id)?;
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

async fn download(
    app: &AppHandle,
    asset_type: &str,
    id: &str,
    url: &str,
    expected_bytes: u64,
    destination: &Path,
) -> Result<(), AppError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let part = destination.with_extension("download");
    let existing = part.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    emit(
        app,
        asset_type,
        id,
        "downloading",
        existing as f32 / expected_bytes.max(1) as f32 * 100.0,
        if existing > 0 {
            "正在继续上次下载"
        } else {
            "开始下载"
        },
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()
        .map_err(|error| AppError::user("无法初始化模型下载。", error.to_string()))?;
    let mut request = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "YDLite");
    if existing > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={existing}-"));
    }
    let response = request
        .send()
        .await
        .map_err(|error| AppError::user("下载失败，请检查网络。", error.to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::user(
            "下载失败，请稍后重试。",
            format!("HTTP {}", response.status()),
        ));
    }
    let resumed = response.status() == reqwest::StatusCode::PARTIAL_CONTENT && existing > 0;
    let start = if resumed { existing } else { 0 };
    let total = response
        .content_length()
        .map(|length| length + start)
        .unwrap_or(expected_bytes);
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&part)
        .await?;
    let mut downloaded = start;
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|error| AppError::user("下载中断，请重试。", error.to_string()))?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            emit(
                app,
                asset_type,
                id,
                "downloading",
                downloaded as f32 / total as f32 * 100.0,
                "下载中",
            );
        }
    }
    file.flush().await?;
    drop(file);
    if downloaded < expected_bytes.saturating_sub(1024) {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(AppError::user(
            "下载文件不完整，请重试。",
            format!("Expected {expected_bytes}, received {downloaded}"),
        ));
    }
    if destination.is_file() {
        fs::remove_file(destination)?;
    }
    fs::rename(part, destination)?;
    emit(app, asset_type, id, "installed", 100.0, "下载完成");
    Ok(())
}

fn extract_runtime(archive: &Path, destination: &Path) -> Result<(), AppError> {
    let archive_file = fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(archive_file)
        .map_err(|error| AppError::user("Whisper 运行组件解压失败。", error.to_string()))?;
    fs::create_dir_all(destination)?;
    let mut found_cli = false;
    for index in 0..zip.len() {
        let mut file = zip
            .by_index(index)
            .map_err(|error| AppError::user("Whisper 运行组件解压失败。", error.to_string()))?;
        if file.is_dir() {
            continue;
        }
        let Some(name) = Path::new(file.name()).file_name() else {
            continue;
        };
        let name = name.to_string_lossy();
        if !(name.ends_with(".exe") || name.ends_with(".dll")) {
            continue;
        }
        if name == "whisper-cli.exe" {
            found_cli = true;
        }
        let mut output = fs::File::create(destination.join(name.as_ref()))?;
        std::io::copy(&mut file, &mut output)?;
    }
    if !found_cli {
        return Err(AppError::user(
            "Whisper 运行组件中缺少 whisper-cli.exe。",
            archive.display().to_string(),
        ));
    }
    Ok(())
}

fn emit(
    app: &AppHandle,
    asset_type: &str,
    id: &str,
    status: &str,
    percent: f32,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "whisper://download-progress",
        WhisperDownloadEvent {
            asset_type: asset_type.to_string(),
            id: id.to_string(),
            status: status.to_string(),
            percent,
            message: message.into(),
        },
    );
}
