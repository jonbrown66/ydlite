use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::commands::DownloadRequest;
use crate::download_metadata;
use crate::errors::AppError;
use crate::process_utils::hidden_command;
use crate::progress::{
    parse_download_plan_line, parse_output_path, parse_progress_line, DownloadProgressEvent,
    DownloadProgressTracker,
};
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
            return Ok(());
        };

        #[cfg(target_os = "windows")]
        let status = hidden_command("taskkill")
            .args(["/PID", &child_id.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        #[cfg(not(target_os = "windows"))]
        let status = hidden_command("kill")
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
    let metadata = download_metadata::take(request.url.trim(), &request.options);
    let emit: ProgressEmitter = Arc::new(move |event| {
        let _ = app.emit("download://progress", event);
    });
    run_download_inner(
        state,
        request,
        dir,
        metadata,
        tool_paths::ytdlp(),
        tool_paths::ffmpeg(),
        emit,
    )
    .await
}

type ProgressEmitter = Arc<dyn Fn(DownloadProgressEvent) + Send + Sync>;

async fn run_download_inner(
    state: DownloadState,
    request: DownloadRequest,
    dir: PathBuf,
    mut metadata: Option<Vec<u8>>,
    program: PathBuf,
    ffmpeg: PathBuf,
    emit: ProgressEmitter,
) -> Result<(), AppError> {
    state.reserve().await?;
    loop {
        let input_metadata = metadata.take();
        let using_metadata = input_metadata.is_some();
        let mut args = ytdlp::download_args(
            &request.mode,
            request.format_id.as_deref(),
            &dir.to_string_lossy(),
            request.url.trim(),
            &request
                .options
                .to_ytdlp_options(ytdlp::site_profile(request.url.trim())),
        );
        if using_metadata {
            args.pop(); // The metadata is the only input; passing the URL would download it twice.
            args.extend(["--load-info-json".into(), "-".into()]);
        }
        let mut command = hidden_command(&program);
        if ffmpeg.is_file() {
            command.arg("--ffmpeg-location").arg(&ffmpeg);
        }
        command
            .kill_on_drop(true)
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUTF8", "1")
            .args(args)
            .stdin(if using_metadata {
                Stdio::piped()
            } else {
                Stdio::null()
            })
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
        let cancel_requested = state.inner.lock().await.cancel_requested;
        if cancel_requested {
            if let Err(error) = state.cancel_current().await {
                let _ = child.kill().await;
                state.finish(Some(child_id)).await;
                return Err(error);
            }
        }
        emit(DownloadProgressEvent::status("starting", None));

        let log_lines = Arc::new(Mutex::new(VecDeque::<String>::new()));
        let output_path = Arc::new(Mutex::new(None::<String>));
        let last_progress = Arc::new(Mutex::new(None::<DownloadProgressEvent>));
        let progress_tracker = Arc::new(Mutex::new(DownloadProgressTracker::default()));
        let mut readers = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            readers.push(spawn_line_reader(
                emit.clone(),
                stdout,
                log_lines.clone(),
                output_path.clone(),
                last_progress.clone(),
                progress_tracker.clone(),
            ));
        }
        if let Some(stderr) = child.stderr.take() {
            readers.push(spawn_line_reader(
                emit.clone(),
                stderr,
                log_lines.clone(),
                output_path.clone(),
                last_progress.clone(),
                progress_tracker.clone(),
            ));
        }

        // Feed metadata through the pipe, without persisting signed media URLs to disk.
        let input_writer = child
            .stdin
            .take()
            .zip(input_metadata)
            .map(|(mut input, json)| {
                tokio::spawn(async move {
                    input.write_all(&json).await?;
                    input.shutdown().await
                })
            });

        let status = match child.wait().await {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill().await;
                state.finish(Some(child_id)).await;
                for reader in readers {
                    reader.abort();
                }
                emit(DownloadProgressEvent::status(
                    "error",
                    Some(error.to_string()),
                ));
                return Err(error.into());
            }
        };
        for reader in readers {
            let _ = reader.await;
        }
        if let Some(writer) = input_writer {
            let _ = writer.await;
        }
        let cancelled = {
            let mut inner = state.inner.lock().await;
            inner.child_id = None;
            inner.cancel_requested
        };
        let detail = {
            let lines = log_lines.lock().await;
            lines
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("\n")
        };
        let final_progress = last_progress.lock().await.clone();
        if using_metadata
            && !status.success()
            && !cancelled
            && should_refresh_metadata(&detail, final_progress.as_ref())
        {
            // Expired URLs or changed site signatures get one fresh extraction. Keep
            // the slot reserved and do not emit an error/completion for this attempt.
            emit(DownloadProgressEvent {
                message: Some("Refreshing media link…".into()),
                ..DownloadProgressEvent::status("starting", None)
            });
            continue;
        }
        let was_cancelled = state.finish(None).await;
        let final_file_path = {
            let path = output_path.lock().await;
            path.clone()
        };

        if was_cancelled {
            emit(DownloadProgressEvent::status("cancelled", None));
            return Ok(());
        } else if status.success() {
            emit(DownloadProgressEvent {
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
                metrics: final_progress
                    .as_ref()
                    .and_then(|event| event.metrics.clone()),
            });
            return Ok(());
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
                metrics: None,
            };
            emit(current_state);
            return Err(AppError::user(
                "下载失败，请查看详细日志。",
                if detail.trim().is_empty() {
                    format!("yt-dlp exited with status: {status}")
                } else {
                    format!("{detail}\n\n退出状态：{status}")
                },
            ));
        }
    }
}

fn should_refresh_metadata(detail: &str, progress: Option<&DownloadProgressEvent>) -> bool {
    let received_data = progress.is_some_and(|event| {
        event.status == "processing"
            || event.percent.is_some_and(|percent| percent > 0.0)
            || event
                .metrics
                .as_ref()
                .and_then(|metrics| metrics.downloaded_bytes)
                .unwrap_or(0)
                > 0
    });
    if received_data {
        return false;
    }
    let detail = detail.to_ascii_lowercase();
    [
        "http error 401",
        "http error 403",
        "http error 404",
        "http error 410",
        "signature",
        "url expired",
        "jsondecodeerror",
        "keyerror",
        "invalid info json",
    ]
    .iter()
    .any(|reason| detail.contains(reason))
}

fn spawn_line_reader<R>(
    emit: ProgressEmitter,
    reader: R,
    log_lines: Arc<Mutex<VecDeque<String>>>,
    output_path: Arc<Mutex<Option<String>>>,
    last_progress: Arc<Mutex<Option<DownloadProgressEvent>>>,
    progress_tracker: Arc<Mutex<DownloadProgressTracker>>,
) -> JoinHandle<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        let mut last_emit: Option<Instant> = None;
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().starts_with(ytdlp::PROGRESS_PLAN_PREFIX) {
                if let Some(plan) = parse_download_plan_line(&line) {
                    progress_tracker.lock().await.set_plan(plan);
                }
                continue;
            }
            {
                let mut logs = log_lines.lock().await;
                logs.push_back(line.clone());
                if logs.len() > 1000 {
                    logs.pop_front();
                }
            }
            if let Some(path) = parse_output_path(&line) {
                let mut output = output_path.lock().await;
                *output = Some(path);
            }
            if let Some(mut event) = parse_progress_line(&line) {
                progress_tracker.lock().await.apply(&mut event);
                if event.percent.is_some() || event.total.is_some() || event.speed.is_some() {
                    let mut progress = last_progress.lock().await;
                    *progress = Some(event.clone());
                }
                if event.status != "downloading"
                    || last_emit.is_none_or(|time| time.elapsed() >= Duration::from_millis(200))
                {
                    last_emit = Some(Instant::now());
                    emit(event);
                }
            } else {
                emit(DownloadProgressEvent {
                    status: "log".to_string(),
                    percent: None,
                    total: None,
                    speed: None,
                    eta: None,
                    line: Some(line),
                    message: None,
                    file_path: None,
                    metrics: None,
                });
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancellation_before_spawn_keeps_slot_reserved() {
        let state = DownloadState::default();
        state.reserve().await.unwrap();
        state.cancel_current().await.unwrap();
        assert!(state.reserve().await.is_err());
        state.set_child_id(42).await;
        assert!(state.finish(Some(42)).await);
        assert!(state.reserve().await.is_ok());
    }

    #[test]
    fn metadata_retry_does_not_repeat_disk_or_postprocessing_failures() {
        assert!(should_refresh_metadata(
            "ERROR: HTTP Error 403: Forbidden",
            None
        ));
        assert!(should_refresh_metadata(
            "JSONDecodeError: invalid input",
            None
        ));
        assert!(!should_refresh_metadata("Permission denied", None));
        assert!(!should_refresh_metadata("No space left on device", None));
        assert!(!should_refresh_metadata(
            "Requested format is not available",
            None
        ));
        let mut event = DownloadProgressEvent::status("downloading", None);
        event.percent = Some(30.0);
        assert!(!should_refresh_metadata("HTTP Error 403", Some(&event)));
        event.status = "processing".into();
        event.percent = None;
        assert!(!should_refresh_metadata("signature error", Some(&event)));
    }

    #[tokio::test]
    #[ignore = "requires the installed yt-dlp executable and a local HTTP fixture"]
    async fn real_downloader_reuses_metadata_falls_back_and_cancels_once() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Mutex as StdMutex;

        let program = tool_paths::ytdlp();
        assert!(
            program.is_file(),
            "Install yt-dlp before running this check"
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let server_stop = stop.clone();
        let media = Arc::new(vec![0u8; 256 * 1024]);
        let server_media = media.clone();
        let page_hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let server_page_hits = page_hits.clone();
        let server = std::thread::spawn(move || {
            while !server_stop.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                let body = server_media.clone();
                let page_hits = server_page_hits.clone();
                std::thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    let mut request = [0u8; 4096];
                    let length = stream.read(&mut request).unwrap_or(0);
                    let request = String::from_utf8_lossy(&request[..length]);
                    if request.starts_with("GET /page ") {
                        page_hits.fetch_add(1, Ordering::Relaxed);
                        std::thread::sleep(Duration::from_millis(600));
                        let html = b"<html><title>Fixture</title><video src=\"/media.mp4\"></video></html>";
                        let header = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", html.len());
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(html);
                    } else if request.starts_with("GET /expired") {
                        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    } else {
                        let header = format!("HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                        if stream.write_all(header.as_bytes()).is_err() {
                            return;
                        }
                        if !request.starts_with("HEAD ") {
                            for chunk in body.chunks(8192) {
                                if stream.write_all(chunk).is_err() {
                                    break;
                                }
                                std::thread::sleep(Duration::from_millis(10));
                            }
                        }
                    }
                });
            }
        });
        let root = std::env::temp_dir().join(format!(
            "ydlite-native-performance-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&root).unwrap();
        let result = async {
            for (name, cached_url) in [("url-baseline", format!("{base}/media.mp4")), ("parsed-reuse", format!("{base}/media.mp4")), ("cached", format!("{base}/media.mp4")), ("expired", format!("{base}/expired.mp4"))] {
                let dir = root.join(name);
                std::fs::create_dir(&dir).unwrap();
                let url = format!("{base}/{}", if ["url-baseline", "parsed-reuse"].contains(&name) { "page" } else { "media.mp4" });
                let metadata = if name == "url-baseline" { None } else if name == "parsed-reuse" {
                    let info = crate::commands::parse_video(crate::commands::ParseVideoRequest { url: url.clone(), options: Default::default() }).await.unwrap();
                    let parsed = download_metadata::take(&info.resolved_url, &Default::default());
                    assert!(parsed.is_some(), "the actual parse -> cache -> download pipeline must hit, including PATH tools");
                    parsed
                } else { Some(serde_json::to_vec(&serde_json::json!({
                    "id":"fixture", "title":"fixture", "extractor":"generic", "extractor_key":"Generic",
                    "webpage_url":url, "formats":[{"format_id":"http", "url":cached_url,
                    "ext":"mp4", "vcodec":"unknown", "acodec":"unknown", "filesize":media.len()}]
                })).unwrap()) };
                let page_hits_before = page_hits.load(Ordering::Relaxed);
                let first_bytes = Arc::new(StdMutex::new(None::<Duration>));
                let first_bytes_recorded = first_bytes.clone();
                let clicked_at = Instant::now();
                let events = Arc::new(StdMutex::new(Vec::new()));
                let recorded = events.clone();
                let state = DownloadState::default();
                run_download_inner(state.clone(), DownloadRequest {
                    url, dir: dir.to_string_lossy().into(), mode: crate::commands::DownloadMode::Best,
                    format_id: None, options: Default::default(),
                }, dir, metadata, program.clone(), PathBuf::new(), Arc::new(move |event| {
                    if event.metrics.as_ref().and_then(|metrics| metrics.downloaded_bytes).unwrap_or(0) > 0 {
                        first_bytes_recorded.lock().unwrap().get_or_insert_with(|| clicked_at.elapsed());
                    }
                    recorded.lock().unwrap().push(event);
                })).await.unwrap();
                println!("Download trigger -> first bytes ({name}): {:.3}s", first_bytes.lock().unwrap().unwrap().as_secs_f64());
                if name == "parsed-reuse" {
                    assert_eq!(page_hits.load(Ordering::Relaxed), page_hits_before, "download must not re-extract the parsed page");
                }
                let events = events.lock().unwrap();
                assert_eq!(events.iter().filter(|e| e.status == "finished").count(), 1, "{name}: one completion");
                assert!(!events.iter().any(|e| e.status == "error"), "{name}: no intermediate error");
                let finished = events.iter().find(|e| e.status == "finished").unwrap();
                assert_eq!(std::fs::read(finished.file_path.as_ref().unwrap()).unwrap(), *media);
                if name == "expired" {
                    assert!(events.iter().any(|e| e.message.as_deref() == Some("Refreshing media link…")));
                }
                drop(events);
                state.reserve().await.unwrap();
                state.clear().await;
            }
            let state = DownloadState::default();
            let active = state.clone();
            let events = Arc::new(StdMutex::new(Vec::new()));
            let recorded = events.clone();
            let started = Arc::new(tokio::sync::Notify::new());
            let notify = started.clone();
            let dir = root.join("cancel");
            std::fs::create_dir(&dir).unwrap();
            let task = tokio::spawn(run_download_inner(active, DownloadRequest {
                url: format!("{base}/media.mp4"), dir: dir.to_string_lossy().into(),
                mode: crate::commands::DownloadMode::Best, format_id: None, options: Default::default(),
            }, dir, Some(b"{}".to_vec()), program, PathBuf::new(), Arc::new(move |event| {
                if event.status == "starting" { notify.notify_one(); }
                recorded.lock().unwrap().push(event);
            })));
            started.notified().await;
            assert!(state.reserve().await.is_err());
            state.cancel_current().await.unwrap();
            tokio::time::timeout(Duration::from_secs(15), task).await.unwrap().unwrap().unwrap();
            let events = events.lock().unwrap();
            assert_eq!(events.iter().filter(|e| e.status == "cancelled").count(), 1);
            assert_eq!(events.iter().filter(|e| e.status == "starting").count(), 1, "cancel does not retry");
            assert!(!events.iter().any(|e| ["finished", "error"].contains(&e.status.as_str())));
        }.await;
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
        std::fs::remove_dir_all(root).unwrap();
        result
    }
}
