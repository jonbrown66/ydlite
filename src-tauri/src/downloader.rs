use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::commands::DownloadRequest;
use crate::errors::AppError;
use crate::progress::{parse_output_path, parse_progress_line, DownloadProgressEvent};
use crate::tool_paths;
use crate::ytdlp;

#[derive(Clone, Default)]
pub struct DownloadState {
    inner: Arc<Mutex<InnerState>>,
}

#[derive(Default)]
struct InnerState {
    active: bool,
    child_id: Option<u32>,
    cancel_requested: bool,
}

impl DownloadState {
    pub async fn reserve(&self) -> Result<(), AppError> {
        let mut state = self.inner.lock().await;
        if state.active {
            return Err(AppError::user(
                "当前已有下载任务进行中，请等待完成或先取消。",
                "Download already active",
            ));
        }
        state.active = true;
        state.child_id = None;
        state.cancel_requested = false;
        Ok(())
    }

    pub async fn set_child_id(&self, child_id: u32) {
        let mut state = self.inner.lock().await;
        if state.active {
            state.child_id = Some(child_id);
        }
    }

    pub async fn clear(&self) {
        let mut state = self.inner.lock().await;
        state.active = false;
        state.child_id = None;
        state.cancel_requested = false;
    }

    pub async fn finish(&self, child_id: Option<u32>) -> bool {
        let mut state = self.inner.lock().await;
        let was_cancelled = state.cancel_requested;
        if child_id.is_none() || state.child_id == child_id {
            state.active = false;
            state.child_id = None;
            state.cancel_requested = false;
        }
        was_cancelled
    }

    pub async fn cancel_current(&self) -> Result<(), AppError> {
        let child_id = {
            let mut state = self.inner.lock().await;
            state.cancel_requested = true;
            state.child_id
        };

        let Some(child_id) = child_id else {
            self.clear().await;
            return Ok(());
        };

        #[cfg(target_os = "windows")]
        let status = Command::new("taskkill")
            .args(["/PID", &child_id.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        #[cfg(not(target_os = "windows"))]
        let status = Command::new("kill")
            .args(["-TERM", &child_id.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match status {
            Ok(_) => Ok(()),
            Err(error) => Err(AppError::user(
                "取消下载失败，请稍后重试。",
                error.to_string(),
            )),
        }
    }
}

pub async fn run_download(
    app: AppHandle,
    state: DownloadState,
    request: DownloadRequest,
    dir: PathBuf,
) -> Result<(), AppError> {
    state.reserve().await?;

    let mut command = Command::new(tool_paths::ytdlp());
    command
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .args(ytdlp::download_args(
            &request.mode,
            request.format_id.as_deref(),
            &dir.to_string_lossy(),
            request.url.trim(),
            &request
                .options
                .to_ytdlp_options(ytdlp::site_profile(request.url.trim())),
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            state.clear().await;
            return Err(AppError::user(
                "未检测到 yt-dlp。请先安装 yt-dlp，并确保它可以在命令行中直接运行。",
                error.to_string(),
            ));
        }
    };

    let child_id = match child.id() {
        Some(id) => id,
        None => {
            state.clear().await;
            return Err(AppError::user(
                "下载进程启动失败，请重试。",
                "Could not read child process id",
            ));
        }
    };

    state.set_child_id(child_id).await;
    emit(&app, DownloadProgressEvent::status("starting", None));

    let log_lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let output_path = Arc::new(Mutex::new(None::<String>));
    let last_progress = Arc::new(Mutex::new(None::<DownloadProgressEvent>));
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        readers.push(spawn_line_reader(
            app.clone(),
            stdout,
            log_lines.clone(),
            output_path.clone(),
            last_progress.clone(),
        ));
    }
    if let Some(stderr) = child.stderr.take() {
        readers.push(spawn_line_reader(
            app.clone(),
            stderr,
            log_lines.clone(),
            output_path.clone(),
            last_progress.clone(),
        ));
    }

    let status = child.wait().await?;
    let was_cancelled = state.finish(Some(child_id)).await;
    for reader in readers {
        let _ = reader.await;
    }
    let detail = {
        let lines = log_lines.lock().await;
        lines.join("\n")
    };
    let final_file_path = {
        let path = output_path.lock().await;
        path.clone()
    };
    let final_progress = {
        let progress = last_progress.lock().await;
        progress.clone()
    };

    if was_cancelled {
        emit(&app, DownloadProgressEvent::status("cancelled", None));
        Ok(())
    } else if status.success() {
        emit(
            &app,
            DownloadProgressEvent {
                status: "finished".to_string(),
                percent: Some(100.0),
                total: final_progress
                    .as_ref()
                    .and_then(|event| event.total.clone()),
                speed: final_progress
                    .as_ref()
                    .and_then(|event| event.speed.clone()),
                eta: Some("00:00".to_string()),
                line: None,
                message: Some("下载完成。".to_string()),
                file_path: final_file_path,
            },
        );
        Ok(())
    } else {
        let current_state = DownloadProgressEvent {
            status: "error".to_string(),
            percent: None,
            total: None,
            speed: None,
            eta: None,
            line: None,
            message: Some("下载失败，请查看详细日志。".to_string()),
            file_path: None,
        };
        emit(&app, current_state);
        Err(AppError::user(
            "下载失败，请查看详细日志。",
            if detail.trim().is_empty() {
                format!("yt-dlp exited with status: {status}")
            } else {
                format!("{detail}\n\n退出状态：{status}")
            },
        ))
    }
}

fn spawn_line_reader<R>(
    app: AppHandle,
    reader: R,
    log_lines: Arc<Mutex<Vec<String>>>,
    output_path: Arc<Mutex<Option<String>>>,
    last_progress: Arc<Mutex<Option<DownloadProgressEvent>>>,
) -> JoinHandle<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            {
                let mut logs = log_lines.lock().await;
                logs.push(line.clone());
            }
            if let Some(path) = parse_output_path(&line) {
                let mut output = output_path.lock().await;
                *output = Some(path);
            }
            if let Some(event) = parse_progress_line(&line) {
                if event.percent.is_some() || event.total.is_some() || event.speed.is_some() {
                    let mut progress = last_progress.lock().await;
                    *progress = Some(event.clone());
                }
                emit(&app, event);
            } else {
                emit(
                    &app,
                    DownloadProgressEvent {
                        status: "processing".to_string(),
                        percent: None,
                        total: None,
                        speed: None,
                        eta: None,
                        line: Some(line),
                        message: None,
                        file_path: None,
                    },
                );
            }
        }
    })
}

fn emit(app: &AppHandle, event: DownloadProgressEvent) {
    let _ = app.emit("download://progress", event);
}
