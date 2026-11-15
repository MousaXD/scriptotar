use super::*;

fn locator(modality: Modality) -> ExactLocator {
    ExactLocator {
        locator: "Section 2, paragraph 3".into(),
        segment_id: (modality == Modality::SpokenTranscript).then(|| "seg-17".into()),
        timestamp_millis: (modality == Modality::SpokenTranscript).then_some(12_500),
        modality,
    }
}

fn source(id: &str, origin: &str) -> ResolvedEvidence {
    ResolvedEvidence {
        evidence_id: id.into(),
        source_id: format!("study-source-{id}"),
        project_id: "project-a".into(),
        study_id: "study-a".into(),
        source_version: Some("sha256:version1".into()),
        locator: locator(Modality::DocumentText),
        excerpt: format!("Verified excerpt for {id}"),
        kind: SourceKind::IndependentDocument,
        state: EvidenceState::Reviewed,
        relation: EvidenceRelation::Supports,
        independent_origin: Some(origin.into()),
        collected_at_epoch: Some(100),
        expires_at_epoch: Some(1000),
    }
}

fn citation(src: &ResolvedEvidence) -> Citation {
    Citation {
        evidence_id: src.evidence_id.clone(),
        source_id: src.source_id.clone(),
        source_version: src.source_version.clone().unwrap_or_default(),
        locator: src.locator.clone(),
        quote: src.excerpt.clone(),
    }
}

fn fact(evidence: &[ResolvedEvidence]) -> ClaimProposal {
    ClaimProposal {
        id: "c1".into(),
        beat_id: "beat-1".into(),
        statement: "The relevant fictional regulation took effect in 2025".into(),
        kind: ClaimKind::Fact,
        consequence: Consequence::Ordinary,
        required_modality: Modality::DocumentText,
        citations: evidence.iter().map(citation).collect(),
        model_claims_verified: true,
    }
}

fn scope(evidence: Vec<ResolvedEvidence>) -> VerificationContext {
    VerificationContext {
        project_id: "project-a".into(),
        study_id: "study-a".into(),
        draft_id: "draft-a".into(),
        draft_revision: 5,
        checked_at_epoch: 200,
        evidence,
        human_reviews: vec![],
    }
}

fn review(ctx: &VerificationContext, claim: &ClaimProposal) -> VerifiedReview {
    VerifiedReview {
        review_id: "review-1".into(),
        reviewer_id: "human-independent-auditor".into(),
        project_id: ctx.project_id.clone(),
        study_id: ctx.study_id.clone(),
        draft_id: ctx.draft_id.clone(),
        draft_revision: ctx.draft_revision,
        claim_id: claim.id.clone(),
        exact_statement: claim.statement.clone(),
        evidence_ids: claim.citations.iter().map(|c| c.evidence_id.clone()).collect(),
        exact_citations: claim.citations.clone(),
        decision: ReviewDecision::Accept,
        reviewed_at_epoch: 199,
    }
}

fn evaluated(ctx: &VerificationContext, claim: &ClaimProposal) -> ClaimResult {
    verify_claims(ctx, &[claim.clone()]).claims.remove(0)
}

fn has(result: &ClaimResult, code: FindingCode) -> bool {
    result.findings.iter().any(|f| f.code == code)
}

#[test]
fn two_independent_reviewed_sources_and_exact_current_human_decision_are_required() {
    let mut ctx = scope(vec![source("a", "publisher-a"), source("b", "publisher-b")]);
    let claim = fact(&ctx.evidence);
    let unreviewed = evaluated(&ctx, &claim);
    assert_eq!(unreviewed.status, VerificationStatus::Unverified);
    assert!(has(&unreviewed, FindingCode::NotIndependentlyReviewed));
    assert!(has(&unreviewed, FindingCode::ModelPromotionAttempt));
    ctx.human_reviews.push(review(&ctx, &claim));
    let verified = evaluated(&ctx, &claim);
    assert_eq!(verified.status, VerificationStatus::Verified);
    assert_eq!(verified.exact_citations, claim.citations);
    // Individual claim verification is not an attested exhaustive prose audit.
    let report = verify_claims(&ctx, &[claim]);
    assert!(report.publication_review_required);
    assert!(!report.inventory_complete);
}

#[test]
fn model_claim_of_corroboration_does_not_verify() {
    let mut ctx = scope(vec![source("a", "same-publisher"), source("b", "same-publisher")]);
    let claim = fact(&ctx.evidence);
    ctx.human_reviews.push(review(&ctx, &claim));
    let result = evaluated(&ctx, &claim);
    assert_eq!(result.status, VerificationStatus::Unverified);
    assert!(has(&result, FindingCode::InsufficientIndependentSources));
}

#[test]
fn unknown_and_forged_citation_fail_closed() {
    let ctx = scope(vec![source("a", "one")]);
    let mut claim = fact(&ctx.evidence);
    claim.citations[0].evidence_id = "invented-evidence".into();
    let result = evaluated(&ctx, &claim);
    assert_eq!(result.status, VerificationStatus::Blocked);
    assert!(has(&result, FindingCode::UnknownCitation));
    claim = fact(&ctx.evidence);
    claim.citations[0].quote.push_str(" extra");
    let result = evaluated(&ctx, &claim);
    assert_eq!(result.status, VerificationStatus::Blocked);
    assert!(has(&result, FindingCode::ForgedLocatorOrQuote));
    claim = fact(&ctx.evidence);
    claim.citations[0].source_id = "wrong-source".into();
    assert!(has(&evaluated(&ctx, &claim), FindingCode::SourceMismatch));
}

#[test]
fn foreign_project_and_study_sources_never_pass() {
    for foreign_project in [true, false] {
        let mut src = source("a", "owner");
        if foreign_project { src.project_id = "project-b".into(); }
        else { src.study_id = "study-b".into(); }
        let ctx = scope(vec![src]);
        let result = evaluated(&ctx, &fact(&ctx.evidence));
        assert_eq!(result.status, VerificationStatus::Blocked);
        assert!(has(&result, FindingCode::CrossProjectReference));
    }
}

#[test]
fn disputed_revoked_untrusted_missing_and_unversioned_source_rejected() {
    let cases = [
        (EvidenceState::Disputed, FindingCode::DisputedEvidence),
        (EvidenceState::Revoked, FindingCode::RevokedSource),
        (EvidenceState::Untrusted, FindingCode::UntrustedSource),
        (EvidenceState::Unavailable, FindingCode::UnavailableSource),
    ];
    for (state, code) in cases {
        let mut src = source("a", "owner");
        src.state = state;
        let ctx = scope(vec![src]);
        let result = evaluated(&ctx, &fact(&ctx.evidence));
        assert_eq!(result.status, VerificationStatus::Blocked);
        assert!(has(&result, code));
    }
    let mut src = source("a", "owner");
    src.source_version = None;
    let ctx = scope(vec![src]);
    assert!(has(&evaluated(&ctx, &fact(&ctx.evidence)), FindingCode::UnversionedSource));
}

#[test]
fn stale_source_version_and_outdated_evidence_never_verify() {
    let mut ctx = scope(vec![source("a", "owner-a"), source("b", "owner-b")]);
    let mut claim = fact(&ctx.evidence);
    ctx.human_reviews.push(review(&ctx, &claim));
    claim.citations[0].source_version = "sha256:old".into();
    let result = evaluated(&ctx, &claim);
    assert_eq!(result.status, VerificationStatus::Blocked);
    assert!(has(&result, FindingCode::StaleSourceVersion));
    claim = fact(&ctx.evidence);
    ctx.evidence[0].expires_at_epoch = Some(199);
    let result = evaluated(&ctx, &claim);
    assert_eq!(result.status, VerificationStatus::Unverified);
    assert!(has(&result, FindingCode::OutdatedSource));
}

#[test]
fn transcript_does_not_prove_visual_claim_or_uncited_spoken_segment() {
    let mut transcript = source("a", "speaker");
    transcript.kind = SourceKind::SocialTranscript;
    transcript.locator = locator(Modality::SpokenTranscript);
    let ctx = scope(vec![transcript.clone()]);
    let mut claim = fact(&ctx.evidence);
    claim.required_modality = Modality::VisualFrame;
    assert!(has(&evaluated(&ctx, &claim), FindingCode::WrongEvidenceModality));
    let mut unknown_time = transcript.clone();
    unknown_time.locator.segment_id = None;
    let ctx = scope(vec![unknown_time]);
    claim = fact(&ctx.evidence);
    claim.required_modality = Modality::SpokenTranscript;
    assert!(has(&evaluated(&ctx, &claim), FindingCode::MissingSegmentOrTimestamp));
}

#[test]
fn contradiction_beats_review_and_a_model_success_label() {
    let mut ctx = scope(vec![source("a", "owner-a"), source("b", "owner-b")]);
    let claim = fact(&ctx.evidence);
    ctx.human_reviews.push(review(&ctx, &claim));
    ctx.evidence[0].relation = EvidenceRelation::Contradicts;
    let result = evaluated(&ctx, &claim);
    assert_eq!(result.status, VerificationStatus::Contradicted);
    assert!(has(&result, FindingCode::ContradictoryEvidence));
}

#[test]
fn consequential_claims_need_a_current_human_review_even_with_two_sources() {
    let ctx = scope(vec![source("a", "owner-a"), source("b", "owner-b")]);
    let mut claim = fact(&ctx.evidence);
    for consequence in [Consequence::Legal, Consequence::Financial, Consequence::Health, Consequence::OtherHighStakes] {
        claim.consequence = consequence;
        let result = evaluated(&ctx, &claim);
        assert_eq!(result.status, VerificationStatus::Unverified);
        assert!(has(&result, FindingCode::ConsequentialHumanReviewRequired));
    }
}

#[test]
fn revision_lineage_remains_inspectable_but_cannot_promote_stale_review() {
    let mut ctx = scope(vec![source("a", "owner-a"), source("b", "owner-b")]);
    let claim = fact(&ctx.evidence);
    ctx.human_reviews.push(review(&ctx, &claim));
    assert_eq!(evaluated(&ctx, &claim).status, VerificationStatus::Verified);
    ctx.draft_revision = 6;
    let report = verify_claims(&ctx, &[claim]);
    assert_eq!(report.claims[0].status, VerificationStatus::Unverified);
    assert!(has(&report.claims[0], FindingCode::InvalidReviewLineage));
    assert_eq!(report.review_lineage.len(), 1);
    assert_eq!(report.review_lineage[0].draft_revision, 5);
}

#[test]
fn incomplete_or_untrusted_records_cannot_promote() {
    let mut ctx = scope(vec![source("a", "one"), source("b", "two")]);
    let claim = fact(&ctx.evidence);
    ctx.human_reviews.push(review(&ctx, &claim));
    ctx.evidence[1].state = EvidenceState::Unreviewed;
    assert_eq!(evaluated(&ctx, &claim).status, VerificationStatus::Unverified);
    ctx.evidence[1].state = EvidenceState::Reviewed;
    ctx.evidence[1].collected_at_epoch = None;
    assert!(has(&evaluated(&ctx, &claim), FindingCode::UnknownFreshness));
}

#[test]
fn generated_fact_claim_extraction_never_invents_verified_citations() {
    use crate::writing::{BeatRole, FactualClaim, ScriptBeat, ScriptDraft};
    let draft = ScriptDraft {
        id: "d".into(), angle_id: "a".into(), title: "Test".into(),
        language: "en".into(),
        beats: vec![
            ScriptBeat {
                id: "b1".into(), role: BeatRole::Hook, narration: "Claim one".into(),
                on_screen_text: None, shots: vec![], evidence_ids: vec![],
                manually_edited: false,
            },
            ScriptBeat {
                id: "b2".into(), role: BeatRole::Payoff, narration: "Unclassified statement".into(),
                on_screen_text: None, shots: vec![], evidence_ids: vec![],
                manually_edited: false,
            },
        ],
        cta: None, caption: None,
        claims: vec![FactualClaim {
            id: "c1".into(), beat_id: "b1".into(),
            assertion: "Claim one".into(), status: ClaimStatus::Supported,
            evidence_ids: vec!["model-invented".into()],
        }],
    };
    let proposals = extract_draft_claims(&draft);
    assert_eq!(proposals.len(), 2);
    assert!(proposals[0].model_claims_verified);
    assert!(proposals[0].citations.is_empty());
    assert_eq!(proposals[1].kind, ClaimKind::UnsupportedStatement);
    let result = verify_claims(&scope(vec![]), &proposals);
    assert!(result.publication_review_required);
    assert!(result.claims.iter().all(|r| r.status != VerificationStatus::Verified));
}

#[test]
fn duplicate_evidence_ids_are_ambiguous_not_arbitrarily_selected() {
    let ctx = scope(vec![source("a", "one"), source("a", "two")]);
    assert!(has(&evaluated(&ctx, &fact(&ctx.evidence[..1])), FindingCode::UnknownCitation));
}


#[test]
fn old_review_cannot_verify_a_new_source_version_even_with_updated_citations() {
    let mut ctx = scope(vec![source("a", "publisher-a"), source("b", "publisher-b")]);
    let original = fact(&ctx.evidence);
    ctx.human_reviews.push(review(&ctx, &original));
    assert_eq!(evaluated(&ctx, &original).status, VerificationStatus::Verified);
    // Underlying evidence identities remain unchanged, but the second content
    // version and quotation have changed since the reviewer saw them.
    ctx.evidence[1].source_version = Some("sha256:version2".into());
    ctx.evidence[1].excerpt = "Updated source excerpt".into();
    let updated = fact(&ctx.evidence);
    let result = evaluated(&ctx, &updated);
    assert_eq!(result.status, VerificationStatus::Unverified);
    assert!(has(&result, FindingCode::InvalidReviewLineage));
    assert_eq!(ctx.human_reviews[0].exact_citations, original.citations);
}

#[test]
fn missing_claim_inventory_and_model_opinion_cannot_clear_publication_review() {
    let ctx = scope(vec![]);
    let empty = verify_claims(&ctx, &[]);
    assert!(empty.publication_review_required);
    assert!(!empty.inventory_complete);

    let opinion = ClaimProposal {
        id: "opinion-1".into(), beat_id: "beat-1".into(),
        statement: "This new product is always the safest choice".into(),
        kind: ClaimKind::Opinion, consequence: Consequence::Health,
        required_modality: Modality::DocumentText, citations: vec![],
        model_claims_verified: false,
    };
    let report = verify_claims(&ctx, &[opinion]);
    assert_eq!(report.claims[0].status, VerificationStatus::Opinion);
    assert!(report.publication_review_required);
    assert!(!report.inventory_complete);
}
