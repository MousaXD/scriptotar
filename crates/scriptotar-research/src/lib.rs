use std::{
    env,
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};

use scriptotar_core::{now_rfc3339, ResearchItem};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

const ALLOWED_DOMAINS: &[&str] = &["instagram.com", "tiktok.com", "youtube.com", "youtu.be"];
const MAX_PROFILE_URL_CHARS: usize = 2_048;
const MAX_PROVIDER_LINE_BYTES: usize = 2 * 1024 * 1024;
const MAX_PROVIDER_ERROR_BYTES: u64 = 64 * 1024;
const MAX_RAW_DESCRIPTION_CHARS: usize = 12_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedResearchUrl(Url);

impl ValidatedResearchUrl {
    pub fn as_url(&self) -> &Url {
        &self.0
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ResearchError {
    #[error("research URL is invalid: {0}")]
    InvalidUrl(String),
    #[error("research URL host is not allowed")]
    HostNotAllowed,
    #[error("research URL must not contain credentials")]
    EmbeddedCredentials,
    #[error("research URL uses a non-standard port")]
    NonStandardPort,
    #[error("research provider output is too large")]
    OutputTooLarge,
    #[error("research provider returned invalid data: {0}")]
    InvalidProviderData(String),
    #[error("research provider failed: {0}")]
    Provider(String),
}

#[derive(Debug, Clone, Default)]
pub struct NetworkPolicy;

impl NetworkPolicy {
    pub fn validate(&self, raw: &str) -> Result<ValidatedResearchUrl, ResearchError> {
        if raw.chars().count() > MAX_PROFILE_URL_CHARS {
            return Err(ResearchError::InvalidUrl("URL is too long".to_owned()));
        }
        let url = Url::parse(raw).map_err(|error| ResearchError::InvalidUrl(error.to_string()))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(ResearchError::InvalidUrl(
                "only http and https are supported".to_owned(),
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(ResearchError::EmbeddedCredentials);
        }
        if url.fragment().is_some() {
            return Err(ResearchError::InvalidUrl(
                "URL fragments are not supported".to_owned(),
            ));
        }
        if let Some(port) = url.port() {
            let standard = matches!((url.scheme(), port), ("http", 80) | ("https", 443));
            if !standard {
                return Err(ResearchError::NonStandardPort);
            }
        }
        let host = url.host_str().ok_or(ResearchError::HostNotAllowed)?;
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        if !ALLOWED_DOMAINS
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
        {
            return Err(ResearchError::HostNotAllowed);
        }
        Ok(ValidatedResearchUrl(url))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchObservation {
    pub source_url: String,
    pub platform: String,
    pub title: Option<String>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
    pub comment_count: Option<i64>,
    pub published_at: Option<String>,
    pub duration_seconds: Option<f64>,
    pub raw_json: Option<String>,
}

pub trait ResearchProvider: Send + Sync {
    fn scan(
        &self,
        profile: &ValidatedResearchUrl,
        limit: u16,
    ) -> Result<Vec<ResearchObservation>, ResearchError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YtDlpCommand {
    program: PathBuf,
    prefix_args: Vec<String>,
}

impl YtDlpCommand {
    pub fn executable(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            prefix_args: Vec::new(),
        }
    }

    pub fn python_module(python: impl Into<PathBuf>) -> Self {
        Self {
            program: python.into(),
            prefix_args: vec!["-m".to_owned(), "yt_dlp".to_owned()],
        }
    }

    pub fn from_environment() -> Self {
        if let Some(executable) = env::var_os("SCRIPTOTAR_YTDLP_EXECUTABLE") {
            return Self::executable(executable);
        }
        if let Some(python) = env::var_os("SCRIPTOTAR_SIDECAR_PYTHON") {
            return Self::python_module(python);
        }
        Self::executable("yt-dlp")
    }
}

#[derive(Debug, Clone)]
pub struct YtDlpProvider {
    command: YtDlpCommand,
    cookie_browser: Option<String>,
}

impl Default for YtDlpProvider {
    fn default() -> Self {
        Self::new(YtDlpCommand::from_environment())
    }
}

impl YtDlpProvider {
    pub fn new(command: YtDlpCommand) -> Self {
        Self {
            command,
            cookie_browser: None,
        }
    }

    pub fn with_cookie_browser(
        mut self,
        cookie_browser: Option<&str>,
    ) -> Result<Self, ResearchError> {
        self.cookie_browser = validate_cookie_browser(cookie_browser)?;
        Ok(self)
    }
}

impl ResearchProvider for YtDlpProvider {
    fn scan(
        &self,
        profile: &ValidatedResearchUrl,
        limit: u16,
    ) -> Result<Vec<ResearchObservation>, ResearchError> {
        let limit = limit.clamp(1, 200);
        let mut command = Command::new(&self.command.program);
        command.args(&self.command.prefix_args).args([
            "--skip-download",
            "--dump-json",
            "--ignore-errors",
            "--no-warnings",
            "--socket-timeout",
            "20",
            "--retries",
            "2",
            "--extractor-retries",
            "2",
            "--playlist-end",
            &limit.to_string(),
        ]);
        if let Some(cookie_browser) = &self.cookie_browser {
            command.args(["--cookies-from-browser", cookie_browser]);
        }
        command
            .arg("--")
            .arg(profile.as_url().as_str())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| {
            ResearchError::Provider(format!(
                "could not start yt-dlp runtime: {}",
                provider_io_message(&error)
            ))
        })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ResearchError::Provider("yt-dlp stdout was unavailable".to_owned()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ResearchError::Provider("yt-dlp stderr was unavailable".to_owned()))?;
        let stderr_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr
                .take(MAX_PROVIDER_ERROR_BYTES)
                .read_to_end(&mut bytes);
            String::from_utf8_lossy(&bytes).trim().to_owned()
        });

        let mut observations = Vec::new();
        let mut invalid_lines = 0_usize;
        let mut reader = BufReader::new(stdout);
        let mut line = Vec::new();
        loop {
            line.clear();
            let read = reader
                .read_until(b'\n', &mut line)
                .map_err(|error| ResearchError::Provider(provider_io_message(&error)))?;
            if read == 0 {
                break;
            }
            if line.len() > MAX_PROVIDER_LINE_BYTES {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ResearchError::OutputTooLarge);
            }
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            match serde_json::from_slice::<Value>(&line) {
                Ok(raw) => match normalize_research_item(&raw, profile.as_url()) {
                    Ok(Some(item)) => observations.push(item),
                    Ok(None) => {}
                    Err(_) => invalid_lines += 1,
                },
                Err(_) => invalid_lines += 1,
            }
            if observations.len() >= usize::from(limit) {
                break;
            }
        }

        let status = child
            .wait()
            .map_err(|error| ResearchError::Provider(provider_io_message(&error)))?;
        let stderr = stderr_reader.join().unwrap_or_default();
        if !status.success() && observations.is_empty() {
            let detail = safe_provider_detail(&stderr);
            return Err(ResearchError::Provider(if detail.is_empty() {
                format!("yt-dlp exited with status {status}")
            } else {
                format!("yt-dlp exited with status {status}: {detail}")
            }));
        }
        if observations.is_empty() && invalid_lines > 0 {
            return Err(ResearchError::InvalidProviderData(format!(
                "yt-dlp returned {invalid_lines} unreadable item(s)"
            )));
        }
        Ok(observations)
    }
}

fn validate_cookie_browser(raw: Option<&str>) -> Result<Option<String>, ResearchError> {
    let Some(value) = raw
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "none")
    else {
        return Ok(None);
    };
    if value.chars().count() > 160
        || value.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.' | '+'))
        })
    {
        return Err(ResearchError::Provider(
            "cookie browser setting contains unsupported characters".to_owned(),
        ));
    }
    Ok(Some(value.to_owned()))
}

fn provider_io_message(error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::NotFound => "executable was not found".to_owned(),
        std::io::ErrorKind::PermissionDenied => "permission was denied".to_owned(),
        _ => "local provider process I/O failed".to_owned(),
    }
}

fn safe_provider_detail(raw: &str) -> String {
    let sanitized = raw.replace('\r', " ").replace('\n', " ");
    let mut chars = sanitized.chars();
    let shortened = chars.by_ref().take(1_200).collect::<String>();
    if chars.next().is_some() {
        format!("{shortened}…")
    } else {
        shortened
    }
}

fn normalize_research_item(
    raw: &Value,
    creator_url: &Url,
) -> Result<Option<ResearchObservation>, ResearchError> {
    let source_url = first_string(raw, &["webpage_url", "original_url", "url"])
        .filter(|value| is_absolute_http_url(value))
        .or_else(|| youtube_fallback(raw));
    let Some(source_url) = source_url else {
        return Ok(None);
    };

    let title = first_string(raw, &["title", "fulltitle"])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(2_000).collect());
    let published_at = first_string(raw, &["upload_date", "release_date"])
        .and_then(normalize_upload_date)
        .or_else(|| {
            raw.get("timestamp")
                .or_else(|| raw.get("release_timestamp"))
                .and_then(Value::as_f64)
                .map(|value| format!("unix:{value:.0}"))
        });
    let raw_json = sanitized_raw_json(raw)?;

    Ok(Some(ResearchObservation {
        platform: platform_from_url(&source_url, creator_url),
        source_url,
        title,
        view_count: integer_metric(raw.get("view_count")),
        like_count: integer_metric(raw.get("like_count")),
        comment_count: integer_metric(raw.get("comment_count")),
        published_at,
        duration_seconds: raw.get("duration").and_then(Value::as_f64),
        raw_json,
    }))
}

fn first_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
}

fn integer_metric(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
    })
}

fn is_absolute_http_url(raw: &str) -> bool {
    Url::parse(raw)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https") && url.host().is_some())
}

fn youtube_fallback(raw: &Value) -> Option<String> {
    let extractor = first_string(raw, &["extractor_key", "extractor"])?;
    if !extractor.to_ascii_lowercase().contains("youtube") {
        return None;
    }
    let id = raw.get("id")?.as_str()?.trim();
    if id.is_empty() || id.chars().count() > 128 {
        return None;
    }
    Some(format!("https://www.youtube.com/watch?v={id}"))
}

fn platform_from_url(source_url: &str, creator_url: &Url) -> String {
    let host = Url::parse(source_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .or_else(|| creator_url.host_str().map(str::to_owned))
        .unwrap_or_default()
        .to_ascii_lowercase();
    if host == "instagram.com" || host.ends_with(".instagram.com") {
        "Instagram"
    } else if host == "tiktok.com" || host.ends_with(".tiktok.com") {
        "TikTok"
    } else if host == "youtube.com"
        || host.ends_with(".youtube.com")
        || host == "youtu.be"
        || host.ends_with(".youtu.be")
    {
        "YouTube"
    } else {
        "Web"
    }
    .to_owned()
}

fn normalize_upload_date(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.len() == 8 && value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Some(format!("{}-{}-{}", &value[..4], &value[4..6], &value[6..8]));
    }
    if value.is_empty() {
        None
    } else {
        Some(value.chars().take(64).collect())
    }
}

fn sanitized_raw_json(raw: &Value) -> Result<Option<String>, ResearchError> {
    let mut safe = Map::new();
    for key in ["id", "uploader", "channel", "channel_url", "thumbnail"] {
        if let Some(value) = raw.get(key).and_then(Value::as_str) {
            safe.insert(
                key.to_owned(),
                json!(value.chars().take(2_048).collect::<String>()),
            );
        }
    }
    if let Some(description) = raw.get("description").and_then(Value::as_str) {
        safe.insert(
            "description".to_owned(),
            json!(description
                .chars()
                .take(MAX_RAW_DESCRIPTION_CHARS)
                .collect::<String>()),
        );
    }
    if safe.is_empty() {
        return Ok(None);
    }
    serde_json::to_string(&Value::Object(safe))
        .map(Some)
        .map_err(|error| ResearchError::InvalidProviderData(error.to_string()))
}

#[derive(Debug)]
pub struct ResearchService<P> {
    provider: P,
    policy: NetworkPolicy,
}

impl<P> ResearchService<P>
where
    P: ResearchProvider,
{
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            policy: NetworkPolicy,
        }
    }

    pub fn scan(
        &self,
        project_id: Uuid,
        creator_id: Option<Uuid>,
        raw_url: &str,
        limit: u16,
    ) -> Result<Vec<ResearchItem>, ResearchError> {
        let validated = self.policy.validate(raw_url)?;
        let observations = self.provider.scan(&validated, limit.clamp(1, 200))?;
        observations
            .into_iter()
            .map(|observation| {
                let source_url = self.policy.validate(&observation.source_url)?;
                Ok(ResearchItem {
                    id: Uuid::new_v4(),
                    project_id,
                    creator_id,
                    source_url: source_url.as_url().to_string(),
                    platform: observation.platform,
                    title: observation.title,
                    view_count: observation.view_count,
                    like_count: observation.like_count,
                    comment_count: observation.comment_count,
                    published_at: observation.published_at,
                    duration_seconds: observation.duration_seconds,
                    raw_json: observation.raw_json,
                    scanned_at: now_rfc3339(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    use super::*;

    #[derive(Clone, Default)]
    struct FakeProvider {
        calls: Arc<Mutex<Vec<(String, u16)>>>,
    }

    impl ResearchProvider for FakeProvider {
        fn scan(
            &self,
            profile: &ValidatedResearchUrl,
            limit: u16,
        ) -> Result<Vec<ResearchObservation>, ResearchError> {
            self.calls
                .lock()
                .unwrap()
                .push((profile.as_url().to_string(), limit));
            Ok(Vec::new())
        }
    }

    #[test]
    fn accepts_supported_platform_subdomains_and_standard_ports() {
        let policy = NetworkPolicy;
        assert!(policy
            .validate("https://www.instagram.com/example/")
            .is_ok());
        assert!(policy.validate("https://m.youtube.com/@example").is_ok());
        assert!(policy
            .validate("https://www.youtube.com:443/@example")
            .is_ok());
    }

    #[test]
    fn rejects_lookalike_nonstandard_port_and_non_http_urls() {
        let policy = NetworkPolicy;
        assert_eq!(
            policy.validate("https://youtube.com.attacker.example/watch?v=1"),
            Err(ResearchError::HostNotAllowed)
        );
        assert_eq!(
            policy.validate("https://youtube.com:444/@example"),
            Err(ResearchError::NonStandardPort)
        );
        assert!(matches!(
            policy.validate("file:///etc/passwd"),
            Err(ResearchError::InvalidUrl(_))
        ));
    }

    #[test]
    fn invalid_url_never_reaches_provider() {
        let provider = FakeProvider::default();
        let calls = provider.calls.clone();
        let service = ResearchService::new(provider);
        assert!(service
            .scan(Uuid::new_v4(), None, "http://127.0.0.1/private", 50)
            .is_err());
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn provider_limit_is_clamped() {
        let provider = FakeProvider::default();
        let calls = provider.calls.clone();
        let service = ResearchService::new(provider);
        service
            .scan(
                Uuid::new_v4(),
                None,
                "https://www.tiktok.com/@example",
                u16::MAX,
            )
            .unwrap();
        assert_eq!(calls.lock().unwrap()[0].1, 200);
    }

    #[test]
    fn provider_returned_source_urls_are_revalidated() {
        #[derive(Clone)]
        struct MaliciousProvider;
        impl ResearchProvider for MaliciousProvider {
            fn scan(
                &self,
                _profile: &ValidatedResearchUrl,
                _limit: u16,
            ) -> Result<Vec<ResearchObservation>, ResearchError> {
                Ok(vec![ResearchObservation {
                    source_url: "http://127.0.0.1/private".to_owned(),
                    platform: "YouTube".to_owned(),
                    title: None,
                    view_count: None,
                    like_count: None,
                    comment_count: None,
                    published_at: None,
                    duration_seconds: None,
                    raw_json: None,
                }])
            }
        }
        let error = ResearchService::new(MaliciousProvider)
            .scan(Uuid::new_v4(), None, "https://www.youtube.com/@creator", 25)
            .unwrap_err();
        assert_eq!(error, ResearchError::HostNotAllowed);
    }

    #[cfg(unix)]
    #[test]
    fn yt_dlp_provider_parses_line_stream_without_live_network() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let fixture = temp.path().join("fake-yt-dlp");
        fs::write(
            &fixture,
            "#!/bin/sh\nprintf '%s\\n' '{\"id\":\"abc\",\"extractor_key\":\"Youtube\",\"title\":\"Fixture\",\"view_count\":42,\"like_count\":5,\"comment_count\":2,\"upload_date\":\"20260809\",\"duration\":12.5}'\n",
        )
        .unwrap();
        fs::set_permissions(&fixture, fs::Permissions::from_mode(0o755)).unwrap();
        let provider = YtDlpProvider::new(YtDlpCommand::executable(fixture));
        let validated = NetworkPolicy
            .validate("https://www.youtube.com/@creator")
            .unwrap();
        let items = provider.scan(&validated, 25).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source_url, "https://www.youtube.com/watch?v=abc");
        assert_eq!(items[0].title.as_deref(), Some("Fixture"));
        assert_eq!(items[0].view_count, Some(42));
        assert_eq!(items[0].published_at.as_deref(), Some("2026-08-09"));
    }
}
