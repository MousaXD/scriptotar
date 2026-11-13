//! Offline, evidence-aware analysis primitives. No network, persistence, or AI calls.
//! Callers must validate project membership before mapping external Study IDs.

use std::collections::{HashMap, HashSet};

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

/// UTC instant with nanosecond precision. Validates RFC3339 without a new
/// workspace dependency, keeping root Cargo.lock owned by Agent 00.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObservationTimestamp {
    pub unix_seconds: i64,
    pub nanosecond: u32,
}

impl ObservationTimestamp {
    pub fn parse_rfc3339(raw: &str) -> Option<Self> {
        let bytes = raw.as_bytes();
        if bytes.len() < 20
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || !matches!(bytes[10], b'T' | b't')
            || bytes[13] != b':'
            || bytes[16] != b':'
        {
            return None;
        }
        let digits = |start: usize, end: usize| -> Option<u32> {
            std::str::from_utf8(bytes.get(start..end)?)
                .ok()?
                .parse()
                .ok()
        };
        let year = digits(0, 4)? as i64;
        let month = digits(5, 7)?;
        let day = digits(8, 10)?;
        let hour = digits(11, 13)?;
        let minute = digits(14, 16)?;
        let second = digits(17, 19)?;
        let epoch_day = validated_civil_day(year, month, day)?;
        if hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        let mut index = 19;
        let mut nanosecond = 0_u32;
        if bytes.get(index) == Some(&b'.') {
            index += 1;
            let start = index;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                if index - start >= 9 {
                    return None;
                }
                nanosecond = nanosecond * 10 + u32::from(bytes[index] - b'0');
                index += 1;
            }
            if start == index {
                return None;
            }
            for _ in 0..(9 - (index - start)) {
                nanosecond *= 10;
            }
        }
        let offset = match bytes.get(index) {
            Some(&b'Z') | Some(&b'z') if index + 1 == bytes.len() => 0_i64,
            Some(&sign) if matches!(sign, b'+' | b'-')
                && index + 6 == bytes.len()
                && bytes[index + 3] == b':' => {
                let offset_hours = digits(index + 1, index + 3)?;
                let offset_minutes = digits(index + 4, index + 6)?;
                if offset_hours > 23 || offset_minutes > 59 {
                    return None;
                }
                let seconds = i64::from(offset_hours * 3_600 + offset_minutes * 60);
                if sign == b'+' { seconds } else { -seconds }
            }
            _ => return None,
        };
        Some(Self {
            unix_seconds: epoch_day * 86_400 + i64::from(hour * 3_600 + minute * 60 + second)
                - offset,
            nanosecond,
        })
    }

    pub fn add_seconds(self, seconds: i64) -> Self {
        Self {
            unix_seconds: self.unix_seconds.saturating_add(seconds),
            ..self
        }
    }

    pub fn seconds_since(self, earlier: Self) -> i64 {
        self.unix_seconds.saturating_sub(earlier.unix_seconds)
            - i64::from(self.nanosecond < earlier.nanosecond)
    }
}

/// Gregorian day index relative to Unix epoch, rejecting impossible dates.
fn validated_civil_day(year: i64, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=max_day).contains(&day) {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + i64::from(day) - 1;
    let year_day = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + year_day - 719_468)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", content = "epoch_day", rename_all = "snake_case")]
pub enum PublicationDate {
    Known(i64),
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
    pub observed_at: ObservationTimestamp,
    pub published: PublicationDate,
    pub views: Option<u64>,
    pub likes: Option<u64>,
    pub comments: Option<u64>,
}

fn checked_count(
    value: Option<i64>,
    field: &'static str,
) -> Result<Option<u64>, IntelligenceError> {
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
    let bytes = value.as_bytes();
    if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        if let (Ok(year), Ok(month), Ok(day)) = (
            std::str::from_utf8(&bytes[0..4]).unwrap_or("").parse::<i64>(),
            std::str::from_utf8(&bytes[5..7]).unwrap_or("").parse::<u32>(),
            std::str::from_utf8(&bytes[8..10]).unwrap_or("").parse::<u32>(),
        ) {
            if let Some(epoch_day) = validated_civil_day(year, month, day) {
                return PublicationDate::Known(epoch_day);
            }
        }
    }
    if let Some(seconds) = value
        .strip_prefix("unix:")
        .and_then(|s| s.parse::<i64>().ok())
    {
        return PublicationDate::Known(seconds.div_euclid(86_400));
    }
    PublicationDate::Unrecognized
}

impl MetricSnapshot {
    /// Preserves missing counts. A zero view count is real but not a rate denominator.
    /// Rejects negative counts and malformed collection times instead of guessing.
    pub fn from_research_item(item: &ResearchItem) -> Result<Self, IntelligenceError> {
        let observed_at = ObservationTimestamp::parse_rfc3339(&item.scanned_at)
            .ok_or(IntelligenceError::InvalidTimestamp)?;
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

    pub fn is_stale_at(&self, as_of: ObservationTimestamp, max_age_days: i64) -> bool {
        max_age_days < 0
            || self.observed_at > as_of
            || as_of.seconds_since(self.observed_at) > max_age_days.saturating_mul(86_400)
    }

    fn video_age_days(&self) -> Option<i64> {
        match &self.published {
            PublicationDate::Known(date) => {
                let age = self.observed_at.unix_seconds.div_euclid(86_400) - date;
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
    let mut seen_ids = HashMap::<Uuid, (Uuid, String)>::new();
    let mut seen = HashMap::<(Uuid, String, ObservationTimestamp), MetricSnapshot>::new();
    for mut snapshot in snapshots {
        snapshot.source_url = canonical_source(&snapshot.source_url)?;
        if let Some((project, source)) = seen_ids.insert(
            snapshot.research_item_id,
            (snapshot.project_id, snapshot.source_url.clone()),
        ) {
            if project != snapshot.project_id {
                return Err(IntelligenceError::ProjectMismatch);
            }
            if source != snapshot.source_url {
                return Err(IntelligenceError::ContradictoryObservation);
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
        (a.project_id, &a.source_url, a.observed_at).cmp(&(
            b.project_id,
            &b.source_url,
            b.observed_at,
        ))
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
    as_of: ObservationTimestamp,
    max_stale_days: i64,
) -> Result<CreatorComparison, IntelligenceError> {
    let target_url = canonical_source(&target.source_url)?;
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
    let mut newest_by_url: HashMap<String, &MetricSnapshot> = HashMap::new();
    for peer in available {
        if peer.research_item_id == target.research_item_id && peer.project_id != target.project_id
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
            if peer_age.saturating_mul(2) >= age_days && peer_age <= age_days.saturating_mul(2) {
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
        return Ok(unavailable(
            ComparisonLimitation::MissingViews,
            peer_views.len(),
        ));
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
    pub start: ObservationTimestamp,
    pub end: ObservationTimestamp,
    pub observed_view_increase: u64,
    pub views_per_day: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrowthLimitation {
    UnavailableCounts,
    NonIncreasingTime,
    InsufficientObservationWindow,
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
    if older.platform != newer.platform
        || canonical_source(&older.source_url)? != canonical_source(&newer.source_url)?
    {
        return Ok(Err(GrowthLimitation::DifferentSource));
    }
    let (Some(start_views), Some(end_views)) = (older.views, newer.views) else {
        return Ok(Err(GrowthLimitation::UnavailableCounts));
    };
    let elapsed = newer.observed_at.seconds_since(older.observed_at);
    if elapsed <= 0 {
        return Ok(Err(GrowthLimitation::NonIncreasingTime));
    }
    if elapsed < 3_600 {
        return Ok(Err(GrowthLimitation::InsufficientObservationWindow));
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

/// Semantic analysis contracts are deliberately distinct from direct excerpts:
/// no model or heuristic implementation is implied by these types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptFeature {
    Hook,
    OpeningPromise,
    NarrativeSetup,
    NarrativeTurn,
    SpokenPacing,
    Payoff,
    CallToAction,
    RhetoricalDevice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptHypothesis {
    pub feature: TranscriptFeature,
    pub statement: String,
    pub evidence: Vec<EvidenceRef>,
    pub confidence: SourceConfidence,
    /// Transcript-only hypotheses are never evidence of what the frames show.
    pub limitations: Vec<String>,
}

/// Validate support and conservatively label a proposed interpretation as inferred.
/// This API records a hypothesis; it does not determine whether the hypothesis is true.
pub fn propose_transcript_hypothesis(
    feature: TranscriptFeature,
    statement: &str,
    evidence: Vec<EvidenceRef>,
) -> Result<TranscriptHypothesis, IntelligenceError> {
    if statement.trim().is_empty()
        || statement.chars().count() > 500
        || evidence.is_empty()
        || evidence.len() > 12
    {
        return Err(IntelligenceError::InvalidSegment);
    }
    let first_scope = &evidence[0].scope;
    if evidence.iter().any(|reference| {
        &reference.scope != first_scope
            || reference.segment_id.is_empty()
            || reference.start_ms >= reference.end_ms
            || reference.excerpt.trim().is_empty()
            || reference.excerpt.chars().count() > MAX_EXCERPT_CHARS
    }) {
        return Err(IntelligenceError::InvalidSegment);
    }
    Ok(TranscriptHypothesis {
        feature,
        statement: statement.trim().to_owned(),
        evidence,
        confidence: SourceConfidence::InferredFromTranscript,
        limitations: vec![
            "Transcript-only interpretation; frames, edits and audience behavior were not inspected."
                .into(),
        ],
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureSupport {
    pub project_id: Uuid,
    pub feature: TranscriptFeature,
    /// Number of distinct research sources with a proposed interpretation.
    /// Absence of a hypothesis is not a negative observation.
    pub sources_with_hypothesis: usize,
}

/// Deterministically count proposed features across distinct sources.
/// Does not label a feature as successful, causal or independently corroborated.
pub fn count_transcript_features(
    project_id: Uuid,
    hypotheses: &[TranscriptHypothesis],
) -> Result<Vec<FeatureSupport>, IntelligenceError> {
    let mut counts: HashMap<TranscriptFeature, HashSet<Uuid>> = HashMap::new();
    for hypothesis in hypotheses {
        if hypothesis.evidence.is_empty()
            || hypothesis.evidence.iter().any(|e| {
                e.scope.project_id != project_id || e.scope != hypothesis.evidence[0].scope
            })
        {
            return Err(IntelligenceError::ProjectMismatch);
        }
        counts
            .entry(hypothesis.feature)
            .or_default()
            .insert(hypothesis.evidence[0].scope.research_item_id);
    }
    let mut result: Vec<_> = counts
        .into_iter()
        .map(|(feature, sources)| FeatureSupport {
            project_id,
            feature,
            sources_with_hypothesis: sources.len(),
        })
        .collect();
    result.sort_by_key(|item| item.feature);
    Ok(result)
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
    let populated: Vec<_> = segments
        .iter()
        .filter(|s| !s.text.trim().is_empty())
        .collect();
    let mut warnings = Vec::new();
    if matches!(access, SourceAccess::Partial) {
        warnings.push(
            "Source extraction was partial; unobserved sections must not be inferred.".into(),
        );
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
