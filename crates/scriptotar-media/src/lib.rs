use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const SIDECAR_PROTOCOL_VERSION: u16 = 1;
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "mov", "webm", "m4v", "avi"];
const TRANSCRIPTION_MODELS: &[&str] = &["small", "medium", "turbo", "large-v3"];

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MediaError {
    #[error("unsupported local media extension")]
    UnsupportedExtension,
    #[error("unsupported transcription model: {0}")]
    UnsupportedModel(String),
    #[error("media tool failed: {0}")]
    Tool(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadRequest {
    pub job_id: Uuid,
    pub url: String,
    pub output_directory: PathBuf,
    pub quality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeRequest {
    pub job_id: Uuid,
    pub input: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptionRequest {
    pub job_id: Uuid,
    pub input: PathBuf,
    pub output_directory: PathBuf,
    pub model: String,
    pub device: String,
    pub language: String,
    pub word_timestamps: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SidecarCommand {
    Ping {
        protocol: u16,
    },
    Transcribe {
        protocol: u16,
        request: TranscriptionRequest,
    },
    Cancel {
        protocol: u16,
        job_id: Uuid,
    },
    Shutdown {
        protocol: u16,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SidecarEvent {
    Ready {
        protocol: u16,
        sidecar_version: String,
        capabilities: Vec<String>,
    },
    Progress {
        protocol: u16,
        job_id: Uuid,
        stage: String,
        fraction: Option<f32>,
        message: Option<String>,
    },
    Result {
        protocol: u16,
        job_id: Uuid,
        transcript_json: String,
    },
    Error {
        protocol: u16,
        job_id: Option<Uuid>,
        code: String,
        message: String,
        retryable: bool,
    },
}

pub trait YtDlpRunner: Send + Sync {
    fn download(&self, request: &DownloadRequest) -> Result<PathBuf, MediaError>;
}

pub trait FfmpegRunner: Send + Sync {
    fn probe(&self, request: &ProbeRequest) -> Result<(), MediaError>;
}

pub trait TranscriptionSidecar: Send + Sync {
    fn transcribe(&self, request: &TranscriptionRequest) -> Result<(), MediaError>;
    fn cancel(&self, job_id: Uuid) -> Result<(), MediaError>;
    fn shutdown(&self) -> Result<(), MediaError>;
}

#[derive(Debug, Clone, Default)]
pub struct MediaPolicy;

impl MediaPolicy {
    pub fn validate_local_input(&self, path: &Path) -> Result<(), MediaError> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .ok_or(MediaError::UnsupportedExtension)?;
        if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
            Ok(())
        } else {
            Err(MediaError::UnsupportedExtension)
        }
    }

    pub fn validate_model(&self, model: &str) -> Result<(), MediaError> {
        if TRANSCRIPTION_MODELS.contains(&model) {
            Ok(())
        } else {
            Err(MediaError::UnsupportedModel(model.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_commands_are_versioned() {
        let command = SidecarCommand::Ping {
            protocol: SIDECAR_PROTOCOL_VERSION,
        };
        let json = serde_json::to_string(&command).unwrap();
        assert!(json.contains("\"protocol\":1"));
        assert!(json.contains("\"type\":\"ping\""));
    }

    #[test]
    fn local_media_extension_is_validated_before_tools_run() {
        let policy = MediaPolicy;
        assert!(policy
            .validate_local_input(Path::new("/tmp/video.mp4"))
            .is_ok());
        assert_eq!(
            policy.validate_local_input(Path::new("/tmp/payload.sh")),
            Err(MediaError::UnsupportedExtension)
        );
    }

    #[test]
    fn model_names_are_explicit() {
        let policy = MediaPolicy;
        assert!(policy.validate_model("large-v3").is_ok());
        assert!(matches!(
            policy.validate_model("../../model"),
            Err(MediaError::UnsupportedModel(_))
        ));
    }
}
