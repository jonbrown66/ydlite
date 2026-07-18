use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{message}")]
    User { message: String, detail: String },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub message: String,
    pub detail: String,
}

impl AppError {
    pub fn user(message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::User {
            message: message.into(),
            detail: detail.into(),
        }
    }

    pub fn is_rate_limited(&self) -> bool {
        matches!(
            self,
            Self::User { detail, .. }
                if detail.contains("HTTP 429") || detail.contains("429 Too Many Requests")
        )
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let payload = match self {
            AppError::User { message, detail } => ErrorPayload {
                message: message.clone(),
                detail: detail.clone(),
            },
            AppError::Io(error) => ErrorPayload {
                message: "本地命令执行失败，请确认依赖已安装并可在命令行中运行。".to_string(),
                detail: error.to_string(),
            },
            AppError::Json(error) => ErrorPayload {
                message: "视频信息解析失败，请查看详细日志。".to_string(),
                detail: error.to_string(),
            },
        };
        payload.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_only_http_rate_limit_errors() {
        assert!(AppError::user(
            "Gemini 已达到当前项目的调用或费用限制。",
            "HTTP 429 Too Many Requests"
        )
        .is_rate_limited());
        assert!(!AppError::user(
            "Gemini 拒绝了请求，请检查模型与参数。",
            "HTTP 400 Bad Request"
        )
        .is_rate_limited());
    }
}
