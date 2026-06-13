use regex::Regex;
use serde::Serialize;

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

pub fn parse_progress_line(line: &str) -> Option<DownloadProgressEvent> {
    let completed = Regex::new(
        r"^\[download\]\s+100%\s+of\s+(?P<total>\S+)\s+in\s+(?P<elapsed>\S+)\s+at\s+(?P<speed>\S+)",
    )
    .ok()?;
    if let Some(captures) = completed.captures(line) {
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

    let progress = Regex::new(
        r"^\[download\]\s+(?P<percent>\d+(?:\.\d+)?)%\s+of\s+(?:~\s*)?(?P<total>\S+)(?:\s+at\s+(?P<speed>\S+))?(?:\s+ETA\s+(?P<eta>\S+))?",
    )
    .ok()?;

    if let Some(captures) = progress.captures(line) {
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
}
