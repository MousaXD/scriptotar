//! Advisory offline checks. These heuristics cannot establish accuracy,
//! originality, legal compliance, or predicted engagement.
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    AngleOption, BeatRole, ClaimStatus, CreativeBrief, EvidenceKind, EvidenceRef, EvidenceStatus,
    ScriptDraft,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryLevel {
    Info,
    Warning,
    NeedsReview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryKind {
    GenericHook,
    Repetition,
    DurationMismatch,
    MissingCitation,
    UnsupportedClaim,
    UncertainLanguage,
    PotentialSourceOverlap,
    CtaMismatch,
    SimilarAngles,
    IncompleteClaimInventory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Advisory {
    pub kind: AdvisoryKind,
    pub level: AdvisoryLevel,
    pub beat_id: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DurationEstimate {
    pub words: usize,
    pub approximate_seconds: u32,
    pub assumed_words_per_minute: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DraftAudit {
    pub duration: DurationEstimate,
    /// Advisory, never an engagement, copyright, or correctness score.
    pub advisories: Vec<Advisory>,
}

/// Offline review that makes no network calls and does not mutate its inputs.
/// Only annotated factual claims can be assessed; others require human review.
pub fn audit_draft(
    draft: &ScriptDraft,
    brief: &CreativeBrief,
    evidence: &[EvidenceRef],
) -> DraftAudit {
    let words = draft
        .beats
        .iter()
        .map(|beat| tokens(&beat.narration).len())
        .sum::<usize>();
    let arabic = brief.language.to_lowercase().contains("arab")
        || brief.language.trim().eq_ignore_ascii_case("ar")
        || brief.language.contains("عرب");
    let rate: u16 = if arabic { 125 } else { 150 };
    let duration = DurationEstimate {
        words,
        approximate_seconds: ((words as u64 * 60).div_ceil(u64::from(rate)))
            .min(u64::from(u32::MAX)) as u32,
        assumed_words_per_minute: rate,
    };
    let mut advisories = Vec::new();

    if let Some(hook) = draft.beats.iter().find(|beat| beat.role == BeatRole::Hook) {
        let opening = hook.narration.trim().to_lowercase();
        let generic_starts = [
            "in this video",
            "did you know",
            "hey guys",
            "today we're",
            "let's talk about",
            "في هذا الفيديو",
            "هل تعلم",
            "اليوم سنتحدث",
        ];
        if tokens(&opening).len() < 6
            || generic_starts
                .iter()
                .any(|phrase| opening.starts_with(phrase))
        {
            advisories.push(note(
                AdvisoryKind::GenericHook,
                AdvisoryLevel::Warning,
                Some(&hook.id),
                "Opening may be too generic or short; review its specific promise.",
            ));
        }
    } else {
        advisories.push(note(
            AdvisoryKind::GenericHook,
            AdvisoryLevel::Warning,
            None,
            "No explicit opening hook is marked in the draft.",
        ));
    }

    for (position, left) in draft.beats.iter().enumerate() {
        for right in draft.beats.iter().skip(position + 1) {
            let first = tokens(&left.narration);
            let second = tokens(&right.narration);
            if first.len() >= 5 && second.len() >= 5 && similarity(&first, &second) > 0.80 {
                advisories.push(note(
                    AdvisoryKind::Repetition,
                    AdvisoryLevel::Warning,
                    Some(&right.id),
                    "This beat repeats much of an earlier beat.",
                ));
            }
        }
    }

    let target = u32::from(brief.duration_seconds);
    if target > 0
        && (duration.approximate_seconds > target.saturating_mul(14) / 10
            || duration.approximate_seconds.saturating_mul(10) < target.saturating_mul(6))
    {
        advisories.push(note(
            AdvisoryKind::DurationMismatch,
            AdvisoryLevel::Info,
            None,
            "Estimated narration time differs considerably from the target; speaking speed and pauses vary.",
        ));
    }

    if draft.claims.is_empty() {
        advisories.push(note(
            AdvisoryKind::IncompleteClaimInventory,
            AdvisoryLevel::Info,
            None,
            "No factual claims are annotated. Unannotated claims cannot be checked by this review.",
        ));
    }

    for claim in &draft.claims {
        let support = claim.evidence_ids.iter().any(|id| {
            evidence.iter().any(|item| {
                item.id == *id
                    && item.kind == EvidenceKind::IndependentDocument
                    && item.status == EvidenceStatus::Corroborated
            })
        });
        match claim.status {
            ClaimStatus::Supported if claim.evidence_ids.is_empty() => advisories.push(note(
                AdvisoryKind::MissingCitation,
                AdvisoryLevel::NeedsReview,
                Some(&claim.beat_id),
                "Supported factual claim needs a citation.",
            )),
            ClaimStatus::Supported if !support => advisories.push(note(
                AdvisoryKind::UnsupportedClaim,
                AdvisoryLevel::NeedsReview,
                Some(&claim.beat_id),
                "Claim labeled supported but has no declared corroborated independent reference.",
            )),
            ClaimStatus::Unverified | ClaimStatus::Disputed => {
                let beat_text = draft
                    .beats
                    .iter()
                    .find(|beat| beat.id == claim.beat_id)
                    .map(|beat| beat.narration.to_lowercase())
                    .unwrap_or_default();
                let hedges = [
                    "may",
                    "might",
                    "possibly",
                    "reportedly",
                    "unverified",
                    "according to",
                    "not confirmed",
                    "قد",
                    "ربما",
                    "بحسب",
                    "غير مؤكد",
                    "يُقال",
                ];
                if !hedges.iter().any(|hedge| beat_text.contains(hedge)) {
                    advisories.push(note(
                        AdvisoryKind::UncertainLanguage,
                        AdvisoryLevel::Warning,
                        Some(&claim.beat_id),
                        "An unverified or disputed claim may read as certain; qualify or remove it.",
                    ));
                }
            }
            _ => {}
        }
    }

    if let Some(expected) = &brief.desired_cta {
        if !expected.trim().is_empty() {
            match draft.cta.as_deref() {
                None | Some("") => advisories.push(note(
                    AdvisoryKind::CtaMismatch,
                    AdvisoryLevel::Warning,
                    None,
                    "The brief requests a CTA, but the draft has none.",
                )),
                Some(actual) => {
                    if similarity(&tokens(expected), &tokens(actual)) < 0.10 {
                        advisories.push(note(
                            AdvisoryKind::CtaMismatch,
                            AdvisoryLevel::Info,
                            None,
                            "CTA wording may not align with the requested action; review manually.",
                        ));
                    }
                }
            }
        }
    }

    // Compare token windows, not arbitrary individual common words. A match is
    // a review cue only, not a copyright or plagiarism determination.
    let mut source_windows = HashSet::new();
    for item in evidence {
        if matches!(
            item.kind,
            EvidenceKind::SocialTranscript | EvidenceKind::IndependentDocument
        ) {
            let source = tokens(&item.excerpt);
            for group in source.windows(8) {
                source_windows.insert(group.join(" "));
            }
        }
    }
    for beat in &draft.beats {
        let narration = tokens(&beat.narration);
        if narration
            .windows(8)
            .any(|group| source_windows.contains(&group.join(" ")))
        {
            advisories.push(note(
                AdvisoryKind::PotentialSourceOverlap,
                AdvisoryLevel::NeedsReview,
                Some(&beat.id),
                "Eight consecutive normalized words match a source excerpt; check for distinctive copied phrasing.",
            ));
        }
    }

    DraftAudit {
        duration,
        advisories,
    }
}

/// Detect similar hooks/angles. Similarity is lexical and advisory; different
/// words can still express the same idea and identical terms can be appropriate.
pub fn compare_angles(angles: &[AngleOption]) -> Vec<Advisory> {
    let mut findings = Vec::new();
    for (index, left) in angles.iter().enumerate() {
        for right in angles.iter().skip(index + 1) {
            let hook_similarity = similarity(&tokens(&left.hook), &tokens(&right.hook));
            let approach_similarity = similarity(&tokens(&left.approach), &tokens(&right.approach));
            if hook_similarity > 0.65 || (hook_similarity > 0.35 && approach_similarity > 0.65) {
                findings.push(note(
                    AdvisoryKind::SimilarAngles,
                    AdvisoryLevel::Warning,
                    None,
                    "Two proposed approaches share similar hook/angle wording; ask for a different mechanism.",
                ));
            }
        }
    }
    findings
}

fn note(
    kind: AdvisoryKind,
    level: AdvisoryLevel,
    beat_id: Option<&str>,
    description: &str,
) -> Advisory {
    Advisory {
        kind,
        level,
        beat_id: beat_id.map(str::to_owned),
        description: description.to_owned(),
    }
}

fn tokens(input: &str) -> Vec<String> {
    input
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn similarity(left: &[String], right: &[String]) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let first: HashSet<&str> = left.iter().map(String::as_str).collect();
    let second: HashSet<&str> = right.iter().map(String::as_str).collect();
    let union = first.union(&second).count();
    if union == 0 {
        0.0
    } else {
        first.intersection(&second).count() as f64 / union as f64
    }
}
