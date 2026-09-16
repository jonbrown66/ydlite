use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

const PROGRESS_PLAN_PREFIX: &str = "__YDLITE_PLAN__";
const IN_PROGRESS_MAX_PERCENT: f32 = 99.8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DownloadProgressPlan {
    stage_sizes: Vec<Option<u64>>,
}

#[derive(Debug, Default)]
pub(crate) struct DownloadProgressTracker {
    plan: Option<DownloadProgressPlan>,
    stage_index: usize,
    stage_high_water_percent: f32,
    last_raw_percent: Option<f32>,
    last_emitted_percent: f32,
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
        }
    }
}

impl DownloadProgressTracker {
    pub fn set_plan(&mut self, plan: DownloadProgressPlan) {
        self.plan = Some(plan);
        self.stage_index = 0;
        self.stage_high_water_percent = 0.0;
        self.last_raw_percent = None;
    }

    pub fn apply(&mut self, event: &mut DownloadProgressEvent) {
        if event.status != "downloading" {
            return;
        }

        let Some(raw_percent) = event.percent.filter(|percent| percent.is_finite()) else {
            return;
        };
        let raw_percent = raw_percent.clamp(0.0, 100.0);

        if let Some(plan) = &self.plan {
            if self.stage_index + 1 < plan.stage_sizes.len()
                && self
                    .last_raw_percent
                    .is_some_and(|previous| previous >= 95.0)
                && raw_percent <= 5.0
            {
                self.stage_index += 1;
                self.stage_high_water_percent = 0.0;
                self.last_raw_percent = None;
            }

            self.stage_high_water_percent = self.stage_high_water_percent.max(raw_percent);
            self.last_raw_percent = Some(raw_percent);
            let aggregate_percent =
                plan.aggregate_percent(self.stage_index, self.stage_high_water_percent);
            self.last_emitted_percent = self
                .last_emitted_percent
                .max(aggregate_percent.min(IN_PROGRESS_MAX_PERCENT));
        } else {
            self.last_emitted_percent = self
                .last_emitted_percent
                .max(raw_percent.min(IN_PROGRESS_MAX_PERCENT));
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
    })
}

fn is_download_format(value: &str) -> bool {
    !value.is_empty() && !value.eq_ignore_ascii_case("na")
}

fn parse_stage_size(value: &str) -> Option<u64> {
    value.parse::<u64>().ok().filter(|size| *size > 0)
}

pub fn parse_progress_line(line: &str) -> Option<DownloadProgressEvent> {
    let line = line.trim();

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
        });
    }

    if line.contains("Merger") || line.contains("ExtractAudio") || line.contains("Destination") {
        return Some(DownloadProgressEvent::status(
            "processing",
            Some(line.to_string()),
        ));
    }

    if line.contains("100%") || line.contains("has already been downloaded") {
        return Some(DownloadProgressEvent {
            status: "downloading".to_string(),
            percent: Some(100.0),
            total: None,
            speed: None,
            eta: None,
            line: Some(line.to_string()),
            message: None,
            file_path: None,
        });
    }

    None
}

pub fn parse_output_path(line: &str) -> Option<String> {
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
    fn keeps_single_stage_progress_monotonic_when_no_plan_is_available() {
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
        };
        tracker.apply(&mut event);

        event.percent = Some(55.0);
        tracker.apply(&mut event);

        assert_eq!(event.percent, Some(70.0));
    }
}
