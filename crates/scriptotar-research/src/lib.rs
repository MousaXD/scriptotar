use scriptotar_core::{now_rfc3339, ResearchItem};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

const ALLOWED_DOMAINS: &[&str] = &["instagram.com", "tiktok.com", "youtube.com", "youtu.be"];

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
    #[error("research provider failed: {0}")]
    Provider(String),
}

#[derive(Debug, Clone, Default)]
pub struct NetworkPolicy;

impl NetworkPolicy {
    pub fn validate(&self, raw: &str) -> Result<ValidatedResearchUrl, ResearchError> {
        let url = Url::parse(raw).map_err(|error| ResearchError::InvalidUrl(error.to_string()))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(ResearchError::InvalidUrl(
                "only http and https are supported".to_owned(),
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(ResearchError::EmbeddedCredentials);
        }
        if url.port().is_some() {
            return Err(ResearchError::NonStandardPort);
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
        Ok(observations
            .into_iter()
            .map(|observation| ResearchItem {
                id: Uuid::new_v4(),
                project_id,
                creator_id,
                source_url: observation.source_url,
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
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

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
    fn accepts_supported_platform_subdomains() {
        let policy = NetworkPolicy;
        assert!(policy
            .validate("https://www.instagram.com/example/")
            .is_ok());
        assert!(policy.validate("https://m.youtube.com/@example").is_ok());
    }

    #[test]
    fn rejects_lookalike_and_non_http_urls() {
        let policy = NetworkPolicy;
        assert_eq!(
            policy.validate("https://youtube.com.attacker.example/watch?v=1"),
            Err(ResearchError::HostNotAllowed)
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
}
