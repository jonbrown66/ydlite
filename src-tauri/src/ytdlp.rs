use crate::commands::DownloadMode;

pub const OUTPUT_TEMPLATE: &str = "%(title).200B.%(ext)s";
const GENERIC_IMPERSONATE_ARGS: [&str; 2] = ["--extractor-args", "generic:impersonate"];
const BEST_COMPATIBLE_FORMAT: &str =
    "bv*[ext=mp4]+ba[ext=m4a]/bv*[vcodec^=avc1]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b";
const P720_COMPATIBLE_FORMAT: &str = "bv*[ext=mp4][height<=720]+ba[ext=m4a]/bv*[vcodec^=avc1][height<=720]+ba[ext=m4a]/b[ext=mp4][height<=720]/bv*[height<=720]+ba/b[height<=720]/best";

#[derive(Clone, Debug, Default)]
pub struct YtdlpOptions {
    pub cookies_from_browser: Option<String>,
    pub cookies_file: Option<String>,
    pub headers: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SiteProfile {
    Bilibili,
    Youtube,
    Twitter,
    Instagram,
    Generic,
}

pub fn site_profile(url: &str) -> SiteProfile {
    let lower = url.to_ascii_lowercase();
    if lower.contains("bilibili.com") || lower.contains("b23.tv") {
        SiteProfile::Bilibili
    } else if lower.contains("youtube.com") || lower.contains("youtu.be") {
        SiteProfile::Youtube
    } else if lower.contains("twitter.com") || lower.contains("x.com") {
        SiteProfile::Twitter
    } else if lower.contains("instagram.com") {
        SiteProfile::Instagram
    } else {
        SiteProfile::Generic
    }
}

pub fn parse_args(url: &str, flat_playlist: bool, options: &YtdlpOptions) -> Vec<String> {
    let mut args = vec![
        "-J".to_string(),
        "--encoding".to_string(),
        "utf-8".to_string(),
    ];
    if flat_playlist {
        args.push("--flat-playlist".to_string());
    } else {
        args.push("--no-playlist".to_string());
    }
    args.push("--no-warnings".to_string());
    args.extend(GENERIC_IMPERSONATE_ARGS.into_iter().map(str::to_string));
    args.extend(option_args(options));
    args.push(url.to_string());
    args
}

pub fn download_args(
    mode: &DownloadMode,
    custom_format: Option<&str>,
    dir: &str,
    url: &str,
    options: &YtdlpOptions,
) -> Vec<String> {
    let mut args = Vec::new();
    args.extend(mode_args(mode, custom_format));
    args.extend(
        [
            "--no-playlist",
            "--newline",
            "-N",
            "4",
            "--no-warnings",
            "--windows-filenames",
            "--restrict-filenames",
            "--encoding",
            "utf-8",
            "-o",
            OUTPUT_TEMPLATE,
            "-P",
            dir,
        ]
        .into_iter()
        .map(str::to_string),
    );
    args.extend(GENERIC_IMPERSONATE_ARGS.into_iter().map(str::to_string));
    args.extend(option_args(options));
    args.push(url.to_string());
    args
}

fn option_args(options: &YtdlpOptions) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(browser) = &options.cookies_from_browser {
        args.push("--cookies-from-browser".to_string());
        args.push(browser.clone());
    }
    if let Some(file) = &options.cookies_file {
        args.push("--cookies".to_string());
        args.push(file.clone());
    }
    for (name, value) in &options.headers {
        args.push("--add-header".to_string());
        args.push(format!("{name}:{value}"));
    }
    args
}

fn mode_args(mode: &DownloadMode, custom_format: Option<&str>) -> Vec<String> {
    match mode {
        DownloadMode::Best => vec![
            "-f".to_string(),
            BEST_COMPATIBLE_FORMAT.to_string(),
            "--merge-output-format".to_string(),
            "mp4".to_string(),
        ],
        DownloadMode::P720 => vec![
            "-f".to_string(),
            P720_COMPATIBLE_FORMAT.to_string(),
            "--merge-output-format".to_string(),
            "mp4".to_string(),
        ],
        DownloadMode::Mp3 => vec![
            "-x".to_string(),
            "--audio-format".to_string(),
            "mp3".to_string(),
            "--audio-quality".to_string(),
            "0".to_string(),
        ],
        DownloadMode::Custom => {
            if let Some(fmt) = custom_format {
                vec![
                    "-f".to_string(),
                    format!("{}+ba[ext=m4a]/{}+bestaudio/best/{}", fmt, fmt, fmt),
                    "--merge-output-format".to_string(),
                    "mp4".to_string(),
                ]
            } else {
                vec![
                    "-f".to_string(),
                    BEST_COMPATIBLE_FORMAT.to_string(),
                    "--merge-output-format".to_string(),
                    "mp4".to_string(),
                ]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_always_disable_playlist() {
        let args = parse_args("https://example.com/video", false, &YtdlpOptions::default());

        assert!(args.iter().any(|arg| arg == "--no-playlist"));
        assert!(args.iter().any(|arg| arg == "--no-warnings"));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--extractor-args", "generic:impersonate"]));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://example.com/video")
        );
    }

    #[test]
    fn download_args_match_single_video_windows_safe_defaults() {
        let args = download_args(
            &DownloadMode::Best,
            None,
            "D:\\Downloads",
            "https://example.com/video",
            &YtdlpOptions::default(),
        );

        assert!(args.iter().any(|arg| arg == "--no-playlist"));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["-f", BEST_COMPATIBLE_FORMAT]));
        assert!(args.iter().any(|arg| arg == "--newline"));
        assert!(args.windows(2).any(|pair| pair == ["-N", "4"]));
        assert!(args.iter().any(|arg| arg == "--no-warnings"));
        assert!(args.iter().any(|arg| arg == "--windows-filenames"));
        assert!(args.iter().any(|arg| arg == "--restrict-filenames"));
        assert!(args.windows(2).any(|pair| pair == ["-P", "D:\\Downloads"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--extractor-args", "generic:impersonate"]));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://example.com/video")
        );
    }

    #[test]
    fn args_include_cookie_and_header_options() {
        let args = parse_args(
            "https://www.bilibili.com/video/BV1xx",
            false,
            &YtdlpOptions {
                cookies_from_browser: Some("firefox".to_string()),
                cookies_file: None,
                headers: vec![(
                    "Referer".to_string(),
                    "https://www.bilibili.com/".to_string(),
                )],
            },
        );

        assert!(args
            .windows(2)
            .any(|pair| pair == ["--cookies-from-browser", "firefox"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--add-header", "Referer:https://www.bilibili.com/"]));
    }

    #[test]
    fn custom_video_format_prefers_m4a_audio_for_mp4_compatibility() {
        let args = download_args(
            &DownloadMode::Custom,
            Some("137"),
            "D:\\Downloads",
            "https://example.com/video",
            &YtdlpOptions::default(),
        );

        assert!(args.windows(2).any(|pair| {
            pair == [
                "-f",
                "137+ba[ext=m4a]/137+bestaudio/best/137",
            ]
        }));
    }
}
