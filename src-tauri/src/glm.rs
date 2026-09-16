use crate::errors::AppError;
use crate::openai_compatible::OpenAiCompatibleClient;
use std::fs;
use std::path::Path;

pub const API_BASE: &str = "https://api.z.ai/api/paas/v4";
pub const MODEL: &str = "glm-4.7-flash";

const KEYRING_SERVICE: &str = "com.ydlite.desktop";
const KEYRING_USER: &str = "glm-api-key";
const ENV_KEY: &str = "GLM_API_KEY";

pub fn client() -> Result<OpenAiCompatibleClient, AppError> {
    OpenAiCompatibleClient::new(API_BASE, load_api_key()?, MODEL)
}

#[cfg(target_os = "windows")]
pub fn save_api_key(value: Option<&str>) -> Result<(), AppError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| AppError::user("无法访问 Windows 凭据管理器。", error.to_string()))?;
    match value.map(str::trim) {
        Some(value) if !value.is_empty() => entry
            .set_password(value)
            .map_err(|error| AppError::user("无法保存 GLM API Key。", error.to_string())),
        _ => {
            let _ = entry.delete_credential();
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
pub fn load_api_key() -> Result<String, AppError> {
    if let Some(value) = load_env_api_key() {
        return Ok(value);
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| AppError::user("无法访问 Windows 凭据管理器。", error.to_string()))?;
    entry.get_password().map_err(|error| {
        AppError::user(
            "尚未配置 GLM API Key。",
            format!("Credential unavailable: {error}"),
        )
    })
}

fn load_env_api_key() -> Option<String> {
    if let Some(value) = non_empty(std::env::var(ENV_KEY).ok()) {
        return Some(value);
    }

    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            candidates.push(parent.join(".env"));
        }
    }
    if let Ok(current_dir) = std::env::current_dir() {
        let mut directory = Some(current_dir.as_path());
        while let Some(path) = directory {
            candidates.push(path.join(".env"));
            directory = path.parent();
        }
    }

    candidates.into_iter().find_map(|path| read_env_key(&path))
}

fn read_env_key(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    content.lines().find_map(parse_env_line)
}

fn parse_env_line(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (name, raw_value) = line.split_once('=')?;
    if name.trim() != ENV_KEY {
        return None;
    }
    let value = raw_value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
        .trim();
    non_empty(Some(value.to_string()))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}

#[cfg(not(target_os = "windows"))]
pub fn save_api_key(_value: Option<&str>) -> Result<(), AppError> {
    Err(AppError::user(
        "当前平台暂不支持安全保存 GLM Key。",
        "Windows Credential Manager is required",
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn load_api_key() -> Result<String, AppError> {
    if let Some(value) = load_env_api_key() {
        return Ok(value);
    }
    Err(AppError::user(
        "当前平台暂不支持 GLM Key。",
        "Windows Credential Manager is required",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_quoted_env_values() {
        assert_eq!(parse_env_line("GLM_API_KEY=abc"), Some("abc".into()));
        assert_eq!(parse_env_line("GLM_API_KEY=\"abc\""), Some("abc".into()));
        assert_eq!(parse_env_line("OTHER_KEY=abc"), None);
        assert_eq!(parse_env_line("# GLM_API_KEY=abc"), None);
    }
}
