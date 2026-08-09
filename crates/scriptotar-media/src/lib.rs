use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const SIDECAR_PROTOCOL_VERSION: u16 = 1;
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "mov", "webm", "m4v", "avi"];
const TRANSCRIPTION_MODELS: &[&str] = &["small", "medium", "turbo", "large-v3"];

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MediaError {
    #[error("unsupported local media extension")]
    UnsupportedExtension,
    #[error("unsupported transcription model: {0}")]
    UnsupportedModel(String),
    #[error("sidecar protocol error: {0}")]
    Protocol(String),
    #[error("media tool failed: {0}")]
    Tool(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SidecarInputKind {
    Url,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidecarInput {
    pub kind: SidecarInputKind,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidecarOutput {
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidecarOptions {
    pub model: String,
    pub device: String,
    pub language: String,
    pub quality: String,
    pub cookies_browser: String,
    pub max_duration_seconds: u32,
    pub copy_source: bool,
    pub translate: bool,
    pub batched: bool,
    pub batch_size: u32,
    pub keep_failed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SidecarCommand {
    Ping {
        protocol: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    Transcribe {
        protocol: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        job_id: String,
        input: SidecarInput,
        output: SidecarOutput,
        options: SidecarOptions,
    },
    Cancel {
        protocol: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        job_id: String,
    },
    Shutdown {
        protocol: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SidecarCapabilities {
    pub sidecar_version: String,
    pub protocol_versions: Vec<u16>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SidecarErrorPayload {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default)]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SidecarSourceResult {
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub source_url: Option<String>,
    pub duration_seconds: Option<f64>,
    pub extractor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SidecarTranscriptResult {
    pub text: String,
    pub clean_text: String,
    pub segments: Value,
    pub words: Value,
    pub language: Option<String>,
    pub language_probability: Option<f64>,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SidecarArtifacts {
    pub text: Option<String>,
    pub clean_text: Option<String>,
    pub timestamp_text: Option<String>,
    pub srt: Option<String>,
    pub vtt: Option<String>,
    pub json: Option<String>,
    pub media: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SidecarResult {
    pub source: SidecarSourceResult,
    pub transcript: Box<SidecarTranscriptResult>,
    pub artifacts: SidecarArtifacts,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SidecarEvent {
    Ready {
        protocol: u16,
        capabilities: SidecarCapabilities,
    },
    Pong {
        protocol: u16,
        #[serde(default)]
        request_id: Option<String>,
        sidecar_version: String,
    },
    Accepted {
        protocol: u16,
        job_id: String,
        #[serde(default)]
        request_id: Option<String>,
    },
    JobStarted {
        protocol: u16,
        job_id: String,
    },
    Progress {
        protocol: u16,
        job_id: String,
        stage: String,
        #[serde(default)]
        percent: Option<f32>,
        #[serde(default)]
        message: Option<String>,
    },
    Result {
        protocol: u16,
        job_id: String,
        result: SidecarResult,
    },
    Error {
        protocol: u16,
        #[serde(default)]
        job_id: Option<String>,
        error: SidecarErrorPayload,
        #[serde(default)]
        request_id: Option<String>,
    },
    Cancelled {
        protocol: u16,
        job_id: String,
        reason: String,
    },
    Shutdown {
        protocol: u16,
        #[serde(default)]
        request_id: Option<String>,
    },
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
    fn transcribe_command_matches_python_protocol_shape() {
        let command = SidecarCommand::Transcribe {
            protocol: SIDECAR_PROTOCOL_VERSION,
            request_id: None,
            job_id: "abc".to_owned(),
            input: SidecarInput {
                kind: SidecarInputKind::File,
                value: "/tmp/a.mp4".to_owned(),
            },
            output: SidecarOutput {
                root: "/tmp/out".to_owned(),
            },
            options: SidecarOptions {
                model: "medium".to_owned(),
                device: "auto".to_owned(),
                language: "auto".to_owned(),
                quality: "720p".to_owned(),
                cookies_browser: "none".to_owned(),
                max_duration_seconds: 3600,
                copy_source: true,
                translate: false,
                batched: false,
                batch_size: 8,
                keep_failed: false,
            },
        };
        let value = serde_json::to_value(command).unwrap();
        assert_eq!(value["type"], "transcribe");
        assert_eq!(value["job_id"], "abc");
        assert_eq!(value["input"]["kind"], "file");
        assert!(value.get("request").is_none());
    }

    #[test]
    fn progress_and_nested_error_events_parse() {
        let progress: SidecarEvent = serde_json::from_str(
            r#"{"protocol":1,"type":"progress","job_id":"abc","stage":"transcribing","percent":50.0,"message":"halfway"}"#,
        )
        .unwrap();
        assert!(matches!(progress, SidecarEvent::Progress { .. }));
        let error: SidecarEvent = serde_json::from_str(
            r#"{"protocol":1,"type":"error","job_id":"abc","error":{"code":"X","message":"bad","retryable":false}}"#,
        )
        .unwrap();
        assert!(matches!(error, SidecarEvent::Error { .. }));
    }

    #[test]
    fn ready_capabilities_are_forward_compatible() {
        let ready: SidecarEvent = serde_json::from_str(
            r#"{"protocol":1,"type":"ready","capabilities":{"sidecar_version":"0.1.0","protocol_versions":[1],"features":{"cancellation":true}}}"#,
        )
        .unwrap();
        match ready {
            SidecarEvent::Ready { capabilities, .. } => {
                assert_eq!(capabilities.protocol_versions, vec![1]);
                assert!(capabilities.extra.contains_key("features"));
            }
            _ => panic!("expected ready"),
        }
    }

    #[test]
    fn local_media_extension_and_model_are_validated() {
        let policy = MediaPolicy;
        assert!(policy
            .validate_local_input(Path::new("/tmp/video.mp4"))
            .is_ok());
        assert_eq!(
            policy.validate_local_input(Path::new("/tmp/payload.sh")),
            Err(MediaError::UnsupportedExtension)
        );
        assert!(policy.validate_model("large-v3").is_ok());
        assert!(matches!(
            policy.validate_model("../../model"),
            Err(MediaError::UnsupportedModel(_))
        ));
    }
}
