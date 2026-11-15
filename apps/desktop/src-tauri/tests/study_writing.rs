//! Controlled, offline integration contract tests for the new service module.
//! The app's central services registration remains owned by Agent 00.
#[allow(dead_code)]
#[path = "../src/services/study_writing.rs"]
mod study_writing;

use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use scriptotar_ai::{
    writing::{
        BeatRole, CreativeBrief, EvidenceKind, EvidenceLocator, EvidenceRef, EvidenceStatus,
    },
    AiRequest, AiResponse,
};
use serde_json::json;
use study_writing::{
    apply_reviewed_beat_revision, copy_angles_prompt, export_candidate, generate_three,
    prepare_three, AtomicCandidateStore, CurrentStudy, DurablePermissionBoundary, GeneratedBatch,
    GenerationPlan, SelectedWritingSource, StepAdmission, StopSignal, StudyWritingContext,
    WriteResult, WritingTextProvider, WritingWorkflowError,
};
use tempfile::TempDir;
use uuid::Uuid;

fn context() -> StudyWritingContext {
    let ids = [
        (Uuid::new_v4(), Uuid::new_v4()),
        (Uuid::new_v4(), Uuid::new_v4()),
        (Uuid::new_v4(), Uuid::new_v4()),
    ];
    StudyWritingContext {
        project_id: Uuid::new_v4(),
        study_id: Uuid::new_v4(),
        study_revision: 5,
        sources: ids
            .iter()
            .map(|(study_source_id, origin_id)| SelectedWritingSource {
                study_source_id: *study_source_id,
                origin_id: *origin_id,
            })
            .collect(),
        brief: CreativeBrief {
            goal: "An original community explainer".into(),
            audience: "neighborhood readers".into(),
            platform: "reels".into(),
            language: "en".into(),
            region: None,
            dialect: None,
            tone: Some("clear".into()),
            duration_seconds: 40,
            objective: "inform with clear limits".into(),
            desired_cta: None,
            constraints: vec!["Do not invent engagement metrics".into()],
            style_guide: None,
        },
        evidence: ids
            .iter()
            .enumerate()
            .map(|(i, (_, origin_id))| EvidenceRef {
                id: format!("e{}", i + 1),
                locator: EvidenceLocator {
                    source_id: origin_id.to_string(),
                    canonical_url: None,
                    timestamp_seconds: Some((i * 17) as u32),
                    document_section: Some(format!("Transcript segment {}", i + 1)),
                    collected_at: Some("2026-11-01T10:00:00Z".into()),
                },
                excerpt: format!(
                    "Fictional source {} discussing a community gathering",
                    i + 1
                ),
                kind: EvidenceKind::SocialTranscript,
                status: EvidenceStatus::Attributed,
            })
            .collect(),
        findings: vec![],
    }
}

fn angles_json() -> String {
    json!({ "angles": [
        { "id":"contrast", "approach":"contrast two options", "hook":"One neighborhood, two very different routes", "promise":"See a surprising choice", "outline":[{"role":"hook","intent":"contrast","estimated_seconds":10}], "evidence_ids":["e1"] },
        { "id":"journey", "approach":"first person diary", "hook":"The notebook I carried changed my walking plan", "promise":"A human journey", "outline":[{"role":"hook","intent":"journey","estimated_seconds":12}], "evidence_ids":["e2"] },
        { "id":"guide", "approach":"practical how to", "hook":"Build a practical checklist in just three steps", "promise":"Actionable tips", "outline":[{"role":"hook","intent":"guide","estimated_seconds":9}], "evidence_ids":["e3"] }
    ]}).to_string()
}

fn draft_json(index: usize) -> String {
    let (angle, evidence, body) = match index {
        1 => (
            "contrast",
            "e1",
            "Two paths tell entirely different stories about this fictional town.",
        ),
        2 => (
            "journey",
            "e2",
            "I found a small notebook and followed its handwritten walking notes.",
        ),
        _ => (
            "guide",
            "e3",
            "First outline your route, next check the details, then share a plan.",
        ),
    };
    json!({
        "id":format!("script-{index}"), "angle_id":angle,
        "title": format!("Original route idea {index}"), "language":"en",
        "beats":[
            {"id":"hook","role":"hook","narration":body,"on_screen_text":null,
             "shots":[],"evidence_ids":[evidence],"manually_edited":true},
            {"id":"cta","role":"cta","narration":"Invite a neighbor to discuss the idea.",
             "on_screen_text":null,"shots":[],"evidence_ids":[],"manually_edited":false}
        ],
        "cta":"Share your opinion", "caption":"Original script test",
        "claims":[{"id":"claim-a","beat_id":"hook","assertion":"A source talks about a gathering",
                    "status":"unverified","evidence_ids":[evidence]}]
    })
    .to_string()
}

struct FixtureProvider {
    responses: VecDeque<String>,
    calls: usize,
    on_call: Option<Box<dyn Fn(usize)>>,
    fail_on: Option<usize>,
}
impl FixtureProvider {
    fn good() -> Self {
        Self {
            responses: [angles_json(), draft_json(1), draft_json(2), draft_json(3)].into(),
            calls: 0,
            on_call: None,
            fail_on: None,
        }
    }
}
impl WritingTextProvider for FixtureProvider {
    fn generate(&mut self, _request: &AiRequest) -> Result<AiResponse, ()> {
        self.calls += 1;
        if let Some(callback) = &self.on_call {
            callback(self.calls);
        }
        if self.fail_on == Some(self.calls) {
            return Err(());
        }
        self.responses
            .pop_front()
            .map(|text| AiResponse { text })
            .ok_or(())
    }
}

struct FixtureBoundary {
    allowed: bool,
    step_response: HashMap<String, String>,
    pending: HashSet<String>,
    uncertain: HashSet<String>,
    attempts: usize,
}
impl FixtureBoundary {
    fn new() -> Self {
        Self {
            allowed: true,
            step_response: HashMap::new(),
            pending: HashSet::new(),
            uncertain: HashSet::new(),
            attempts: 0,
        }
    }
}
impl DurablePermissionBoundary for FixtureBoundary {
    fn begin(
        &mut self,
        run_id: &str,
        _context: &StudyWritingContext,
        step: &str,
        request: &AiRequest,
    ) -> WriteResult<StepAdmission> {
        self.attempts += 1;
        if !self.allowed {
            return Err(WritingWorkflowError::Denied("approval required".into()));
        }
        // This fixture key binds the prompt as well as the step/run, so a
        // changed prompt cannot reuse a receipt from another context.
        let key = format!("{run_id}:{step}:{}", request.prompt);
        if self.uncertain.contains(&key) || self.pending.contains(&key) {
            return Err(WritingWorkflowError::UncertainEffect);
        }
        if let Some(response) = self.step_response.get(&key) {
            return Ok(StepAdmission::Recorded {
                response_json: response.clone(),
            });
        }
        self.pending.insert(key.clone());
        Ok(StepAdmission::Dispatch { effect_key: key })
    }
    fn record_response(&mut self, effect_key: &str, response_json: &str) -> WriteResult<()> {
        assert!(self.pending.remove(effect_key));
        self.step_response
            .insert(effect_key.into(), response_json.into());
        Ok(())
    }
    fn mark_uncertain(&mut self, effect_key: &str) -> WriteResult<()> {
        self.pending.remove(effect_key);
        self.uncertain.insert(effect_key.into());
        Ok(())
    }
    fn abandon_before_dispatch(&mut self, effect_key: &str) -> WriteResult<()> {
        self.pending.remove(effect_key);
        self.uncertain.insert(effect_key.into());
        Ok(())
    }
}

struct MutableStudy(Rc<RefCell<StudyWritingContext>>);
impl CurrentStudy for MutableStudy {
    fn load(&self, project_id: Uuid, study_id: Uuid) -> WriteResult<StudyWritingContext> {
        let ctx = self.0.borrow().clone();
        if ctx.project_id != project_id || ctx.study_id != study_id {
            return Err(WritingWorkflowError::InvalidContext("foreign project"));
        }
        Ok(ctx)
    }
}
struct TestStop(Rc<Cell<bool>>);
impl StopSignal for TestStop {
    fn cancelled(&self) -> bool {
        self.0.get()
    }
}

/// An independent test-only transaction sink demonstrating the contract.
/// Production uses the existing canonical Studio tables and a future
/// Agent 00/02 approved atomic batch/effect receipt adapter.
struct SqlFixtureSink {
    db: Connection,
    current: Rc<RefCell<StudyWritingContext>>,
    fail_at: Option<usize>,
}
impl SqlFixtureSink {
    fn open(path: &std::path::Path, current: Rc<RefCell<StudyWritingContext>>) -> Self {
        let db = Connection::open(path).unwrap();
        db.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS fixture_batch (
               run_id TEXT PRIMARY KEY, project_id TEXT NOT NULL, study_id TEXT NOT NULL,
               context_json TEXT NOT NULL, candidate_ids TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS fixture_candidate (
               draft_id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES fixture_batch(run_id),
               ordinal INTEGER NOT NULL, version INTEGER NOT NULL, content_json TEXT NOT NULL,
               UNIQUE(run_id,ordinal));",
        )
        .unwrap();
        Self {
            db,
            current,
            fail_at: None,
        }
    }
    fn count(&self) -> i64 {
        self.db
            .query_row("SELECT COUNT(*) FROM fixture_candidate", [], |row| {
                row.get(0)
            })
            .unwrap()
    }
    fn get(&self, id: Uuid) -> GeneratedBatch {
        let raw: String = self.db.query_row(
            "SELECT context_json FROM fixture_batch b JOIN fixture_candidate d ON d.run_id=b.run_id WHERE d.draft_id=?1",
            params![id.to_string()],|row|row.get(0)).unwrap();
        serde_json::from_str(&raw).unwrap()
    }
}
impl AtomicCandidateStore for SqlFixtureSink {
    fn commit_three(&mut self, batch: &GeneratedBatch) -> WriteResult<[Uuid; 3]> {
        if self.current.borrow().clone() != batch.context {
            return Err(WritingWorkflowError::StaleContext);
        }
        if batch.candidates.len() != 3 {
            return Err(WritingWorkflowError::InvalidContext("batch size"));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?;
        let saved: Option<String> = tx.query_row(
            "SELECT candidate_ids FROM fixture_batch WHERE run_id=?1 AND project_id=?2 AND study_id=?3",
            params![batch.run_id,batch.context.project_id.to_string(),batch.context.study_id.to_string()],
            |row|row.get(0),
        ).optional().map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?;
        if let Some(raw) = saved {
            let values: [Uuid; 3] = serde_json::from_str(&raw).unwrap();
            return Ok(values);
        }
        let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let all = serde_json::to_string(batch).unwrap();
        tx.execute("INSERT INTO fixture_batch(run_id,project_id,study_id,context_json,candidate_ids) VALUES(?1,?2,?3,?4,?5)",
           params![batch.run_id,batch.context.project_id.to_string(),batch.context.study_id.to_string(),
               all,serde_json::to_string(&ids).unwrap()]
        ).map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?;
        for (i, candidate) in batch.candidates.iter().enumerate() {
            if self.fail_at == Some(i) {
                return Err(WritingWorkflowError::PersistFailed(
                    "injected SQL failure".into(),
                ));
            }
            tx.execute("INSERT INTO fixture_candidate(draft_id,run_id,ordinal,version,content_json) VALUES(?1,?2,?3,1,?4)",
                params![ids[i].to_string(),batch.run_id,i as i64,serde_json::to_string(candidate).unwrap()]
            ).map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| WritingWorkflowError::PersistFailed(e.to_string()))?;
        Ok(ids)
    }
}

fn setup() -> (
    TempDir,
    GenerationPlan,
    MutableStudy,
    FixtureBoundary,
    FixtureProvider,
    TestStop,
) {
    let temp = TempDir::new().unwrap();
    let c = context();
    let plan = prepare_three("run-one", c.clone()).unwrap();
    let current = MutableStudy(Rc::new(RefCell::new(c)));
    let boundary = FixtureBoundary::new();
    let provider = FixtureProvider::good();
    let stop = TestStop(Rc::new(Cell::new(false)));
    (temp, plan, current, boundary, provider, stop)
}

#[test]
fn three_distinct_source_linked_candidates_persist_reopen_export_without_rebilling() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let path = temp.path().join("test.sqlite");
    assert!(copy_angles_prompt(&plan).contains("Propose exactly 3"));
    assert_eq!(provider.calls, 0);
    let mut sink = SqlFixtureSink::open(&path, current.0.clone());
    let ids = generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink).unwrap();
    assert_eq!(provider.calls, 4);
    assert_eq!(sink.count(), 3);
    let captured = sink.get(ids[0]);
    assert_eq!(captured.candidates.len(), 3);
    for (i, candidate) in captured.candidates.iter().enumerate() {
        assert_eq!(candidate.draft.angle_id, candidate.angle.id);
        assert_eq!(candidate.lineage.len(), 1);
        assert_eq!(
            candidate.lineage[0].source_id,
            plan.context.sources[i].study_source_id
        );
        assert!(export_candidate(candidate).contains("Transcript segment"));
        assert!(study_writing::needs_human_fact_review(candidate));
        assert!(candidate
            .draft
            .beats
            .iter()
            .all(|beat| !beat.manually_edited));
    }
    drop(sink);
    let mut reopened = SqlFixtureSink::open(&path, current.0.clone());
    assert_eq!(reopened.count(), 3);
    assert_eq!(reopened.get(ids[0]).candidates, captured.candidates);
    let again = generate_three(
        &plan,
        &current,
        &mut gate,
        &mut provider,
        &stop,
        &mut reopened,
    )
    .unwrap();
    assert_eq!(again, ids);
    assert_eq!(provider.calls, 4);
    assert_eq!(reopened.count(), 3);
}

#[test]
fn denied_approval_does_not_contact_provider_or_persist() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    gate.allowed = false;
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert!(matches!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::Denied(_))
    ));
    assert_eq!(provider.calls, 0);
    assert_eq!(sink.count(), 0);
}

#[test]
fn external_provider_failure_is_uncertain_and_must_not_replay() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    provider.fail_on = Some(2);
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::ProviderFailed)
    );
    assert_eq!(sink.count(), 0);
    provider.fail_on = None;
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::UncertainEffect)
    );
    assert_eq!(provider.calls, 2);
}

#[test]
fn malformed_response_does_not_save_any_candidate() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    provider.responses[2] = "{not-json".into();
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert!(matches!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert_eq!(sink.count(), 0);
}

#[test]
fn cancel_after_step_blocks_next_billable_request_and_all_writes() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let cancelled = stop.0.clone();
    provider.on_call = Some(Box::new(move |_| cancelled.set(true)));
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::Cancelled)
    );
    assert_eq!(provider.calls, 1);
    assert_eq!(sink.count(), 0);
}

#[test]
fn changed_source_review_blocks_old_generated_content_before_next_dispatch() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let shared = current.0.clone();
    provider.on_call = Some(Box::new(move |_| {
        shared.borrow_mut().evidence[0].excerpt = "Evidence was reviewed and changed".into();
    }));
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::StaleContext)
    );
    assert_eq!(provider.calls, 1);
    assert_eq!(sink.count(), 0);
}

#[test]
fn rejects_foreign_or_unattributed_source_and_untrusted_corroboration() {
    let c = context();
    let mut foreign = c.clone();
    foreign.evidence[0].locator.source_id = Uuid::new_v4().to_string();
    assert!(matches!(
        prepare_three("bad", foreign),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    let mut missing = c.clone();
    missing.evidence.pop();
    assert!(matches!(
        prepare_three("bad", missing),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    let mut lying = c;
    lying.evidence[0].status = EvidenceStatus::Corroborated;
    assert!(matches!(
        prepare_three("bad", lying),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
}

#[test]
fn rejects_similar_angles_and_duplicate_scripts() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let mut angles: serde_json::Value = serde_json::from_str(&angles_json()).unwrap();
    angles["angles"][1]["hook"] = angles["angles"][0]["hook"].clone();
    provider.responses[0] = angles.to_string();
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    );
    assert_eq!(sink.count(), 0);
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let a: serde_json::Value = serde_json::from_str(&draft_json(1)).unwrap();
    let mut b: serde_json::Value = serde_json::from_str(&draft_json(2)).unwrap();
    b["beats"] = a["beats"].clone();
    provider.responses[2] = b.to_string();
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    );
    assert_eq!(sink.count(), 0);
}

#[test]
fn late_transaction_failure_rolls_back_all_three() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let path = temp.path().join("test.sqlite");
    let mut sink = SqlFixtureSink::open(&path, current.0.clone());
    sink.fail_at = Some(2);
    assert!(matches!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::PersistFailed(_))
    ));
    assert_eq!(sink.count(), 0);
    drop(sink);
    let reopened = SqlFixtureSink::open(&path, current.0.clone());
    assert_eq!(reopened.count(), 0);
}

#[test]
fn targeted_revision_rejects_stale_edits_and_preserves_other_beats() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    let ids = generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink).unwrap();
    let mut candidate = sink.get(ids[0]).candidates[0].clone();
    let original = candidate.draft.beats[0].narration.clone();
    candidate.draft.beats[0].manually_edited = true;
    let response =
        json!({"beat_id":"hook","replacement_narration":"A different original introduction"})
            .to_string();
    assert!(matches!(
        apply_reviewed_beat_revision(
            &candidate,
            &plan.context,
            &response,
            "hook",
            "Try another opening",
            &original,
            false
        ),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert!(matches!(
        apply_reviewed_beat_revision(
            &candidate,
            &plan.context,
            &response,
            "hook",
            "Try another opening",
            "stale text",
            true
        ),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    let changed = apply_reviewed_beat_revision(
        &candidate,
        &plan.context,
        &response,
        "hook",
        "Try another opening",
        &original,
        true,
    )
    .unwrap();
    assert_eq!(changed.draft.beats[1], candidate.draft.beats[1]);
    assert!(changed.draft.beats[0].manually_edited);
    assert!(changed.draft.beats[0].evidence_ids.is_empty());
    assert_eq!(
        changed.draft.claims[0].status,
        scriptotar_ai::writing::ClaimStatus::Unverified
    );
    assert!(changed.draft.claims[0].evidence_ids.is_empty());
    assert_eq!(changed.draft.beats[0].role, BeatRole::Hook);
}

#[test]
fn cross_project_loader_refuses_a_different_project_scope() {
    let c = context();
    let loader = MutableStudy(Rc::new(RefCell::new(c.clone())));
    assert!(matches!(
        loader.load(Uuid::new_v4(), c.study_id),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
}

#[test]
fn separately_approved_revision_proposal_does_not_change_saved_text() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    let ids = generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink).unwrap();
    let candidate = sink.get(ids[0]).candidates[0].clone();
    let original = candidate.draft.clone();
    provider.responses.push_back(
        json!({
            "beat_id":"hook","replacement_narration":"A revised, original introduction"
        })
        .to_string(),
    );
    let proposal = study_writing::propose_beat_revision(
        &plan,
        &current,
        &mut gate,
        &mut provider,
        &stop,
        &candidate,
        "hook",
        "Improve clarity",
    )
    .unwrap();
    assert!(proposal.contains("revised"));
    assert_eq!(provider.calls, 5);
    assert_eq!(sink.get(ids[0]).candidates[0].draft, original);
}

#[test]
fn optional_model_review_is_advisory_not_a_verification_verdict() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    let ids = generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink).unwrap();
    let candidate = sink.get(ids[0]).candidates[0].clone();
    provider.responses.push_back(
        json!({
            "findings":[{"beat_id":"hook","concern":"Check its underlying source",
                "recommendation":"Ask a human to verify the quoted detail",
                "evidence_ids":["e1"],"needs_human_review":false}]
        })
        .to_string(),
    );
    let reviewed = study_writing::review_candidate(
        &plan,
        &current,
        &mut gate,
        &mut provider,
        &stop,
        &candidate,
    )
    .unwrap();
    assert!(reviewed.review_findings[0].needs_human_review);
    assert_eq!(reviewed.draft, candidate.draft);
    assert_eq!(sink.get(ids[0]).candidates[0].review_findings.len(), 0);
}

#[test]
fn arabic_output_and_locator_remain_intact() {
    let (temp, _plan, current, mut gate, mut provider, stop) = setup();
    let mut c = current.0.borrow_mut();
    c.brief.language = "ar".into();
    c.brief.goal = "شرح أصلي للمجتمع".into();
    let plan = prepare_three("run-arabic", c.clone()).unwrap();
    drop(c);
    provider.responses =
        VecDeque::from([angles_json(), draft_json(1), draft_json(2), draft_json(3)]);
    for (i, raw) in provider.responses.iter_mut().enumerate().skip(1) {
        let mut draft: serde_json::Value = serde_json::from_str(raw).unwrap();
        draft["language"] = json!("ar");
        draft["beats"][0]["narration"] = json!(format!(
            "هذه فكرة عربية أصلية مختلفة رقم {i} ونص جديد للمشاهدين"
        ));
        *raw = draft.to_string();
    }
    let mut sink = SqlFixtureSink::open(&temp.path().join("arabic.sqlite"), current.0.clone());
    let ids = generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink).unwrap();
    assert_eq!(sink.count(), 3);
    assert!(export_candidate(&sink.get(ids[0]).candidates[0]).contains("هذه فكرة"));
}

#[test]
fn canonical_gateway_adapter_reuses_real_source_rows_and_rejects_cross_project() {
    use scriptotar_ai::writing::{WritingInput, WritingTask};
    use scriptotar_core::{StudySource, StudySourceKind};

    let scoped = context();
    let sources: Vec<_> = scoped
        .sources
        .iter()
        .map(|source| StudySource {
            id: source.study_source_id,
            project_id: scoped.project_id,
            study_id: scoped.study_id,
            kind: StudySourceKind::Transcript,
            reference_id: Some(source.origin_id),
            locator: "fixture local transcript".into(),
            manual_text: None,
            fingerprint: source.origin_id.to_string(),
            source_revision: None,
            collected_at: "2026-11-01T10:00:00Z".into(),
            uncertainty: None,
            available: true,
            created_at: "2026-11-01T10:00:00Z".into(),
        })
        .collect();
    let input = WritingInput {
        brief: scoped.brief.clone(),
        evidence: scoped.evidence.clone(),
        findings: scoped.findings.clone(),
        task: WritingTask::ProposeAngles { count: 3 },
    };
    let reconstructed = StudyWritingContext::from_scoped_study(
        scoped.project_id,
        scoped.study_id,
        scoped.study_revision,
        &sources,
        input.clone(),
    )
    .unwrap();
    assert_eq!(reconstructed, scoped);
    assert!(matches!(
        StudyWritingContext::from_scoped_study(
            Uuid::new_v4(),
            scoped.study_id,
            scoped.study_revision,
            &sources,
            input,
        ),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
}
