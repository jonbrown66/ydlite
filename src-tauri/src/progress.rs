use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

const PROGRESS_PLAN_PREFIX: &str = "__YDLITE_PLAN__";
const IN_PROGRESS_MAX_PERCENT: f32 = 99.8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DownloadProgressPlan {
    stage_sizes: Vec<Option<u64>>,
    stage_ids: Vec<String>,
}

#[derive(Debug, Default)]
pub(crate) struct DownloadProgressTracker {
    plan: Option<DownloadProgressPlan>,
    stage_index: usize,
    stage_high_water_percent: f32,
    last_raw_percent: Option<f32>,
    last_emitted_percent: f32,
    current_stage_id: Option<String>,
    stage_downloaded: Vec<u64>,
    stage_exact: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadMetrics {
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub total_is_estimate: bool,
    /// Raw bytes per second, so the UI can draw a real speed trace.
    pub speed_bytes: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub status: String,
    pub percent: Option<f32>,
    pub total: Option<String>,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub line: Option<String>,
    pub message: Option<String>,
    pub file_path: Option<String>,
    pub metrics: Option<DownloadMetrics>,
}

impl DownloadProgressEvent {
    pub fn status(status: &str, line: Option<String>) -> Self {
        Self {
            status: status.to_string(),
            percent: None,
            total: None,
            speed: None,
            eta: None,
            line,
            message: None,
            file_path: None,
            metrics: None,
        }
    }
}

impl DownloadProgressTracker {
    pub fn set_plan(&mut self, plan: DownloadProgressPlan) {
        self.stage_downloaded = vec![0; plan.stage_sizes.len()];
        self.stage_exact = vec![false; plan.stage_sizes.len()];
        self.plan = Some(plan);
        self.stage_index = 0;
        self.stage_high_water_percent = 0.0;
        self.last_raw_percent = None;
    }

    pub fn apply(&mut self, event: &mut DownloadProgressEvent) {
        if event.status != "downloading" {
            return;
        }

        let structured = event.line.as_deref().and_then(structured_progress);
        if let Some((stage_id, data)) = &structured {
            let changed = self.current_stage_id.as_deref() != Some(stage_id.as_str());
            self.current_stage_id = Some(stage_id.clone());
            if let Some(plan) = &mut self.plan {
                if let Some(index) = plan.stage_ids.iter().position(|id| id == stage_id) {
                    self.stage_index = index;
                    let exact_total = data
                        .get("total_bytes")
                        .and_then(json_byte_count)
                        .filter(|value| *value > 0);
                    if let Some(total) = exact_total.or_else(|| {
                        data.get("total_bytes_estimate")
                            .and_then(json_byte_count)
                            .filter(|v| *v > 0)
                    }) {
                        plan.stage_sizes[index] = Some(total);
                        self.stage_exact[index] = exact_total.is_some();
                    }
                    self.stage_downloaded[index] = data
                        .get("downloaded_bytes")
                        .and_then(json_byte_count)
                        .unwrap_or(self.stage_downloaded[index]);
                    let total = plan.stage_sizes.iter().try_fold(0u64, |sum, size| {
                        size.and_then(|size| sum.checked_add(size))
                    });
                    let completed = plan
                        .stage_sizes
                        .iter()
                        .take(index)
                        .enumerate()
                        .map(|(i, size)| size.unwrap_or(self.stage_downloaded[i]))
                        .sum::<u64>();
                    let downloaded = completed.saturating_add(self.stage_downloaded[index]);
                    event.metrics = Some(DownloadMetrics {
                        downloaded_bytes: Some(downloaded),
                        total_bytes: total,
                        total_is_estimate: !self.stage_exact.iter().all(|exact| *exact),
                        speed_bytes: data
                            .get("speed")
                            .and_then(|v| v.as_f64())
                            .filter(|v| v.is_finite() && *v >= 0.0),
                    });
                    event.total = total.map(|bytes| byte_label(bytes as f64));
                    // yt-dlp's ETA covers only the current stream; include the remaining audio/video.
                    event.eta = total
                        .zip(
                            data.get("speed")
                                .and_then(|v| v.as_f64())
                                .filter(|v| v.is_finite() && *v > 0.0),
                        )
                        .map(|(total, speed)| {
                            time_label(
                                (total.saturating_sub(downloaded) as f64 / speed).ceil() as u64
                            )
                        });
                    event.percent = total.map(|total| {
                        (downloaded as f64 / total as f64 * 100.0)
                            .clamp(0.0, IN_PROGRESS_MAX_PERCENT as f64)
                            as f32
                    });
                    if total.is_none() {
                        event.message = Some(format!(
                            "Downloading stream {}/{}",
                            index + 1,
                            plan.stage_sizes.len()
                        ));
                    }
                    return;
                }
            }
            if changed {
                self.stage_high_water_percent = 0.0;
                self.last_raw_percent = None;
            }
        }
        let Some(raw_percent) = event.percent.filter(|percent| percent.is_finite()) else {
            return;
        };
        let raw_percent = raw_percent.clamp(0.0, 100.0);

        if let Some(plan) = &self.plan {
            if structured.is_none()
                && self.stage_index + 1 < plan.stage_sizes.len()
                && self
                    .last_raw_percent
                    .is_some_and(|previous| previous >= 95.0)
                && raw_percent <= 5.0
            {
                self.stage_index += 1;
                self.stage_high_water_percent = 0.0;
                self.last_raw_percent = None;
            }

            self.stage_high_water_percent = raw_percent;
            self.last_raw_percent = Some(raw_percent);
            if structured.is_some() && plan.stage_sizes.iter().any(|size| size.is_none()) {
                event.percent = None;
                event.message = Some(format!(
                    "Downloading stream {}/{}",
                    self.stage_index + 1,
                    plan.stage_sizes.len()
                ));
                return;
            }
            let aggregate_percent =
                plan.aggregate_percent(self.stage_index, self.stage_high_water_percent);
            self.last_emitted_percent = aggregate_percent.min(IN_PROGRESS_MAX_PERCENT);
        } else {
            self.last_emitted_percent = raw_percent.min(IN_PROGRESS_MAX_PERCENT);
        }

        event.percent = Some(self.last_emitted_percent);
    }
}

impl DownloadProgressPlan {
    fn aggregate_percent(&self, stage_index: usize, stage_percent: f32) -> f32 {
        let all_sizes_known = self
            .stage_sizes
            .iter()
            .all(|size| size.is_some_and(|value| value > 0));
        let weights = if all_sizes_known {
            self.stage_sizes
                .iter()
                .map(|size| size.unwrap_or_default() as f32)
                .collect::<Vec<_>>()
        } else {
            vec![1.0; self.stage_sizes.len()]
        };
        let total_weight = weights.iter().sum::<f32>().max(1.0);
        let completed_weight = weights.iter().take(stage_index).sum::<f32>();
        let current_weight = weights.get(stage_index).copied().unwrap_or_default();

        ((completed_weight + current_weight * stage_percent / 100.0) / total_weight * 100.0)
            .clamp(0.0, 100.0)
    }
}

pub(crate) fn parse_download_plan_line(line: &str) -> Option<DownloadProgressPlan> {
    let values = line.trim().strip_prefix(PROGRESS_PLAN_PREFIX)?;
    let mut fields = values.split('|');
    let first_format = fields.next()?.trim();
    let first_size = fields.next()?.trim();
    let second_format = fields.next()?.trim();
    let second_size = fields.next()?.trim();

    if fields.next().is_some()
        || !is_download_format(first_format)
        || !is_download_format(second_format)
    {
        return None;
    }

    Some(DownloadProgressPlan {
        stage_sizes: vec![parse_stage_size(first_size), parse_stage_size(second_size)],
        stage_ids: vec![first_format.to_string(), second_format.to_string()],
    })
}

fn structured_progress(line: &str) -> Option<(String, serde_json::Value)> {
    let payload = line.trim().strip_prefix(crate::ytdlp::PROGRESS_PREFIX)?;
    let (stage, json) = payload.split_once('|')?;
    let data = serde_json::from_str::<serde_json::Value>(json).ok()?;
    Some((stage.to_string(), data))
}

fn byte_label(bytes: f64) -> String {
    if bytes >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} GiB", bytes / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024.0 * 1024.0 {
        format!("{:.2} MiB", bytes / (1024.0 * 1024.0))
    } else if bytes >= 1024.0 {
        format!("{:.1} KiB", bytes / 1024.0)
    } else {
        format!("{bytes:.0} B")
    }
}

fn time_label(seconds: u64) -> String {
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

fn is_download_format(value: &str) -> bool {
    !value.is_empty() && !value.eq_ignore_ascii_case("na")
}

fn parse_stage_size(value: &str) -> Option<u64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|size| size.is_finite() && *size >= 1.0 && *size < u64::MAX as f64)
        .map(|size| size.round() as u64)
}

fn json_byte_count(value: &serde_json::Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_f64()
            .filter(|bytes| bytes.is_finite() && *bytes >= 0.0 && *bytes < u64::MAX as f64)
            .map(|bytes| bytes.round() as u64)
    })
}

pub fn parse_progress_line(line: &str) -> Option<DownloadProgressEvent> {
    let line = line.trim();

    if let Some((_, data)) = structured_progress(line) {
        let number = |key: &str| {
            data.get(key)
                .and_then(|value| value.as_f64())
                .filter(|value| value.is_finite() && *value >= 0.0)
        };
        let total = number("total_bytes")
            .filter(|value| *value > 0.0)
            .or_else(|| number("total_bytes_estimate").filter(|value| *value > 0.0));
        let percent = if data.get("status").and_then(|value| value.as_str()) == Some("finished") {
            Some(100.0)
        } else {
            number("downloaded_bytes")
                .zip(total)
                .map(|(downloaded, total)| (downloaded / total * 100.0).clamp(0.0, 100.0) as f32)
        };
        return Some(DownloadProgressEvent {
            status: "downloading".to_string(),
            percent,
            total: total.map(byte_label),
            speed: number("speed").map(|value| format!("{}/s", byte_label(value))),
            eta: number("eta").map(|value| time_label(value.ceil() as u64)),
            line: Some(line.to_string()),
            message: None,
            file_path: None,
            metrics: Some(DownloadMetrics {
                downloaded_bytes: number("downloaded_bytes").map(|value| value as u64),
                total_bytes: total.map(|value| value as u64),
                total_is_estimate: number("total_bytes").filter(|value| *value > 0.0).is_none(),
                speed_bytes: number("speed").filter(|value| *value > 0.0),
            }),
        });
    }

    static COMPLETED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r"^\[download\]\s+100%\s+of\s+(?P<total>\S+)\s+in\s+(?P<elapsed>\S+)\s+at\s+(?P<speed>\S+)",
    )
    .expect("valid progress regex")
    });
    if let Some(captures) = COMPLETED.captures(line) {
        return Some(DownloadProgressEvent {
            status: "downloading".to_string(),
            percent: Some(100.0),
            total: captures
                .name("total")
                .map(|value| value.as_str().to_string()),
            speed: captures
                .name("speed")
                .map(|value| value.as_str().to_string()),
            eta: Some("00:00".to_string()),
            line: Some(line.to_string()),
            message: None,
            file_path: None,
            metrics: None,
        });
    }

    static PROGRESS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r"^\[download\]\s+(?P<percent>\d+(?:\.\d+)?)%\s+of\s+(?:~\s*)?(?P<total>\S+)(?:\s+at\s+(?P<speed>\S+))?(?:\s+ETA\s+(?P<eta>\S+))?",
    )
    .expect("valid progress regex")
    });

    if let Some(captures) = PROGRESS.captures(line) {
        return Some(DownloadProgressEvent {
            status: "downloading".to_string(),
            percent: captures
                .name("percent")
                .and_then(|value| value.as_str().parse::<f32>().ok()),
            total: captures
                .name("total")
                .map(|value| value.as_str().to_string()),
            speed: captures
                .name("speed")
                .map(|value| value.as_str().to_string()),
            eta: captures.name("eta").map(|value| value.as_str().to_string()),
            line: Some(line.to_string()),
            message: None,
            file_path: None,
            metrics: None,
        });
    }

    if line.starts_with("[download] Destination:") {
        return None;
    }
    if line.contains("Merger") || line.contains("ExtractAudio") {
        return Some(DownloadProgressEvent::status(
            "processing",
            Some(line.to_string()),
        ));
    }

    if line.starts_with("[download]")
        && (line.contains("100%") || line.contains("has already been downloaded"))
    {
        return Some(DownloadProgressEvent {
            status: "downloading".to_string(),
            percent: Some(100.0),
            total: None,
            speed: None,
            eta: None,
            line: Some(line.to_string()),
            message: None,
            file_path: None,
            metrics: None,
        });
    }

    None
}

pub fn parse_output_path(line: &str) -> Option<String> {
    if let Some(path) = line.strip_prefix("__YDLITE_FILE__") {
        return Some(path.trim().to_string());
    }
    let prefixes = [
        "[download] Destination: ",
        "[Merger] Merging formats into ",
        "[ExtractAudio] Destination: ",
    ];

    for prefix in prefixes {
        if let Some(value) = line.strip_prefix(prefix) {
            return Some(trim_path_quotes(value));
        }
    }

    None
}

fn trim_path_quotes(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_fragment_estimates_preserve_progress_and_eta() {
        let mut tracker = DownloadProgressTracker::default();
        tracker.set_plan(parse_download_plan_line("__YDLITE_PLAN__video|NA|audio|200.0").unwrap());
        let mut event = parse_progress_line(r#"__YDLITE_PROGRESS__video|{"status":"downloading","downloaded_bytes":500,"total_bytes_estimate":1000.0,"speed":250,"eta":2}"#).unwrap();
        tracker.apply(&mut event);
        assert!((event.percent.unwrap() - 41.666668).abs() < 0.001);
        assert_eq!(event.eta.as_deref(), Some("0:03"));
        assert_eq!(event.metrics.as_ref().unwrap().total_bytes, Some(1200));
        assert!(parse_stage_size("NaN").is_none());
        assert!(parse_stage_size("-1").is_none());
    }

    #[test]
    fn structured_progress_uses_bytes_and_stream_identity() {
        let mut tracker = DownloadProgressTracker::default();
        tracker.set_plan(parse_download_plan_line("__YDLITE_PLAN__video|800|audio|200").unwrap());
        let mut video = parse_progress_line(r#"__YDLITE_PROGRESS__video|{"status":"downloading","downloaded_bytes":400,"total_bytes":800,"speed":200,"eta":2}"#).unwrap();
        tracker.apply(&mut video);
        assert_eq!(video.percent, Some(40.0));
        assert_eq!(video.speed.as_deref(), Some("200 B/s"));
        assert_eq!(video.total.as_deref(), Some("1000 B"));
        assert_eq!(video.eta.as_deref(), Some("0:03"));
        assert_eq!(video.metrics.as_ref().unwrap().downloaded_bytes, Some(400));
        // Resumed or short audio can first report well above 5%, with no video 100% event.
        let mut audio = parse_progress_line(r#"__YDLITE_PROGRESS__audio|{"status":"downloading","downloaded_bytes":80,"total_bytes":200,"speed":60,"eta":2}"#).unwrap();
        tracker.apply(&mut audio);
        assert_eq!(audio.percent, Some(88.0));
        assert_eq!(audio.total.as_deref(), Some("1000 B"));
        assert_eq!(audio.eta.as_deref(), Some("0:02"));
        assert_eq!(audio.metrics.as_ref().unwrap().downloaded_bytes, Some(880));
        assert!(!audio.metrics.as_ref().unwrap().total_is_estimate);
    }

    #[test]
    fn unknown_totals_do_not_invent_a_percentage() {
        let event = parse_progress_line(r#"__YDLITE_PROGRESS__live|{"status":"downloading","downloaded_bytes":800,"speed":200}"#).unwrap();
        assert_eq!(event.percent, None);
        assert_eq!(event.speed.as_deref(), Some("200 B/s"));
        let mut tracker = DownloadProgressTracker::default();
        tracker.set_plan(parse_download_plan_line("__YDLITE_PLAN__video|NA|audio|NA").unwrap());
        let mut video = parse_progress_line(r#"__YDLITE_PROGRESS__video|{"status":"downloading","downloaded_bytes":400,"total_bytes":800}"#).unwrap();
        tracker.apply(&mut video);
        assert_eq!(video.percent, None);
        assert_eq!(video.total, None);
        assert_eq!(video.eta, None);
        assert_eq!(video.metrics.as_ref().unwrap().downloaded_bytes, Some(400));
    }

    #[test]
    fn revised_size_estimates_and_late_plan_remain_truthful() {
        let mut tracker = DownloadProgressTracker::default();
        let mut first = parse_progress_line(r#"__YDLITE_PROGRESS__video|{"status":"downloading","downloaded_bytes":700,"total_bytes_estimate":1000}"#).unwrap();
        tracker.apply(&mut first);
        assert_eq!(first.percent, Some(70.0));
        let mut revised = parse_progress_line(r#"__YDLITE_PROGRESS__video|{"status":"downloading","downloaded_bytes":800,"total_bytes_estimate":2000}"#).unwrap();
        tracker.apply(&mut revised);
        assert_eq!(revised.percent, Some(40.0));
        tracker.set_plan(parse_download_plan_line("__YDLITE_PLAN__video|2000|audio|500").unwrap());
        tracker.apply(&mut revised);
        assert_eq!(revised.percent, Some(32.0));
        assert!(parse_progress_line("[download] Destination: clip.mp4").is_none());
    }

    #[test]
    fn parses_download_progress_with_total_speed_and_eta() {
        let event = parse_progress_line("[download] 42.3% of 128.50MiB at 3.12MiB/s ETA 00:24")
            .expect("expected progress event");

        assert_eq!(event.status, "downloading");
        assert_eq!(event.percent, Some(42.3));
        assert_eq!(event.total.as_deref(), Some("128.50MiB"));
        assert_eq!(event.speed.as_deref(), Some("3.12MiB/s"));
        assert_eq!(event.eta.as_deref(), Some("00:24"));
    }

    #[test]
    fn ignores_non_progress_lines() {
        assert!(parse_progress_line("WARNING: something minor").is_none());
    }

    #[test]
    fn marks_post_processing_lines() {
        let event = parse_progress_line("[Merger] Merging formats into file.mp4")
            .expect("expected processing event");

        assert_eq!(event.status, "processing");
    }

    #[test]
    fn parses_hls_fragment_progress_with_approximate_total() {
        let event = parse_progress_line(
            "[download] 100.0% of ~   2.77MiB at    1.67MiB/s ETA 00:00 (frag 4/4)",
        )
        .expect("expected progress event");

        assert_eq!(event.percent, Some(100.0));
        assert_eq!(event.total.as_deref(), Some("2.77MiB"));
        assert_eq!(event.speed.as_deref(), Some("1.67MiB/s"));
        assert_eq!(event.eta.as_deref(), Some("00:00"));
    }

    #[test]
    fn parses_final_download_line_with_speed_and_size() {
        let event = parse_progress_line("[download] 100% of    2.77MiB in 00:00:01 at 2.02MiB/s")
            .expect("expected progress event");

        assert_eq!(event.percent, Some(100.0));
        assert_eq!(event.total.as_deref(), Some("2.77MiB"));
        assert_eq!(event.speed.as_deref(), Some("2.02MiB/s"));
        assert_eq!(event.eta.as_deref(), Some("00:00"));
    }

    #[test]
    fn parses_download_destination_path() {
        let path = parse_output_path("[download] Destination: D:\\Videos\\clip.mp4")
            .expect("expected output path");

        assert_eq!(path, "D:\\Videos\\clip.mp4");
    }

    #[test]
    fn parses_merged_output_path() {
        let path = parse_output_path("[Merger] Merging formats into \"D:\\Videos\\clip.mp4\"")
            .expect("expected output path");

        assert_eq!(path, "D:\\Videos\\clip.mp4");
    }

    #[test]
    fn parses_two_stage_download_plan_without_urls() {
        let plan = parse_download_plan_line("__YDLITE_PLAN__137|800|140|200")
            .expect("expected a two-stage plan");

        assert_eq!(plan.stage_sizes, vec![Some(800), Some(200)]);
        assert!(parse_download_plan_line("__YDLITE_PLAN__NA|NA|NA|NA").is_none());
    }

    #[test]
    fn aggregates_multi_stage_progress_without_regressing() {
        let mut tracker = DownloadProgressTracker::default();
        tracker.set_plan(DownloadProgressPlan {
            stage_sizes: vec![Some(800), Some(200)],
            stage_ids: vec!["video".to_string(), "audio".to_string()],
        });

        let mut event = DownloadProgressEvent {
            status: "downloading".to_string(),
            percent: Some(50.0),
            total: None,
            speed: None,
            eta: None,
            line: None,
            message: None,
            file_path: None,
            metrics: None,
        };
        tracker.apply(&mut event);
        assert_eq!(event.percent, Some(40.0));

        event.percent = Some(100.0);
        tracker.apply(&mut event);
        assert_eq!(event.percent, Some(80.0));

        event.percent = Some(0.0);
        tracker.apply(&mut event);
        assert_eq!(event.percent, Some(80.0));

        event.percent = Some(50.0);
        tracker.apply(&mut event);
        assert_eq!(event.percent, Some(90.0));

        event.percent = Some(100.0);
        tracker.apply(&mut event);
        assert_eq!(event.percent, Some(IN_PROGRESS_MAX_PERCENT));
    }

    #[test]
    fn accepts_corrected_estimates_without_freezing_progress() {
        let mut tracker = DownloadProgressTracker::default();
        let mut event = DownloadProgressEvent {
            status: "downloading".to_string(),
            percent: Some(70.0),
            total: None,
            speed: None,
            eta: None,
            line: None,
            message: None,
            file_path: None,
            metrics: None,
        };
        tracker.apply(&mut event);

        event.percent = Some(55.0);
        tracker.apply(&mut event);

        assert_eq!(event.percent, Some(55.0));
    }
}
