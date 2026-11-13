use scriptotar_core::ResearchItem;
use uuid::Uuid;

use crate::intelligence::{
    compare_creator_views, deduplicate_snapshots, extract_transcript_outline, observed_view_growth,
    ComparisonLimitation, CreativeBeat, CreatorComparison, GrowthLimitation, IntelligenceError,
    MetricSnapshot, ObservationTimestamp, PublicationDate, SourceAccess, SourceConfidence, TranscriptFeature,
    TranscriptScope, TranscriptSegment, propose_transcript_hypothesis, count_transcript_features,
};

fn item(project: Uuid, creator: Option<Uuid>, id: Uuid, url: &str, days_old: i64, views: Option<i64>) -> ResearchItem {
    let publication = match days_old {
        2 => "2026-11-11",
        5 => "2026-11-08",
        8 => "2026-11-05",
        10 => "2026-11-03",
        150 => "2026-06-16",
        _ => "2026-11-03",
    };
    ResearchItem {
        id,
        project_id: project,
        creator_id: creator,
        source_url: url.into(),
        platform: "YouTube".into(),
        title: None,
        view_count: views,
        like_count: Some(8),
        comment_count: Some(2),
        published_at: Some(publication.to_owned()),
        duration_seconds: None,
        raw_json: None,
        scanned_at: "2026-11-13T10:00:00Z".into(),
    }
}

fn url(id: usize) -> String {
    format!("https://www.youtube.com/watch?v=video{id}")
}

#[test]
fn metrics_preserve_missing_zero_and_invalid_counts() {
    let p = Uuid::new_v4();
    let mut research = item(p, None, Uuid::new_v4(), &url(1), 10, None);
    let snapshot = MetricSnapshot::from_research_item(&research).unwrap();
    assert_eq!(snapshot.views, None);
    assert_eq!(snapshot.observed_interactions_per_view(), None);
    research.view_count = Some(0);
    assert_eq!(
        MetricSnapshot::from_research_item(&research)
            .unwrap()
            .observed_interactions_per_view(),
        None
    );
    research.view_count = Some(20);
    assert_eq!(
        MetricSnapshot::from_research_item(&research)
            .unwrap()
            .observed_interactions_per_view(),
        Some(0.5)
    );
    research.like_count = None;
    assert_eq!(
        MetricSnapshot::from_research_item(&research)
            .unwrap()
            .observed_interactions_per_view(),
        None
    );
    research.comment_count = Some(-1);
    assert_eq!(
        MetricSnapshot::from_research_item(&research).unwrap_err(),
        IntelligenceError::InvalidCount("comments")
    );
}

#[test]
fn missing_invalid_and_unix_upload_dates_are_distinct() {
    let p = Uuid::new_v4();
    let mut research = item(p, None, Uuid::new_v4(), &url(1), 2, Some(2));
    research.published_at = None;
    assert_eq!(
        MetricSnapshot::from_research_item(&research).unwrap().published,
        PublicationDate::Missing
    );
    research.published_at = Some("not-a-date".into());
    assert_eq!(
        MetricSnapshot::from_research_item(&research).unwrap().published,
        PublicationDate::Unrecognized
    );
    research.published_at = Some("unix:0".into());
    assert!(matches!(
        MetricSnapshot::from_research_item(&research).unwrap().published,
        PublicationDate::Known(_)
    ));
    research.scanned_at = "corrupt".into();
    assert_eq!(
        MetricSnapshot::from_research_item(&research).unwrap_err(),
        IntelligenceError::InvalidTimestamp
    );
}

#[test]
fn source_policy_and_project_scoping_are_enforced() {
    let p = Uuid::new_v4();
    let id = Uuid::new_v4();
    let mut research = item(p, None, id, "http://127.0.0.1/private", 8, Some(20));
    assert_eq!(
        MetricSnapshot::from_research_item(&research).unwrap_err(),
        IntelligenceError::InvalidSource
    );
    research.source_url = url(1);
    let first = MetricSnapshot::from_research_item(&research).unwrap();
    research.project_id = Uuid::new_v4();
    let second = MetricSnapshot::from_research_item(&research).unwrap();
    assert_eq!(
        deduplicate_snapshots(vec![first, second]).unwrap_err(),
        IntelligenceError::ProjectMismatch
    );
}

#[test]
fn duplicates_coalesce_and_contradictions_fail_closed() {
    let p = Uuid::new_v4();
    let creator = Some(Uuid::new_v4());
    let a = MetricSnapshot::from_research_item(&item(p, creator, Uuid::new_v4(), &url(1), 8, Some(90))).unwrap();
    let mut repeated = a.clone();
    repeated.research_item_id = Uuid::new_v4();
    assert_eq!(deduplicate_snapshots(vec![a.clone(), repeated.clone()]).unwrap().len(), 1);
    repeated.views = Some(91);
    assert_eq!(
        deduplicate_snapshots(vec![a, repeated]).unwrap_err(),
        IntelligenceError::ContradictoryObservation
    );
}

#[test]
fn same_creator_age_matched_comparison_does_not_mix_projects_or_sources() {
    let p = Uuid::new_v4();
    let c = Some(Uuid::new_v4());
    let target = MetricSnapshot::from_research_item(&item(p, c, Uuid::new_v4(), &url(1), 10, Some(300))).unwrap();
    let peers: Vec<_> = (2..=4)
        .map(|n| MetricSnapshot::from_research_item(
            &item(p, c, Uuid::new_v4(), &url(n), 10, Some((n as i64 - 1) * 100))
        ).unwrap())
        .collect();
    let result = compare_creator_views(&target, &peers, ObservationTimestamp::parse_rfc3339("2026-11-13T18:00:00Z").unwrap(), 1).unwrap();
    match result {
        CreatorComparison::Observed { peer_count, median_peer_views, views_relative_to_peer_median, .. } => {
            assert_eq!(peer_count, 3);
            assert_eq!(median_peer_views, 200.0);
            assert_eq!(views_relative_to_peer_median, 1.5);
        }
        other => panic!("expected creator-relative result, got {other:?}"),
    }
    let mismatched_project = peers.iter().map(|peer| {
        let mut peer = peer.clone();
        peer.project_id = Uuid::new_v4();
        peer
    }).collect::<Vec<_>>();
    assert!(matches!(
        compare_creator_views(&target, &mismatched_project, ObservationTimestamp::parse_rfc3339("2026-11-13T18:00:00Z").unwrap(), 1).unwrap(),
        CreatorComparison::Unavailable {
            reason: ComparisonLimitation::InsufficientAgeMatchedPeers,
            eligible_peers: 0
        }
    ));
    let mut stale = target.clone();
    stale.observed_at = stale.observed_at.add_seconds(-70 * 86_400);
    assert!(matches!(
        compare_creator_views(&stale, &peers, ObservationTimestamp::parse_rfc3339("2026-11-13T18:00:00Z").unwrap(), 30).unwrap(),
        CreatorComparison::Unavailable { reason: ComparisonLimitation::StaleObservation, .. }
    ));
}

#[test]
fn missing_or_age_incompatible_peers_do_not_make_a_rank() {
    let p = Uuid::new_v4();
    let c = Some(Uuid::new_v4());
    let target = MetricSnapshot::from_research_item(&item(p, c, Uuid::new_v4(), &url(1), 5, Some(100))).unwrap();
    let peer = MetricSnapshot::from_research_item(&item(p, c, Uuid::new_v4(), &url(2), 150, Some(500))).unwrap();
    assert!(matches!(
        compare_creator_views(&target, &[peer], ObservationTimestamp::parse_rfc3339("2026-11-13T18:00:00Z").unwrap(), 1).unwrap(),
        CreatorComparison::Unavailable {
            reason: ComparisonLimitation::InsufficientAgeMatchedPeers,
            eligible_peers: 0
        }
    ));
    let mut unknown = target.clone();
    unknown.published = PublicationDate::Missing;
    assert!(matches!(
        compare_creator_views(&unknown, &[], ObservationTimestamp::parse_rfc3339("2026-11-13T18:00:00Z").unwrap(), 1).unwrap(),
        CreatorComparison::Unavailable { reason: ComparisonLimitation::UnknownUploadDate, .. }
    ));
}

#[test]
fn growth_requires_two_monotonic_timestamps_and_counters() {
    let p = Uuid::new_v4();
    let older = MetricSnapshot::from_research_item(&item(p, None, Uuid::new_v4(), &url(1), 8, Some(100))).unwrap();
    let mut newer = older.clone();
    newer.observed_at = newer.observed_at.add_seconds(12 * 3_600);
    newer.views = Some(150);
    let growth = observed_view_growth(&older, &newer).unwrap().unwrap();
    assert_eq!(growth.observed_view_increase, 50);
    assert_eq!(growth.views_per_day, 100.0);
    newer.views = Some(90);
    assert_eq!(
        observed_view_growth(&older, &newer).unwrap(),
        Err(GrowthLimitation::CountersDecreased)
    );
    newer.views = None;
    assert_eq!(
        observed_view_growth(&older, &newer).unwrap(),
        Err(GrowthLimitation::UnavailableCounts)
    );
    newer.views = Some(100);
    newer.observed_at = older.observed_at;
    assert_eq!(
        observed_view_growth(&older, &newer).unwrap(),
        Err(GrowthLimitation::NonIncreasingTime)
    );
}

#[test]
fn transcript_evidence_is_timecoded_limited_and_explicitly_nonvisual() {
    let scope = TranscriptScope {
        project_id: Uuid::new_v4(),
        research_item_id: Uuid::new_v4(),
        transcript_id: Uuid::new_v4(),
        transcript_revision: "fixture-hash-v1".into(),
    };
    let segments = vec![
        TranscriptSegment { segment_id: "1".into(), start_ms: 100, end_ms: 2500, text: "هل تعرف لماذا؟".into() },
        TranscriptSegment { segment_id: "2".into(), start_ms: 3000, end_ms: 7000, text: "Here is context".into() },
        TranscriptSegment { segment_id: "3".into(), start_ms: 8000, end_ms: 11000, text: "This is the ending".into() },
    ];
    let outline = extract_transcript_outline(&scope, &segments, SourceAccess::Partial).unwrap();
    assert_eq!(outline.observations.len(), 3);
    assert_eq!(outline.observations[0].beat, CreativeBeat::OpeningExcerpt);
    assert_eq!(outline.observations[0].evidence.start_ms, 100);
    assert_eq!(outline.observations[1].evidence.segment_id, "2");
    assert_eq!(outline.observations[2].beat, CreativeBeat::ClosingExcerpt);
    assert!(outline.observations.iter().all(|o| o.confidence == SourceConfidence::DirectTranscriptExcerpt));
    assert!(!outline.warnings.is_empty());
    assert!(matches!(
        extract_transcript_outline(&scope, &segments, SourceAccess::Denied),
        Err(IntelligenceError::SourceUnavailable(_))
    ));
    let mut invalid = segments;
    invalid[2].start_ms = 100;
    assert_eq!(
        extract_transcript_outline(&scope, &invalid, SourceAccess::Available).unwrap_err(),
        IntelligenceError::InvalidSegment
    );
}

#[test]
fn silent_transcripts_do_not_fabricate_hooks() {
    let scope = TranscriptScope {
        project_id: Uuid::new_v4(),
        research_item_id: Uuid::new_v4(),
        transcript_id: Uuid::new_v4(),
        transcript_revision: "rev".into(),
    };
    let result = extract_transcript_outline(&scope, &[], SourceAccess::Available).unwrap();
    assert!(result.observations.is_empty());
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn transcript_hypotheses_stay_uncertain_and_project_scoped() {
    let project = Uuid::new_v4();
    let scope = TranscriptScope {
        project_id: project,
        research_item_id: Uuid::new_v4(),
        transcript_id: Uuid::new_v4(),
        transcript_revision: "r1".into(),
    };
    let segment = TranscriptSegment {
        segment_id: "hook".into(),
        start_ms: 0,
        end_ms: 2000,
        text: "Opening phrase".into(),
    };
    let first = extract_transcript_outline(&scope, &[segment], SourceAccess::Available)
        .unwrap()
        .observations
        .remove(0)
        .evidence;
    let hook = propose_transcript_hypothesis(
        TranscriptFeature::Hook,
        "The spoken opening may be a question",
        vec![first.clone()],
    ).unwrap();
    assert_eq!(hook.confidence, SourceConfidence::InferredFromTranscript);
    assert!(hook.limitations[0].contains("frames"));
    let stats = count_transcript_features(project, &[hook.clone(), hook]).unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].sources_with_hypothesis, 1);
    assert_eq!(
        count_transcript_features(Uuid::new_v4(), &[]).unwrap().len(),
        0
    );
    assert_eq!(
        count_transcript_features(Uuid::new_v4(), &[propose_transcript_hypothesis(
            TranscriptFeature::Payoff, "Proposed", vec![first]
        ).unwrap()]).unwrap_err(),
        IntelligenceError::ProjectMismatch
    );
}

#[test]
fn conflicting_peer_observations_cannot_determine_creator_median() {
    let p = Uuid::new_v4();
    let c = Some(Uuid::new_v4());
    let target = MetricSnapshot::from_research_item(
        &item(p, c, Uuid::new_v4(), &url(1), 10, Some(300))
    ).unwrap();
    let a = MetricSnapshot::from_research_item(
        &item(p, c, Uuid::new_v4(), &url(2), 10, Some(100))
    ).unwrap();
    let mut b = a.clone();
    b.research_item_id = Uuid::new_v4();
    b.views = Some(200);
    assert_eq!(
        compare_creator_views(&target, &[a, b], ObservationTimestamp::parse_rfc3339("2026-11-13T18:00:00Z").unwrap(), 1).unwrap_err(),
        IntelligenceError::ContradictoryObservation
    );
}

#[test]
fn utc_offsets_and_calendar_validation_are_unambiguous() {
    let utc = ObservationTimestamp::parse_rfc3339("2026-11-13T10:00:00Z").unwrap();
    let offset = ObservationTimestamp::parse_rfc3339("2026-11-13T14:00:00+04:00").unwrap();
    assert_eq!(utc, offset);
    let fractional = ObservationTimestamp::parse_rfc3339("2026-11-13T10:00:00.123456789Z").unwrap();
    assert_eq!(fractional.unix_seconds, utc.unix_seconds);
    assert_eq!(fractional.nanosecond, 123456789);
    assert!(ObservationTimestamp::parse_rfc3339("2026-02-29T10:00:00Z").is_none());
    assert!(ObservationTimestamp::parse_rfc3339("2026-11-13T10:00:00+99:00").is_none());
    assert!(ObservationTimestamp::parse_rfc3339("2026-11-13T10:00:00").is_none());
    assert!(ObservationTimestamp::parse_rfc3339("2024-02-29T10:00:00Z").is_some());
}
