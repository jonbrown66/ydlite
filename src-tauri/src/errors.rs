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
