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

/// Why the running worker was asked to stop. Pausing keeps partial files so
/// yt-dlp can continue them on the next attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Cancel,
    Pause,
}

impl StopReason {
    fn status(self) -> &'static str {
        match self {
            StopReason::Cancel => "cancelled",
            StopReason::Pause => "paused",
        }
    }
}

#[derive(Default)]
struct InnerState {
    active: bool,
    child_id: Option<u32>,
    stop: Option<StopReason>,
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
        state.stop = None;
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
        state.stop = None;
    }

    pub async fn finish(&self, child_id: Option<u32>) -> Option<StopReason> {
        let mut state = self.inner.lock().await;
        let stop = state.stop;
        if child_id.is_none() || state.child_id == child_id {
            state.active = false;
            state.child_id = None;
            state.stop = None;
        }
        stop
    }

    pub async fn cancel_current(&self) -> Result<(), AppError> {
        self.request_stop(StopReason::Cancel).await
    }

    pub async fn pause_current(&self) -> Result<(), AppError> {
        self.request_stop(StopReason::Pause).await
    }

    async fn request_stop(&self, reason: StopReason) -> Result<(), AppError> {
        let child_id = {
            let mut state = self.inner.lock().await;
            // Completion in flight keeps its outcome; the first user request wins.
            state.stop.get_or_insert(reason);
            state.child_id
        };

        let Some(child_id) = child_id else {
            return Ok(());
        };
        self.kill_child(child_id).await
    }

    async fn stop_child(&self) -> Result<(), AppError> {
        let child_id = self.inner.lock().await.child_id;
        match child_id {
            Some(child_id) => self.kill_child(child_id).await,
            None => Ok(()),
        }
    }

    async fn kill_child(&self, child_id: u32) -> Result<(), AppError> {
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
                "停止下载失败，请稍后重试。",
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
) -> Result<DownloadResult, AppError> {
    let metadata = download_metadata::take(request.url.trim(), &request.options);
    let request_id = request.request_id.clone();
    let emit: ProgressEmitter = Arc::new(move |event| {
        let _ = app.emit(
            "download://progress",
            RequestProgress {
                request_id: request_id.clone(),
                event,
            },
        );
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

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    pub cancelled: bool,
    pub paused: bool,
    pub file_path: Option<String>,
    pub skipped: bool,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestProgress {
    request_id: Option<String>,
    #[serde(flatten)]
    event: DownloadProgressEvent,
}

async fn run_download_inner(
    state: DownloadState,
    request: DownloadRequest,
    dir: PathBuf,
    mut metadata: Option<Vec<u8>>,
    program: PathBuf,
    ffmpeg: PathBuf,
    emit: ProgressEmitter,
) -> Result<DownloadResult, AppError> {
    state.reserve().await?;
    emit(DownloadProgressEvent::status("starting", None));
    if metadata.is_none() {
        let mut command = hidden_command(&program);
        command
            .kill_on_drop(true)
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUTF8", "1")
            .args(ytdlp::parse_args(
                request.url.trim(),
                false,
                &request
                    .options
                    .to_ytdlp_options(ytdlp::site_profile(&request.url)),
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let preparation = async {
            let child = command.spawn()?;
            if let Some(id) = child.id() {
                state.set_child_id(id).await;
            }
            if state.inner.lock().await.stop.is_some() {
                state.stop_child().await?;
            }
            let output = tokio::time::timeout(Duration::from_secs(30), child.wait_with_output())
                .await
                .map_err(|_| AppError::user("解析超时，请检查网络后重试。", "Parse timed out"))??;
            if !output.status.success() {
                return Err(AppError::user(
                    "视频解析失败。",
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
            Ok::<_, AppError>(output.stdout)
        }
        .await;
        state.inner.lock().await.child_id = None;
        match preparation {
            Ok(json) => metadata = Some(json),
            Err(error) => {
                if let Some(stop) = state.finish(None).await {
                    emit(DownloadProgressEvent::status(stop.status(), None));
                    return Ok(DownloadResult {
                        cancelled: stop == StopReason::Cancel,
                        paused: stop == StopReason::Pause,
                        file_path: None,
                        skipped: false,
                    });
                }
                return Err(error);
            }
        }
    }
    let preparation = (|| {
        let info: serde_json::Value = serde_json::from_slice(metadata.as_deref().unwrap())?;
        let profile = format!(
            "{:?}:{}:{:?}:{}",
            request.mode,
            request.format_id.as_deref().unwrap_or(""),
            request.file_options.quality,
            request
                .file_options
                .audio_format_id
                .as_deref()
                .unwrap_or("")
        );
        let plan =
            crate::download_files::plan_output(&dir, &info, &profile, &request.file_options)?;
        let args =
            ytdlp::configured_download_args(&request, &dir.to_string_lossy(), &plan.template)?;
        Ok::<_, AppError>((plan, args))
    })();
    let (plan, base_args) = match preparation {
        Ok(value) => value,
        Err(error) => {
            state.clear().await;
            return Err(error);
        }
    };
    if let Some(existing) = plan.existing {
        let stop = state.finish(None).await;
        let file_path = if stop.is_none() {
            Some(existing.to_string_lossy().into_owned())
        } else {
            None
        };
        emit(DownloadProgressEvent {
            file_path: file_path.clone(),
            message: Some("Already downloaded; existing file retained.".into()),
            ..DownloadProgressEvent::status(stop.map_or("finished", StopReason::status), None)
        });
        return Ok(DownloadResult {
            cancelled: stop == Some(StopReason::Cancel),
            paused: stop == Some(StopReason::Pause),
            file_path,
            skipped: stop.is_none(),
        });
    }
    loop {
        let input_metadata = metadata.take();
        let using_metadata = input_metadata.is_some();
        let mut args = base_args.clone();
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
        let stop_requested = state.inner.lock().await.stop;
        if stop_requested.is_some() {
            if let Err(error) = state.stop_child().await {
                let _ = child.kill().await;
                state.finish(Some(child_id)).await;
                return Err(error);
            }
        }

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
        let stop_requested = {
            let mut inner = state.inner.lock().await;
            inner.child_id = None;
            inner.stop
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
            && stop_requested.is_none()
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
        let stop = state.finish(None).await;
        let final_file_path = {
            let path = output_path.lock().await;
            path.clone()
        };

        if let Some(stop) = stop {
            emit(DownloadProgressEvent::status(stop.status(), None));
            return Ok(DownloadResult {
                cancelled: stop == StopReason::Cancel,
                paused: stop == StopReason::Pause,
                file_path: None,
                skipped: false,
            });
        } else if status.success() {
            if !final_file_path
                .as_ref()
                .is_some_and(|path| PathBuf::from(path).is_file())
            {
                return Err(AppError::user(
                    "下载器未返回可用的输出文件，请重新解析后重试。",
                    "No completed output file found",
                ));
            }
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
                file_path: final_file_path.clone(),
                metrics: final_progress
                    .as_ref()
                    .and_then(|event| event.metrics.clone()),
            });
            return Ok(DownloadResult {
                cancelled: false,
                paused: false,
                file_path: final_file_path,
                skipped: false,
            });
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
    #[ignore = "requires separate video/en/zh fixtures served at YDLITE_TEST_MEDIA_URL's origin"]
    async fn real_audio_track_selection_and_quality_container() {
        let source = std::env::var("YDLITE_TEST_MEDIA_URL").unwrap();
        let base = source.rsplit_once('/').unwrap().0;
        let dir = std::env::temp_dir().join(format!("ydlite-audio-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let info = crate::commands::parse_video(crate::commands::ParseVideoRequest {
            url: source.clone(),
            options: Default::default(),
        })
        .await
        .unwrap();
        let raw = download_metadata::take(&info.resolved_url, &Default::default()).unwrap();
        let mut metadata: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        for key in [
            "url",
            "requested_downloads",
            "requested_formats",
            "format_id",
        ] {
            metadata.as_object_mut().unwrap().remove(key);
        }
        metadata["formats"] = serde_json::json!([
            {"format_id":"video","url":format!("{base}/video.mp4"),"ext":"mp4","protocol":"http","vcodec":"h264","acodec":"none","height":180,"fps":24},
            {"format_id":"en","url":format!("{base}/en.m4a"),"ext":"m4a","protocol":"http","vcodec":"none","acodec":"aac","language":"en"},
            {"format_id":"zh","url":format!("{base}/zh.m4a"),"ext":"m4a","protocol":"http","vcodec":"none","acodec":"aac","language":"zh"}
        ]);
        for (quality, extension) in [
            (crate::download_files::Quality::Compatible, "mp4"),
            (crate::download_files::Quality::Quality, "mkv"),
        ] {
            let mut request: DownloadRequest = serde_json::from_value(serde_json::json!({"url":source,"dir":dir,"mode":"custom","formatId":"video","fileOptions":{"audioFormatId":"zh"}})).unwrap();
            request.file_options.quality = quality;
            let result = run_download_inner(
                DownloadState::default(),
                request,
                dir.clone(),
                Some(serde_json::to_vec(&metadata).unwrap()),
                tool_paths::ytdlp(),
                tool_paths::ffmpeg(),
                Arc::new(|_| {}),
            )
            .await
            .unwrap();
            let path = result.file_path.unwrap();
            assert_eq!(PathBuf::from(&path).extension().unwrap(), extension);
            let decoded = hidden_command(tool_paths::ffmpeg())
                .args([
                    "-v", "error", "-i", &path, "-map", "0:a:0", "-t", "1", "-f", "s16le", "-ac",
                    "1", "-ar", "8000", "-",
                ])
                .output()
                .await
                .unwrap();
            assert!(decoded.status.success());
            let samples: Vec<i16> = decoded
                .stdout
                .chunks_exact(2)
                .map(|s| i16::from_le_bytes([s[0], s[1]]))
                .collect();
            let crossings = samples
                .windows(2)
                .filter(|s| (s[0] < 0) != (s[1] < 0))
                .count();
            let frequency = crossings as f64 / 2.0 / (samples.len() as f64 / 8000.0);
            assert!(
                (820.0..940.0).contains(&frequency),
                "selected zh track must contain the 880 Hz tone, not the 440 Hz automatic track"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    #[ignore = "network download; set YDLITE_TEST_PUBLIC_URL to an authorized public video"]
    async fn real_public_download_and_playback() {
        let url = std::env::var("YDLITE_TEST_PUBLIC_URL").expect("specify the public source");
        let dir = std::env::temp_dir().join(format!("ydlite-public-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        for mode in [
            crate::commands::DownloadMode::Best,
            crate::commands::DownloadMode::P720,
            crate::commands::DownloadMode::Mp3,
        ] {
            let info = crate::commands::parse_video(crate::commands::ParseVideoRequest {
                url: url.clone(),
                options: Default::default(),
            })
            .await
            .unwrap();
            let metadata = download_metadata::take(&info.resolved_url, &Default::default());
            let request = DownloadRequest {
                request_id: Some(uuid::Uuid::new_v4().to_string()),
                url: info.resolved_url,
                dir: dir.to_string_lossy().into_owned(),
                mode: mode.clone(),
                format_id: None,
                options: Default::default(),
                file_options: Default::default(),
                overwrite_confirmed: false,
            };
            let result = run_download_inner(
                DownloadState::default(),
                request,
                dir.clone(),
                metadata,
                tool_paths::ytdlp(),
                tool_paths::ffmpeg(),
                Arc::new(|_| {}),
            )
            .await
            .unwrap();
            let path = result.file_path.unwrap();
            let probe = hidden_command(tool_paths::ffprobe())
                .args([
                    "-v",
                    "error",
                    "-show_entries",
                    "stream=codec_name,height",
                    "-of",
                    "json",
                ])
                .arg(&path)
                .output()
                .await
                .unwrap();
            assert!(probe.status.success(), "output must be playable");
            let streams: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
            let streams = streams["streams"].as_array().unwrap();
            assert!(!streams.is_empty());
            if matches!(mode, crate::commands::DownloadMode::P720) {
                assert!(streams
                    .iter()
                    .any(|s| s["height"].as_u64().is_some_and(|h| h <= 720)));
            }
            if matches!(mode, crate::commands::DownloadMode::Mp3) {
                assert!(streams.iter().any(|s| s["codec_name"] == "mp3"));
            }
            println!(
                "Public source {:?}: playable, {} bytes",
                mode,
                std::fs::metadata(&path).unwrap().len()
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    #[ignore = "requires yt-dlp, FFmpeg and YDLITE_TEST_MEDIA_URL pointing to a playable fixture"]
    async fn real_file_policies_and_mp3_conversion() {
        let url = std::env::var("YDLITE_TEST_MEDIA_URL").expect("set the fixture URL");
        let dir = std::env::temp_dir().join(format!("ydlite-policy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let run = async {
            let info = crate::commands::parse_video(crate::commands::ParseVideoRequest {
                url: url.clone(),
                options: Default::default(),
            })
            .await
            .unwrap();
            let raw = download_metadata::take(&info.resolved_url, &Default::default()).unwrap();
            let mut json: serde_json::Value = serde_json::from_slice(&raw).unwrap();
            json["title"] = "中文标题 同名验证".into();
            json["uploader"] = "测试作者".into();
            let raw = serde_json::to_vec(&json).unwrap();
            let mut request: DownloadRequest =
                serde_json::from_value(serde_json::json!({"url":url,"dir":dir,"mode":"best"}))
                    .unwrap();
            let execute = |request: DownloadRequest| {
                run_download_inner(
                    DownloadState::default(),
                    request,
                    dir.clone(),
                    Some(raw.clone()),
                    tool_paths::ytdlp(),
                    tool_paths::ffmpeg(),
                    Arc::new(|_| {}),
                )
            };
            let first = execute(request.clone()).await.unwrap();
            let path = PathBuf::from(first.file_path.unwrap());
            assert!(path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("中文标题"));
            let bytes = std::fs::read(&path).unwrap();
            assert!(!bytes.is_empty());
            let skipped = execute(request.clone()).await.unwrap();
            assert!(skipped.skipped);
            assert_eq!(skipped.file_path, Some(path.to_string_lossy().into_owned()));
            request.file_options.conflict = crate::download_files::Conflict::Number;
            let copy = execute(request.clone()).await.unwrap();
            assert!(copy.file_path.unwrap().contains(" (2)."));
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            request.file_options.conflict = crate::download_files::Conflict::Overwrite;
            assert!(
                execute(request.clone()).await.is_err(),
                "backend also requires confirmation"
            );
            request.overwrite_confirmed = true;
            std::fs::write(&path, b"replacement fixture").unwrap();
            execute(request.clone()).await.unwrap();
            assert!(
                std::fs::read(&path).unwrap() == bytes,
                "explicit replacement restored real media"
            );
            request.file_options.conflict = crate::download_files::Conflict::Skip;
            request.mode = crate::commands::DownloadMode::Mp3;
            let mp3 = execute(request.clone()).await.unwrap();
            let mp3 = PathBuf::from(mp3.file_path.unwrap());
            assert_eq!(mp3.extension().unwrap(), "mp3");
            let probe = hidden_command(tool_paths::ffprobe())
                .args([
                    "-v",
                    "error",
                    "-show_entries",
                    "stream=codec_name",
                    "-of",
                    "csv=p=0",
                ])
                .arg(&mp3)
                .output()
                .await
                .unwrap();
            assert!(probe.status.success());
            assert!(String::from_utf8_lossy(&probe.stdout).contains("mp3"));
            request.mode = crate::commands::DownloadMode::Custom;
            request.format_id = Some("does-not-exist".into());
            assert!(
                execute(request).await.is_err(),
                "invalid explicit format must fail rather than download best"
            );
        }
        .await;
        std::fs::remove_dir_all(dir).unwrap();
        run
    }

    #[test]
    fn progress_payload_identifies_the_download_attempt() {
        let payload = serde_json::to_value(RequestProgress {
            request_id: Some("attempt-2".into()),
            event: DownloadProgressEvent::status("starting", None),
        })
        .unwrap();
        assert_eq!(payload["requestId"], "attempt-2");
        assert_eq!(payload["status"], "starting");
        assert!(payload.get("event").is_none());
    }

    #[tokio::test]
    async fn cancellation_before_spawn_keeps_slot_reserved() {
        let state = DownloadState::default();
        state.reserve().await.unwrap();
        state.cancel_current().await.unwrap();
        assert!(state.reserve().await.is_err());
        state.set_child_id(42).await;
        assert_eq!(state.finish(Some(42)).await, Some(StopReason::Cancel));
        assert!(state.reserve().await.is_ok());
    }

    #[tokio::test]
    async fn pause_is_distinct_from_cancel_and_the_first_request_wins() {
        let state = DownloadState::default();
        state.reserve().await.unwrap();
        state.pause_current().await.unwrap();
        state.cancel_current().await.unwrap();
        state.set_child_id(7).await;
        assert_eq!(state.finish(Some(7)).await, Some(StopReason::Pause));
        assert!(state.reserve().await.is_ok());
        state.cancel_current().await.unwrap();
        assert_eq!(state.finish(None).await, Some(StopReason::Cancel));
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
        let expired_hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let server_expired_hits = expired_hits.clone();
        let media_hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let server_media_hits = media_hits.clone();
        let server = std::thread::spawn(move || {
            while !server_stop.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                let body = server_media.clone();
                let page_hits = server_page_hits.clone();
                let expired_hits = server_expired_hits.clone();
                let media_hits = server_media_hits.clone();
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
                    } else if request.starts_with("GET /expired")
                        || request.starts_with("HEAD /expired")
                    {
                        expired_hits.fetch_add(1, Ordering::Relaxed);
                        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    } else {
                        if request.starts_with("GET /media.mp4 ") {
                            media_hits.fetch_add(1, Ordering::Relaxed);
                        }
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
                } else {
                    let info = crate::commands::parse_video(crate::commands::ParseVideoRequest {
                        url: url.clone(), options: Default::default(),
                    }).await.unwrap();
                    let raw = download_metadata::take(&info.resolved_url, &Default::default()).expect("real media parsing must cache complete metadata");
                    let mut metadata: serde_json::Value = serde_json::from_slice(&raw).unwrap();
                    if name == "expired" {
                        if metadata.get("url").is_some() { metadata["url"] = cached_url.clone().into(); }
                        if let Some(formats) = metadata["formats"].as_array_mut() {
                            for format in formats { format["url"] = cached_url.clone().into(); }
                        }
                    }
                    Some(serde_json::to_vec(&metadata).unwrap())
                };
                let page_hits_before = page_hits.load(Ordering::Relaxed);
                let expired_hits_before = expired_hits.load(Ordering::Relaxed);
                let media_hits_before = media_hits.load(Ordering::Relaxed);
                let first_bytes = Arc::new(StdMutex::new(None::<Duration>));
                let first_bytes_recorded = first_bytes.clone();
                let clicked_at = Instant::now();
                let events = Arc::new(StdMutex::new(Vec::new()));
                let recorded = events.clone();
                let state = DownloadState::default();
                let outcome = run_download_inner(state.clone(), DownloadRequest {
                    request_id: Some("test-reuse".into()),
                    url, dir: dir.to_string_lossy().into(), mode: crate::commands::DownloadMode::Best,
                    format_id: None, options: Default::default(), file_options: Default::default(), overwrite_confirmed: false,
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
                assert!(!outcome.cancelled);
                assert_eq!(outcome.file_path, finished.file_path, "IPC result must retain the completion path independently of events");
                assert_eq!(std::fs::read(finished.file_path.as_ref().unwrap()).unwrap(), *media);
                if name == "expired" {
                    // yt-dlp may recover internally before the application retry is needed.
                    assert!(expired_hits.load(Ordering::Relaxed) > expired_hits_before, "the stale media URL must actually be attempted");
                    assert!(media_hits.load(Ordering::Relaxed) > media_hits_before, "recovery must fetch the fresh media URL");
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
                request_id: Some("test-cancel".into()),
                url: format!("{base}/media.mp4"), dir: dir.to_string_lossy().into(),
                mode: crate::commands::DownloadMode::Best, format_id: None, options: Default::default(), file_options: Default::default(), overwrite_confirmed: false,
            }, dir, Some(b"{}".to_vec()), program, PathBuf::new(), Arc::new(move |event| {
                if event.status == "starting" { notify.notify_one(); }
                recorded.lock().unwrap().push(event);
            })));
            started.notified().await;
            assert!(state.reserve().await.is_err());
            state.cancel_current().await.unwrap();
            let outcome = tokio::time::timeout(Duration::from_secs(15), task).await.unwrap().unwrap().unwrap();
            assert!(outcome.cancelled);
            assert!(outcome.file_path.is_none());
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

    #[tokio::test]
    #[ignore = "requires the installed yt-dlp executable and a local HTTP fixture"]
    async fn real_pause_keeps_partial_file_and_resume_continues_it() {
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
        let media = Arc::new(vec![7u8; 2 * 1024 * 1024]);
        let server_media = media.clone();
        let range_start = Arc::new(StdMutex::new(None::<u64>));
        let server_range = range_start.clone();
        let server = std::thread::spawn(move || {
            while !server_stop.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                let body = server_media.clone();
                let range = server_range.clone();
                std::thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    let mut request = [0u8; 4096];
                    let length = stream.read(&mut request).unwrap_or(0);
                    let request = String::from_utf8_lossy(&request[..length]).to_string();
                    let offset = request
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("range: bytes=")?
                                .trim()
                                .trim_end_matches('-')
                                .parse::<u64>()
                                .ok()
                        })
                        .unwrap_or(0);
                    if offset > 0 {
                        *range.lock().unwrap() = Some(offset);
                        let header = format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Type: video/mp4\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            offset, body.len() - 1, body.len(), body.len() as u64 - offset
                        );
                        let _ = stream.write_all(header.as_bytes());
                    } else {
                        let header = format!("HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                        let _ = stream.write_all(header.as_bytes());
                    }
                    if !request.starts_with("HEAD ") {
                        for chunk in body[offset as usize..].chunks(8192) {
                            if stream.write_all(chunk).is_err() {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(12));
                        }
                    }
                });
            }
        });
        let dir = std::env::temp_dir().join(format!("ydlite-pause-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let url = format!("{base}/media.mp4");
        let state = DownloadState::default();
        let active = state.clone();
        let started = Arc::new(tokio::sync::Notify::new());
        let notify = started.clone();
        let task = tokio::spawn(run_download_inner(active, DownloadRequest {
            request_id: Some("test-pause".into()),
            url: url.clone(), dir: dir.to_string_lossy().into(),
            mode: crate::commands::DownloadMode::Best, format_id: None, options: Default::default(), file_options: Default::default(), overwrite_confirmed: false,
        }, dir.clone(), None, program.clone(), PathBuf::new(), Arc::new(move |event| {
            if event.status == "downloading" {
                let downloaded = event.metrics.as_ref().and_then(|metrics| metrics.downloaded_bytes).unwrap_or(0);
                if downloaded > 0 {
                    notify.notify_waiters();
                }
            }
        })));
        tokio::time::timeout(Duration::from_secs(60), started.notified()).await.expect("download never reported bytes");
        state.pause_current().await.unwrap();
        let outcome = tokio::time::timeout(Duration::from_secs(20), task).await.unwrap().unwrap().unwrap();
        assert!(outcome.paused, "a pause must not be reported as a cancel");
        assert!(!outcome.cancelled);
        assert!(outcome.file_path.is_none());
        let part = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "part"))
            .expect("pausing must leave the partial file for resuming");
        assert!(std::fs::metadata(&part).unwrap().len() > 0);

        let resumed = run_download_inner(
            DownloadState::default(),
            DownloadRequest {
                request_id: Some("test-resume".into()),
                url, dir: dir.to_string_lossy().into(),
                mode: crate::commands::DownloadMode::Best, format_id: None, options: Default::default(), file_options: Default::default(), overwrite_confirmed: false,
            },
            dir.clone(), None, program, PathBuf::new(), Arc::new(|_| {}),
        )
        .await
        .unwrap();
        assert!(!resumed.paused && !resumed.cancelled);
        let file = PathBuf::from(resumed.file_path.expect("resume completes with a file"));
        assert!(std::fs::read(&file).unwrap() == *media);
        assert!(range_start.lock().unwrap().is_some_and(|offset| offset > 0), "resume must continue from the partial file");
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
