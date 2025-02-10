use arrow_schema::ArrowError;
use std::num::ParseIntError;
use tauri::ipc::InvokeError;
use thiserror::Error;
use tokio::task::JoinError;

pub type AppResult<T> = Result<T, AidenErrors>;

#[derive(Error, Debug)]
pub enum AidenErrors {
    #[error("{0}")]
    StdIo(#[from] std::io::Error),

    #[error("{0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("{0}")]
    Str(&'static str),

    #[error("{0}")]
    String(String),

    #[error("{0}")]
    TaskJoinError(#[from] JoinError),

    #[error("{0}")]
    LoPdfError(#[from] lopdf::Error),

    #[error("{0}")]
    DocxError(#[from] docx_rs::ReaderError),

    #[error("{0}")]
    PromptError(#[from] rig::completion::PromptError),

    #[error("{0}")]
    ParseError(#[from] ParseIntError),

    #[error("{0}")]
    ArrowError(#[from] ArrowError),

    #[error("{0}")]
    CandleError(#[from] candle::Error),

    #[error("{0}")]
    AnyError(#[from] anyhow::Error),

    #[error("{0}")]
    LancedbError(#[from] lancedb::Error),
}

impl From<AidenErrors> for InvokeError {
    fn from(value: AidenErrors) -> Self {
        InvokeError::from_anyhow(value.into())
    }
}
