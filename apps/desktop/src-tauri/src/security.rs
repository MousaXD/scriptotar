use std::{fs, path::Path};

use crate::dto::{AiPromptInput, ResearchQuery};

const MAX_PROJECT_NAME_BYTES: usize = 256;
const MAX_PATH_BYTES: usize = 4096;
const MAX_URL_BYTES: usize = 4096;
const MAX_MODEL_BYTES: usize = 256;
const MAX_TASK_BYTES: usize = 512;
const MAX_AI_CONTEXT_BYTES: usize = 2 * 1024 * 1024;
const MAX_AI_FIELD_BYTES: usize = 16 * 1024;
const MAX_API_KEY_BYTES: usize = 16 * 1024;
const MAX_RESEARCH_IDS: usize = 200;
const MAX_RESEARCH_ID_BYTES: usize = 256;

pub fn validate_project_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("project name cannot be empty".to_owned());
    }
    validate_text_size("project name", trimmed, MAX_PROJECT_NAME_BYTES)
}

pub fn validated_local_media_path(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    validate_text_size("local media path", raw, MAX_PATH_BYTES)?;
    if raw.contains('\0') {
        return Err("local media path contains an invalid NUL byte".to_owned());
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err("local media path must be absolute".to_owned());
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| "local media path could not be resolved".to_owned())?;
    let metadata = fs::metadata(&canonical)
        .map_err(|_| "local media path could not be inspected".to_owned())?;
    if !metadata.is_file() {
        return Err("local media path must point to a regular file".to_owned());
    }
    Ok(canonical.to_string_lossy().into_owned())
}

pub fn validate_url_argument(label: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }
    validate_text_size(label, trimmed, MAX_URL_BYTES)?;
    if trimmed.chars().any(char::is_control) {
        return Err(format!("{label} contains control characters"));
    }
    Ok(())
}

pub fn validate_research_query(query: &ResearchQuery) -> Result<(), String> {
    validate_url_argument("research profile URL", &query.profile_url)?;
    if !(1..=200).contains(&query.limit) {
        return Err("research limit must be between 1 and 200".to_owned());
    }
    Ok(())
}

pub fn validate_research_ids(ids: &[String]) -> Result<(), String> {
    if ids.is_empty() {
        return Err("select at least one research item".to_owned());
    }
    if ids.len() > MAX_RESEARCH_IDS {
        return Err(format!(
            "at most {MAX_RESEARCH_IDS} research items can be queued at once"
        ));
    }
    for id in ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err("research item IDs cannot be empty".to_owned());
        }
        validate_text_size("research item ID", trimmed, MAX_RESEARCH_ID_BYTES)?;
        if trimmed.chars().any(char::is_control) {
            return Err("research item ID contains control characters".to_owned());
        }
    }
    Ok(())
}

pub fn validate_ai_input(input: &AiPromptInput) -> Result<(), String> {
    validate_text_size("AI mode", input.mode.trim(), 64)?;
    validate_text_size("AI provider", input.provider.trim(), 128)?;
    validate_text_size("AI model", input.model.trim(), MAX_MODEL_BYTES)?;
    validate_text_size("AI task", input.task.trim(), MAX_TASK_BYTES)?;
    validate_text_size(
        "AI source context",
        &input.source_text,
        MAX_AI_CONTEXT_BYTES,
    )?;
    for (label, value) in [
        ("AI topic", input.topic.as_str()),
        ("AI audience", input.audience.as_str()),
        ("AI duration", input.duration.as_str()),
        ("AI CTA", input.cta.as_str()),
        ("AI voice instructions", input.voice.as_str()),
    ] {
        validate_text_size(label, value, MAX_AI_FIELD_BYTES)?;
    }
    if let Some(base_url) = input.base_url.as_deref() {
        if !base_url.trim().is_empty() {
            validate_url_argument("AI provider endpoint", base_url)?;
        }
    }
    if let Some(api_key) = input.api_key.as_deref() {
        if api_key.len() > MAX_API_KEY_BYTES {
            return Err("API key exceeds the supported size".to_owned());
        }
        if api_key
            .chars()
            .any(|character| matches!(character, '\r' | '\n' | '\0'))
        {
            return Err("API key contains invalid control characters".to_owned());
        }
    }
    Ok(())
}

fn validate_text_size(label: &str, value: &str, max_bytes: usize) -> Result<(), String> {
    if value.len() > max_bytes {
        return Err(format!("{label} exceeds the supported size"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use tempfile::TempDir;

    #[test]
    fn local_media_paths_must_be_absolute_regular_files() {
        assert!(validated_local_media_path("relative.mp4").is_err());
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("video.mp4");
        fs::write(&path, b"fixture").unwrap();
        let validated = validated_local_media_path(path.to_str().unwrap()).unwrap();
        assert_eq!(PathBuf::from(validated), fs::canonicalize(path).unwrap());
    }

    #[test]
    fn rejects_oversized_ipc_payloads() {
        assert!(validate_project_name(&"x".repeat(MAX_PROJECT_NAME_BYTES + 1)).is_err());
        let ids = (0..=MAX_RESEARCH_IDS)
            .map(|index| index.to_string())
            .collect::<Vec<_>>();
        assert!(validate_research_ids(&ids).is_err());
    }
}
