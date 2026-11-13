//! Offline, evidence-aware analysis primitives. No network, persistence, or AI calls.
//! Callers must validate project membership before mapping external Study IDs.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use scriptotar_core::ResearchItem;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::NetworkPolicy;

const MIN_CREATOR_PEERS: usize = 3;
const MAX_SEGMENTS: usize = 2_000;
const MAX_SEGMENT_CHARS: usize = 2_000;
const MAX_EXCERPT_CHARS: usize = 160;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IntelligenceError {
    #[error("invalid {0} count")]
    InvalidCount(&'static str),
    #[error("invalid observation timestamp")]
    InvalidTimestamp,
    #[error("source URL violates research access policy")]
    InvalidSource,
    #[error("source identifier conflicts across project boundaries")]
    ProjectMismatch,
    #[error("contradictory observations for one source and timestamp")]
    ContradictoryObservation,
    #[error("invalid transcript segment or evidence boundary")]
    InvalidSegment,
    #[error("source is unavailable: {0}")]
    SourceUnavailable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", content = "date", rename_all = "snake_case")]
pub enum PublicationDate {
    Known(NaiveDate),
    Missing,
    Unrecognized,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricSnapshot {
    pub project_id: Uuid,
    pub research_item_id: Uuid,
    pub creator_id: Option<Uuid>,
    pub source_url: String,
    pub platform: String,
    /// When this measurement was collected, not when the video was uploaded.
    pub observed_at: DateTime<Utc>,
    pub published: PublicationDate,
    pub views: Option<u64>,
    pub likes: Option<u64>,
    pub comments: Option<u64>,
}

fn checked_count(value: Option<i64>, field: &'static str) -> Result<Option<u64>, IntelligenceError> {
    value
        .map(|number| u64::try_from(number).map_err(|_| IntelligenceError::InvalidCount(field)))
        .transpose()
}

fn canonical_source(raw: &str) -> Result<String, IntelligenceError> {
    NetworkPolicy
        .validate(raw)
        .map(|url| url.as_url().to_string())
        .map_err(|_| IntelligenceError::InvalidSource)
}

fn publication_date(value: Option<&str>) -> PublicationDate {
    let Some(value) = value else {
        return PublicationDate::Missing;
    };
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return PublicationDate::Known(date);
    }
    if let Some(seconds) = value.strip_prefix("unix:").and_then(|s| s.parse::<i64>().ok()) {
        if let Some(date) = Utc.timestamp_opt(seconds, 0).single() {
            return PublicationDate::Known(date.date_naive());
        }
    }
    PublicationDate::Unrecognized
}

impl MetricSnapshot {
    /// Preserves missing counts. A zero view count is real but not a rate denominator.
    /// Rejects negative counts and malformed collection times instead of guessing.
    pub fn from_research_item(item: &ResearchItem) -> Result<Self, IntelligenceError> {
        let observed_at = DateTime::parse_from_rfc3339(&item.scanned_at)
            .map_err(|_| IntelligenceError::InvalidTimestamp)?
            .with_timezone(&Utc);
        Ok(Self {
            project_id: item.project_id,
            research_item_id: item.id,
            creator_id: item.creator_id,
            source_url: canonical_source(&item.source_url)?,
            platform: item.platform.clone(),
            observed_at,
            published: publication_date(item.published_at.as_deref()),
            views: checked_count(item.view_count, "views")?,
            likes: checked_count(item.like_count, "likes")?,
            comments: checked_count(item.comment_count, "comments")?,
        })
    }

    pub fn observed_interactions_per_view(&self) -> Option<f64> {
        match (self.views, self.likes, self.comments) {
            (Some(views), Some(likes), Some(comments)) if views > 0 => {
                Some((likes as f64 + comments as f64) / views as f64)
            }
            _ => None,
        }
    }

    pub fn is_stale_at(&self, as_of: DateTime<Utc>, max_age_days: i64) -> bool {
        max_age_days < 0
            || self.observed_at > as_of
            || as_of.signed_duration_since(self.observed_at).num_seconds()
                > max_age_days.saturating_mul(86_400)
    }

    fn video_age_days(&self) -> Option<i64> {
        match self.published {
            PublicationDate::Known(date) => {
                let age = self.observed_at.date_naive().signed_duration_since(date).num_days();
                (age >= 1).then_some(age)
            }
            PublicationDate::Missing | PublicationDate::Unrecognized => None,
        }
    }
}

/// Same exact provider URL means the same source for this stage. Do not erase query
/// parameters that might be identity-bearing; alias resolution belongs to P1 ingest.
pub fn deduplicate_snapshots(
    snapshots: impl IntoIterator<Item = MetricSnapshot>,
) -> Result<Vec<MetricSnapshot>, IntelligenceError> {
    let mut seen_ids = HashMap::new();
    let mut seen = HashMap::<(Uuid, String, DateTime<Utc>), MetricSnapshot>::new();
    for mut snapshot in snapshots {
        snapshot.source_url = canonical_source(&snapshot.source_url)?;
        if let Some(project) = seen_ids.insert(snapshot.research_item_id, snapshot.project_id) {
            if project != snapshot.project_id {
                return Err(IntelligenceError::ProjectMismatch);
            }
        }
        let key = (
            snapshot.project_id,
            snapshot.source_url.clone(),
            snapshot.observed_at,
        );
        if let Some(previous) = seen.get(&key) {
            if previous.views != snapshot.views
                || previous.likes != snapshot.likes
                || previous.comments != snapshot.comments
                || previous.creator_id != snapshot.creator_id
                || previous.platform != snapshot.platform
                || previous.published != snapshot.published
            {
                return Err(IntelligenceError::ContradictoryObservation);
            }
            // Stable identity choice for duplicate observations with equal data.
            if snapshot.research_item_id < previous.research_item_id {
                seen.insert(key, snapshot);
            }
            continue;
        }
        seen.insert(key, snapshot);
    }
    let mut result: Vec<_> = seen.into_values().collect();
    result.sort_by(|a, b| {
        (a.project_id, &a.source_url, a.observed_at)
            .cmp(&(b.project_id, &b.source_url, b.observed_at))
    });
    Ok(result)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonLimitation {
    MissingCreator,
    MissingViews,
    UnknownUploadDate,
    StaleObservation,
    InsufficientAgeMatchedPeers,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CreatorComparison {
    Unavailable {
        reason: ComparisonLimitation,
        eligible_peers: usize,
    },
    Observed {
        peer_count: usize,
        median_peer_views: f64,
        views_relative_to_peer_median: f64,
        /// Descriptive, age-matched comparison, not a probability or viral score.
        interpretation: String,
    },
}

/// Compare only different video URLs from the same project, creator and platform.
/// Peer ages at collection must be within 0.5x..2x of the target's video age.
/// Comparisons are deliberately unavailable without at least three eligible peers.
pub fn compare_creator_views(
    target: &MetricSnapshot,
    available: &[MetricSnapshot],
    as_of: DateTime<Utc>,
    max_stale_days: i64,
) -> Result<CreatorComparison, IntelligenceError> {
    let unavailable = |reason, eligible_peers| CreatorComparison::Unavailable {
        reason,
        eligible_peers,
    };
    let Some(creator_id) = target.creator_id else {
        return Ok(unavailable(ComparisonLimitation::MissingCreator, 0));
    };
    let Some(target_views) = target.views else {
        return Ok(unavailable(ComparisonLimitation::MissingViews, 0));
    };
    let Some(age_days) = target.video_age_days() else {
        return Ok(unavailable(ComparisonLimitation::UnknownUploadDate, 0));
    };
    if target.is_stale_at(as_of, max_stale_days) {
        return Ok(unavailable(ComparisonLimitation::StaleObservation, 0));
    }
    let target_url = canonical_source(&target.source_url)?;
    let mut newest_by_url: HashMap<String, &MetricSnapshot> = HashMap::new();
    for peer in available {
        if peer.research_item_id == target.research_item_id
            && peer.project_id != target.project_id
        {
            return Err(IntelligenceError::ProjectMismatch);
        }
        if peer.project_id != target.project_id
            || peer.creator_id != Some(creator_id)
            || peer.platform != target.platform
            || peer.is_stale_at(as_of, max_stale_days)
        {
            continue;
        }
        let peer_url = canonical_source(&peer.source_url)?;
        if peer_url == target_url {
            continue;
        }
        if let Some(previous) = newest_by_url.get(&peer_url) {
            if peer.observed_at == previous.observed_at
                && (peer.views != previous.views
                    || peer.likes != previous.likes
                    || peer.comments != previous.comments
                    || peer.published != previous.published)
            {
                return Err(IntelligenceError::ContradictoryObservation);
            }
            if peer.observed_at <= previous.observed_at {
                continue;
            }
        }
        newest_by_url.insert(peer_url, peer);
    }
    let mut peer_views = Vec::new();
    for peer in newest_by_url.values() {
        if let (Some(peer_age), Some(views)) = (peer.video_age_days(), peer.views) {
            if peer_age.saturating_mul(2) >= age_days
                && peer_age <= age_days.saturating_mul(2)
            {
                peer_views.push(views);
            }
        }
    }
    if peer_views.len() < MIN_CREATOR_PEERS {
        return Ok(unavailable(
            ComparisonLimitation::InsufficientAgeMatchedPeers,
            peer_views.len(),
        ));
    }
    peer_views.sort_unstable();
    let middle = peer_views.len() / 2;
    let median = if peer_views.len() % 2 == 0 {
        (peer_views[middle - 1] as f64 + peer_views[middle] as f64) / 2.0
    } else {
        peer_views[middle] as f64
    };
    if median == 0.0 {
        return Ok(unavailable(ComparisonLimitation::MissingViews, peer_views.len()));
    }
    Ok(CreatorComparison::Observed {
        peer_count: peer_views.len(),
        median_peer_views: median,
        views_relative_to_peer_median: target_views as f64 / median,
        interpretation: "Observed views relative to the creator's age-matched videos, not a retention measure, causal finding or forecast.".into(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewGrowth {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub observed_view_increase: u64,
    pub views_per_day: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrowthLimitation {
    UnavailableCounts,
    NonIncreasingTime,
    CountersDecreased,
    DifferentSource,
}

/// Requires two real snapshots of one source. A decrease can mean a platform
/// correction and must not be silently interpreted as negative audience growth.
pub fn observed_view_growth(
    older: &MetricSnapshot,
    newer: &MetricSnapshot,
) -> Result<Result<ViewGrowth, GrowthLimitation>, IntelligenceError> {
    if older.project_id != newer.project_id {
        return Err(IntelligenceError::ProjectMismatch);
    }
    if canonical_source(&older.source_url)? != canonical_source(&newer.source_url)? {
        return Ok(Err(GrowthLimitation::DifferentSource));
    }
    let (Some(start_views), Some(end_views)) = (older.views, newer.views) else {
        return Ok(Err(GrowthLimitation::UnavailableCounts));
    };
    let elapsed = newer.observed_at.signed_duration_since(older.observed_at).num_seconds();
    if elapsed <= 0 {
        return Ok(Err(GrowthLimitation::NonIncreasingTime));
    }
    if end_views < start_views {
        return Ok(Err(GrowthLimitation::CountersDecreased));
    }
    Ok(Ok(ViewGrowth {
        start: older.observed_at,
        end: newer.observed_at,
        observed_view_increase: end_views - start_views,
        views_per_day: (end_views - start_views) as f64 * 86_400.0 / elapsed as f64,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptScope {
    pub project_id: Uuid,
    pub research_item_id: Uuid,
    pub transcript_id: Uuid,
    /// Caller-supplied immutable transcript revision or fingerprint.
    pub transcript_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptSegment {
    pub segment_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceRef {
    pub scope: TranscriptScope,
    pub segment_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    /// Small, auditable excerpt; no full transcript replication.
    pub excerpt: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreativeBeat {
    OpeningExcerpt,
    MiddleExcerpt,
    ClosingExcerpt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceConfidence {
    DirectTranscriptExcerpt,
    InferredFromTranscript,
    Unverified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreativeObservation {
    pub beat: CreativeBeat,
    pub evidence: EvidenceRef,
    pub confidence: SourceConfidence,
    /// This extractor does not infer hook type, payoff, CTA, visual pacing or retention.
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceAccess {
    Available,
    Denied,
    Unsupported,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptOutline {
    pub source_access: SourceAccess,
    pub observations: Vec<CreativeObservation>,
    pub warnings: Vec<String>,
}

fn evidence(scope: &TranscriptScope, segment: &TranscriptSegment) -> EvidenceRef {
    EvidenceRef {
        scope: scope.clone(),
        segment_id: segment.segment_id.clone(),
        start_ms: segment.start_ms,
        end_ms: segment.end_ms,
        excerpt: segment.text.chars().take(MAX_EXCERPT_CHARS).collect(),
    }
}

/// Produces timestamped, literal opening/middle/closing excerpts only.
/// Semantic hook, narrative, payoff and CTA labels are intentionally left for
/// a separate reviewed extractor that must cite these exact evidence anchors.
pub fn extract_transcript_outline(
    scope: &TranscriptScope,
    segments: &[TranscriptSegment],
    access: SourceAccess,
) -> Result<TranscriptOutline, IntelligenceError> {
    if matches!(access, SourceAccess::Denied | SourceAccess::Unsupported) {
        return Err(IntelligenceError::SourceUnavailable(format!("{access:?}")));
    }
    if scope.transcript_revision.trim().is_empty()
        || scope.transcript_revision.len() > 128
        || segments.len() > MAX_SEGMENTS
    {
        return Err(IntelligenceError::InvalidSegment);
    }
    let mut ids = HashSet::new();
    let mut previous_start = 0;
    for segment in segments {
        if segment.segment_id.is_empty()
            || segment.segment_id.len() > 128
            || !ids.insert(&segment.segment_id)
            || segment.start_ms >= segment.end_ms
            || segment.start_ms < previous_start
            || segment.text.chars().count() > MAX_SEGMENT_CHARS
        {
            return Err(IntelligenceError::InvalidSegment);
        }
        previous_start = segment.start_ms;
    }
    let populated: Vec<_> = segments.iter().filter(|s| !s.text.trim().is_empty()).collect();
    let mut warnings = Vec::new();
    if matches!(access, SourceAccess::Partial) {
        warnings.push("Source extraction was partial; unobserved sections must not be inferred.".into());
    }
    if populated.is_empty() {
        warnings.push("No spoken transcript segments are available.".into());
        return Ok(TranscriptOutline {
            source_access: access,
            observations: Vec::new(),
            warnings,
        });
    }
    let mut positions = vec![(0, CreativeBeat::OpeningExcerpt)];
    if populated.len() >= 3 {
        positions.push((populated.len() / 2, CreativeBeat::MiddleExcerpt));
    }
    if populated.len() >= 2 {
        positions.push((populated.len() - 1, CreativeBeat::ClosingExcerpt));
    }
    let observations = positions
        .into_iter()
        .map(|(index, beat)| CreativeObservation {
            beat,
            evidence: evidence(scope, populated[index]),
            confidence: SourceConfidence::DirectTranscriptExcerpt,
            interpretation: "Observed spoken text at this transcript position; creative intent and visuals are unverified.".into(),
        })
        .collect();
    Ok(TranscriptOutline {
        source_access: access,
        observations,
        warnings,
    })
}
