use std::path::PathBuf;

pub fn ytdlp() -> PathBuf {
    first_existing(
        "YDLITE_YTDLP",
        &[
            tools_dir().join(executable_name("yt-dlp")),
            tools_dir().join("yt-dlp").join(executable_name("yt-dlp")),
        ],
        executable_name("yt-dlp"),
    )
}

pub fn ffmpeg() -> PathBuf {
    first_existing(
        "YDLITE_FFMPEG",
        &[
            tools_dir().join(executable_name("ffmpeg")),
            tools_dir().join("ffmpeg").join(executable_name("ffmpeg")),
        ],
        executable_name("ffmpeg"),
    )
}

pub fn tools_dir() -> PathBuf {
    exe_dir().join("tools")
}

pub fn ytdlp_install_path() -> PathBuf {
    tools_dir().join(executable_name("yt-dlp"))
}

pub fn ffmpeg_install_path() -> PathBuf {
    tools_dir().join("ffmpeg").join(executable_name("ffmpeg"))
}

fn first_existing(env_key: &str, candidates: &[PathBuf], fallback: &str) -> PathBuf {
    if let Ok(value) = std::env::var(env_key) {
        let path = PathBuf::from(value);
        if path.is_file() {
            return path;
        }
    }

    for candidate in candidates {
        if candidate.is_file() {
            return candidate.clone();
        }
    }

    PathBuf::from(fallback)
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn executable_name(name: &str) -> &str {
    if cfg!(target_os = "windows") {
        match name {
            "yt-dlp" => "yt-dlp.exe",
            "ffmpeg" => "ffmpeg.exe",
            _ => name,
        }
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_names_are_platform_aware() {
        let expected = if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else {
            "yt-dlp"
        };

        assert_eq!(executable_name("yt-dlp"), expected);
    }
}
