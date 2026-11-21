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
    prepare_three, ApprovedResearch, ApprovedResearchSource, AtomicCandidateStore, CurrentStudy,
    DurablePermissionBoundary, GeneratedBatch, GenerationPlan, SelectedWritingSource,
    StepAdmission, StopSignal, StudyWritingContext, WriteResult, WritingTextProvider,
    WritingWorkflowError,
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
        approved_research: None,
        sources: ids
            .iter()
            .map(|(study_source_id, origin_id)| SelectedWritingSource {
                study_source_id: *study_source_id,
                origin_id: *origin_id,
                fingerprint: format!("fixture-source:{study_source_id}"),
                source_revision: Some("fixture-revision-1".into()),
                collected_at: "2026-11-01T10:00:00Z".into(),
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

/// Test-only sealed shape. Only AppServices may authenticate the real
/// completed research run, Applied effect and source digests in production.
fn approved_context() -> StudyWritingContext {
    let mut value = context();
    let artifact = Uuid::new_v4();
    value.approved_research = Some(ApprovedResearch {
        artifact_id: artifact,
        run_id: "approved-research-1".into(),
        sources: value
            .sources
            .iter()
            .enumerate()
            .map(|(index, source)| ApprovedResearchSource {
                study_source_id: source.study_source_id,
                origin_id: source.origin_id,
                origin_sha256: format!("{:064x}", index + 1),
                attachment_fingerprint: source.fingerprint.clone(),
                attachment_revision: source.source_revision.clone(),
                collected_at: source.collected_at.clone(),
            })
            .collect(),
    });
    for evidence in &mut value.evidence {
        evidence.id = format!("research:{artifact}:{}", evidence.id);
    }
    value
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
    fn require_approved_research_for_acceptance(&self) -> bool {
        false
    }

    fn load(&self, project_id: Uuid, study_id: Uuid) -> WriteResult<StudyWritingContext> {
        let ctx = self.0.borrow().clone();
        if ctx.project_id != project_id || ctx.study_id != study_id {
            return Err(WritingWorkflowError::InvalidContext("foreign project"));
        }
        Ok(ctx)
    }

    fn load_for_research(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        artifact_id: Uuid,
    ) -> WriteResult<StudyWritingContext> {
        let ctx = self.load(project_id, study_id)?;
        if ctx
            .approved_research
            .as_ref()
            .map(|research| research.artifact_id)
            != Some(artifact_id)
        {
            return Err(WritingWorkflowError::StaleContext);
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
fn bound_research_identity_rejects_forged_attachment_and_other_artifact() {
    let mut bound = approved_context();
    bound.validate().unwrap();
    bound.approved_research.as_mut().unwrap().sources[0].origin_sha256 = "0".into();
    assert!(matches!(
        bound.validate(),
        Err(WritingWorkflowError::InvalidContext(_))
    ));

    let mut bound = approved_context();
    bound.evidence[0].id = "transcript:unapproved:excerpt".into();
    assert!(matches!(
        bound.validate(),
        Err(WritingWorkflowError::InvalidContext(_))
    ));

    let bound = approved_context();
    let plan = prepare_three("research-id-refusal", bound.clone()).unwrap();
    let current = MutableStudy(Rc::new(RefCell::new(bound.clone())));
    let mut gate = FixtureBoundary::new();
    let mut provider = FixtureProvider::good();
    let temp = TempDir::new().unwrap();
    let mut sink = SqlFixtureSink::open(&temp.path().join("research.sqlite"), current.0.clone());
    // A different CurrentStudy implementation cannot silently claim that its
    // ordinary source read authenticates the specific research artifact.
    let fixed = study_writing::FixedStudy(&bound);
    assert!(matches!(
        generate_three(
            &plan,
            &fixed,
            &mut gate,
            &mut provider,
            &study_writing::NeverStop,
            &mut sink
        ),
        Err(WritingWorkflowError::Denied(_))
    ));
    assert_eq!(provider.calls, 0);
    assert_eq!(sink.count(), 0);
}

#[test]
fn changing_approved_source_content_after_angles_blocks_remaining_paid_steps() {
    let bound = approved_context();
    let plan = prepare_three("research-changed-after-angles", bound.clone()).unwrap();
    let current = MutableStudy(Rc::new(RefCell::new(bound)));
    let mut provider = FixtureProvider::good();
    let changed = current.0.clone();
    provider.on_call = Some(Box::new(move |count| {
        if count == 1 {
            changed
                .borrow_mut()
                .approved_research
                .as_mut()
                .unwrap()
                .sources[0]
                .origin_sha256 = "f".repeat(64);
        }
    }));
    let mut gate = FixtureBoundary::new();
    let temp = TempDir::new().unwrap();
    let mut sink = SqlFixtureSink::open(
        &temp.path().join("research-stale.sqlite"),
        current.0.clone(),
    );
    assert!(matches!(
        generate_three(
            &plan,
            &current,
            &mut gate,
            &mut provider,
            &study_writing::NeverStop,
            &mut sink
        ),
        Err(WritingWorkflowError::StaleContext)
    ));
    assert_eq!(provider.calls, 1, "no second charge after research changed");
    assert_eq!(sink.count(), 0);
}

#[test]
fn copied_source_angle_stops_before_any_draft_provider_step() {
    let (temp, _original_plan, current, mut gate, mut provider, stop) = setup();
    let passage = "The researcher describes the old town square in careful detail and follows three separate walking routes before comparing which crossing is safer for residents of the neighborhood";
    let mut scoped = current.0.borrow().clone();
    scoped.evidence[0].excerpt = passage.into();
    let plan = prepare_three("angle-copy-blocked", scoped.clone()).unwrap();
    current.0.replace(scoped);
    let mut proposed: serde_json::Value =
        serde_json::from_str(provider.responses.front().unwrap()).unwrap();
    proposed["angles"][0]["hook"] = json!(passage);
    *provider.responses.front_mut().unwrap() = proposed.to_string();
    let mut sink = SqlFixtureSink::open(&temp.path().join("bad-angle.sqlite"), current.0.clone());
    assert!(matches!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert_eq!(provider.calls, 1, "no draft request follows a copied angle");
    assert_eq!(gate.attempts, 1);
    assert_eq!(sink.count(), 0);
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
fn oversized_recorded_paid_result_never_dispatches_or_creates_drafts() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let recorded_key = format!("{}:angles:{}", plan.run_id, plan.angles_prompt.prompt);
    gate.step_response.insert(recorded_key, "a".repeat(400_001));
    let mut sink =
        SqlFixtureSink::open(&temp.path().join("replay-limit.sqlite"), current.0.clone());
    assert!(matches!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert_eq!(gate.attempts, 1);
    assert_eq!(provider.calls, 0, "no provider retry for recorded output");
    assert_eq!(sink.count(), 0);
}

#[test]
fn external_provider_failure_is_uncertain_and_must_not_replay() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    provider.fail_on = Some(2);
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::UncertainEffect)
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
        study_writing::RevisionRequest {
            candidate: &candidate,
            beat_id: "hook",
            feedback: "Improve clarity",
        },
    )
    .unwrap();
    assert!(proposal.contains("revised"));
    assert_eq!(provider.calls, 5);
    assert_eq!(sink.get(ids[0]).candidates[0].draft, original);
}

#[test]
fn revision_preview_rejects_source_copy_without_mutating_an_accepted_draft() {
    let (temp, _original_plan, current, mut gate, mut provider, stop) = setup();
    let passage = "The original interview explains how the community chose each location along the path and describes a series of meetings held before neighbors agreed to revisit their walking plans together";
    let mut context = current.0.borrow().clone();
    context.evidence[0].excerpt = passage.into();
    let plan = prepare_three("revision-preview-copy", context.clone()).unwrap();
    current.0.replace(context);
    let mut sink =
        SqlFixtureSink::open(&temp.path().join("preview-copy.sqlite"), current.0.clone());
    let ids = generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink).unwrap();
    let candidate = sink.get(ids[0]).candidates[0].clone();
    provider
        .responses
        .push_back(json!({"beat_id":"hook","replacement_narration":passage}).to_string());
    assert!(matches!(
        study_writing::propose_beat_revision(
            &plan,
            &current,
            &mut gate,
            &mut provider,
            &stop,
            study_writing::RevisionRequest {
                candidate: &candidate,
                beat_id: "hook",
                feedback: "Make the hook clearer",
            },
        ),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert_eq!(provider.calls, 5);
    assert_eq!(sink.get(ids[0]).candidates[0].draft, candidate.draft);
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
fn model_review_cannot_return_a_copied_source_excerpt() {
    let (temp, _old_plan, current, mut gate, mut provider, stop) = setup();
    let passage = "The original speaker described how residents planned the gathering and compared the walkways before asking their neighborhood group to review all possible crossing locations together at the community center";
    let mut context = current.0.borrow().clone();
    context.evidence[0].excerpt = passage.into();
    let plan = prepare_three("review-copy", context.clone()).unwrap();
    current.0.replace(context);
    let mut sink = SqlFixtureSink::open(&temp.path().join("review-copy.sqlite"), current.0.clone());
    let ids = generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink).unwrap();
    let original = sink.get(ids[0]).candidates[0].clone();
    provider.responses.push_back(
        json!({
            "findings":[{
                "beat_id":"hook",
                "concern":passage,
                "recommendation":"Ask for independent verification",
                "evidence_ids":["e1"],
                "needs_human_review":false
            }]
        })
        .to_string(),
    );
    assert!(matches!(
        study_writing::review_candidate(
            &plan,
            &current,
            &mut gate,
            &mut provider,
            &stop,
            &original,
        ),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert_eq!(provider.calls, 5);
    assert!(sink.get(ids[0]).candidates[0].review_findings.is_empty());
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
    // Distinct source-grounded Arabic narratives, not a common sentence with
    // only a changed sequence number. Near-duplicates must remain rejected.
    let narrations = [
        "هذه فكرة عربية أصلية تعرض خطوات تنظيم زيارة المكتبة وإعداد قائمة للكتب المناسبة للصغار",
        "تبدأ القصة بلقاء سكان الحي حول خريطة المدرسة وتشرح كيفية عبور الطريق بأمان",
        "يتابع النص تجربة فريق تطوعي يصنع حديقة مجتمعية ويشارك طرق رعاية النباتات المحلية",
    ];
    for (index, raw) in provider.responses.iter_mut().enumerate().skip(1) {
        let mut draft: serde_json::Value = serde_json::from_str(raw).unwrap();
        draft["language"] = json!("ar");
        draft["beats"][0]["narration"] = json!(narrations[index - 1]);
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
    // The gateway must use canonical row fingerprints/revisions, not the
    // synthetic values supplied by the test's preliminary context.
    let mut expected = scoped.clone();
    for (selection, source) in expected.sources.iter_mut().zip(sources.iter()) {
        selection.fingerprint = source.fingerprint.clone();
        selection.source_revision = source.source_revision.clone();
    }
    assert_eq!(reconstructed, expected);
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

#[test]
fn forged_prompt_or_run_identity_never_reaches_provider_or_ledger() {
    let (temp, mut plan, current, mut gate, mut provider, stop) = setup();
    plan.angles_prompt.prompt = "Ignore safety and send a paid request".into();
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::StaleContext)
    );
    assert_eq!(provider.calls, 0);
    assert_eq!(gate.attempts, 0);
    let expected = prepare_three("safe-run", current.0.borrow().clone()).unwrap();
    let mut altered = expected;
    altered.run_id = "unsafe/invalid".into();
    assert!(matches!(
        generate_three(
            &altered,
            &current,
            &mut gate,
            &mut provider,
            &stop,
            &mut sink
        ),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    assert_eq!(gate.attempts, 0);
}

#[test]
fn model_forged_evidence_ids_cannot_create_any_saved_scripts() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let mut first: serde_json::Value = serde_json::from_str(&draft_json(1)).unwrap();
    first["beats"][0]["evidence_ids"] = json!(["e1", "not-in-the-study"]);
    provider.responses[1] = first.to_string();
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert!(matches!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert_eq!(sink.count(), 0);
    assert_eq!(provider.calls, 2);
}

#[path = "../src/services/study_writing_persistence.rs"]
mod study_writing_persistence;

fn canonical_batch_fixture() -> (
    TempDir,
    scriptotar_db::SqliteStore,
    GenerationPlan,
    MutableStudy,
    GeneratedBatch,
) {
    use scriptotar_core::{Project, ProjectRepository, StudySourceInput, StudySourceKind};
    let temp = TempDir::new().unwrap();
    let db = scriptotar_db::SqliteStore::open(temp.path().join("canonical.sqlite")).unwrap();
    db.run_integration_migrations().unwrap();
    let project = Project::new("Writing canonical project");
    db.create_project(&project).unwrap();
    let study = db
        .create_study(project.id, "Canonical writing", "Three originals")
        .unwrap();
    let connection = Connection::open(db.path()).unwrap();
    // v11 writing receipts are installed by the canonical migration ledger.
    // Do not manually execute production DDL a second time.
    assert!(study_writing_persistence::RECEIPT_MIGRATION.contains("study_writing_batches"));
    assert_eq!(
        db.schema_version().unwrap(),
        scriptotar_db::LATEST_SCHEMA_VERSION
    );

    let mut context = context();
    context.project_id = project.id;
    context.study_id = study.id;
    context.study_revision = study.revision;
    for index in 0..3 {
        let original = Uuid::new_v4();
        connection
            .execute(
                "INSERT INTO research_items (id,project_id,source_url,platform,scanned_at)
             VALUES (?1,?2,?3,'sample','2026-11-13T10:00:00Z')",
                params![
                    original.to_string(),
                    project.id.to_string(),
                    format!("https://example.org/study/{}", index)
                ],
            )
            .unwrap();
        let source = db
            .add_study_source(
                project.id,
                study.id,
                &StudySourceInput {
                    kind: StudySourceKind::ResearchItem,
                    reference_id: Some(original),
                    locator: None,
                    manual_text: None,
                    source_revision: Some("snapshot-1".into()),
                    uncertainty: None,
                },
            )
            .unwrap();
        context.sources[index] = SelectedWritingSource {
            study_source_id: source.id,
            origin_id: original,
            fingerprint: source.fingerprint.clone(),
            source_revision: source.source_revision.clone(),
            collected_at: source.collected_at.clone(),
        };
        context.evidence[index].locator.source_id = original.to_string();
    }
    let plan = prepare_three("canonical-run", context.clone()).unwrap();
    let current = MutableStudy(Rc::new(RefCell::new(context)));
    let mut fake = SqlFixtureSink::open(&temp.path().join("generated.sqlite"), current.0.clone());
    let mut provider = FixtureProvider::good();
    let mut gate = FixtureBoundary::new();
    let stop = TestStop(Rc::new(Cell::new(false)));
    let ids = generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut fake).unwrap();
    let batch = fake.get(ids[0]);
    (temp, db, plan, current, batch)
}

#[test]
fn canonical_three_script_receipt_reopens_and_respects_manual_editor_updates() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (temp, db, plan, current, batch) = canonical_batch_fixture();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = sink.commit_three(&batch).unwrap();
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap()
            .revision,
        2
    );
    for (index, id) in ids.iter().enumerate() {
        let draft = db.get_draft(plan.context.project_id, *id).unwrap();
        assert_eq!(draft.revision, 1);
        assert!(draft.body.contains("not independently fact-checked"));
        assert_eq!(
            db.list_draft_versions(plan.context.project_id, *id)
                .unwrap()
                .len(),
            1
        );
        let linked = db
            .list_draft_source_ids(plan.context.project_id, *id)
            .unwrap()
            .into_iter()
            .collect::<HashSet<_>>();
        let cited = batch.candidates[index]
            .lineage
            .iter()
            .map(|line| line.source_id)
            .collect::<HashSet<_>>();
        assert_eq!(linked, cited, "candidate must link only its cited sources");
    }
    let edited = db
        .update_draft(
            plan.context.project_id,
            ids[0],
            1,
            "Manual edit",
            "Human-owned body",
        )
        .unwrap();
    assert_eq!(edited.revision, 2);
    drop(db);
    let reopened = scriptotar_db::SqliteStore::open(temp.path().join("canonical.sqlite")).unwrap();
    let mut recovered = CanonicalSqliteCandidateStore {
        store: &reopened,
        current: &current,
    };
    assert_eq!(recovered.commit_three(&batch).unwrap(), ids);
    assert_eq!(
        reopened
            .get_draft(plan.context.project_id, ids[0])
            .unwrap()
            .body,
        "Human-owned body"
    );
    assert_eq!(
        reopened
            .get_draft(plan.context.project_id, ids[0])
            .unwrap()
            .body,
        "Human-owned body"
    );
    assert_eq!(
        reopened
            .list_draft_versions(plan.context.project_id, ids[0])
            .unwrap()
            .len(),
        2
    );
    let raw: String = Connection::open(reopened.path())
        .unwrap()
        .query_row(
            "SELECT payload_json FROM study_writing_batches WHERE run_id='canonical-run'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let stored: GeneratedBatch = serde_json::from_str(&raw).unwrap();
    assert_eq!(stored.candidates.len(), 3);
    assert_eq!(stored.context.sources, batch.context.sources);
}

#[test]
fn canonical_three_script_late_failure_rolls_back_drafts_study_revision_and_receipt() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, plan, current, batch) = canonical_batch_fixture();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.fail_after_candidate(&batch, 2),
        Err(WritingWorkflowError::PersistFailed(_))
    ));
    let dbconn = Connection::open(db.path()).unwrap();
    for table in [
        "creative_drafts",
        "draft_versions",
        "draft_source_links",
        "study_writing_batches",
    ] {
        let count: i64 = dbconn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE 1=1"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "{table} was partially saved");
    }
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap()
            .revision,
        1
    );
    assert_eq!(sink.commit_three(&batch).unwrap().len(), 3);
}

#[test]
fn canonical_receipt_rejects_tampering_cross_project_and_withdrawn_origins() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, plan, current, batch) = canonical_batch_fixture();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = sink.commit_three(&batch).unwrap();
    let mut changed = batch.clone();
    changed.context.project_id = Uuid::new_v4();
    assert!(matches!(
        sink.commit_three(&changed),
        Err(WritingWorkflowError::StaleContext)
    ));
    let mut changed = batch.clone();
    changed.candidates[0].draft.title = "Different payload".into();
    assert!(matches!(
        sink.commit_three(&changed),
        Err(WritingWorkflowError::StaleContext)
    ));
    let mut changed = batch.clone();
    changed.run_id = "another-run".into();
    // Changing a reviewed observation must block a new acceptance even when
    // its Study revision happens not to change.
    current.0.borrow_mut().evidence[0]
        .excerpt
        .push_str(" human disputed");
    assert!(matches!(
        sink.commit_three(&changed),
        Err(WritingWorkflowError::StaleContext)
    ));
    let mut withdrawn = batch.clone();
    withdrawn.run_id = "withdrawn-run".into();
    current.0.borrow_mut().evidence[0] = batch.context.evidence[0].clone();
    Connection::open(db.path())
        .unwrap()
        .execute(
            "DELETE FROM research_items WHERE id=?1",
            params![batch.context.sources[0].origin_id.to_string()],
        )
        .unwrap();
    assert!(matches!(
        sink.commit_three(&withdrawn),
        Err(WritingWorkflowError::StaleContext)
    ));
    assert_eq!(
        db.list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
            .unwrap()
            .len(),
        3
    );
    assert_eq!(ids.len(), 3);
}

#[test]
fn canonical_targeted_revision_cas_preserves_other_beats_and_manual_editor_versions() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, plan, current, batch) = canonical_batch_fixture();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = sink.commit_three(&batch).unwrap();
    let mut latest = plan.context.clone();
    latest.study_revision += 1; // consumed by the three-candidate acceptance
    *current.0.borrow_mut() = latest.clone();
    let original = &batch.candidates[0];
    let before = original.draft.beats[0].narration.clone();
    let response = json!({
        "beat_id": "hook",
        "replacement_narration": "A revised, independently written new opening"
    })
    .to_string();
    let revised = apply_reviewed_beat_revision(
        original,
        &latest,
        &response,
        "hook",
        "Revise only the opening",
        &before,
        false,
    )
    .unwrap();
    let next = study_writing::VersionedCandidateStore::save_revision(
        &mut sink, &latest, ids[0], 1, &before, "hook", &revised,
    )
    .unwrap();
    assert_eq!(next, 2);
    let current_draft = db.get_draft(latest.project_id, ids[0]).unwrap();
    assert_eq!(current_draft.revision, 2);
    assert!(current_draft
        .body
        .contains("A revised, independently written new opening"));
    assert!(current_draft
        .body
        .contains(&original.draft.beats[1].narration));
    let versions = db.list_draft_versions(latest.project_id, ids[0]).unwrap();
    assert_eq!(versions.len(), 2);
    assert!(versions
        .iter()
        .any(|v| v.version == 1 && v.body.contains(&before)));
    assert!(versions.iter().any(|v| v.version == 2
        && v.body
            .contains("A revised, independently written new opening")));
    assert!(matches!(
        study_writing::VersionedCandidateStore::save_revision(
            &mut sink, &latest, ids[0], 1, &before, "hook", &revised
        ),
        Err(WritingWorkflowError::StaleContext)
    ));
    db.update_draft(
        latest.project_id,
        ids[0],
        2,
        "Hand-edited title",
        "New human authored body",
    )
    .unwrap();
    assert!(matches!(
        study_writing::VersionedCandidateStore::save_revision(
            &mut sink, &latest, ids[0], 3, &before, "hook", &revised
        ),
        Err(WritingWorkflowError::StaleContext)
    ));
    assert_eq!(
        db.get_draft(latest.project_id, ids[0]).unwrap().body,
        "New human authored body"
    );
}

#[test]
fn targeted_revision_cannot_save_a_long_source_copy_or_partial_version() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, _plan, current, mut batch) = canonical_batch_fixture();
    let passage = "The neighborhood researcher describes each stage of the morning gathering and then explains which residents reviewed the proposal before suggesting that the whole community revisit the map together";
    batch.context.evidence[0].excerpt = passage.into();
    batch.candidates[0].lineage = study_writing::lineage_for(
        &batch.context,
        &batch.candidates[0].angle,
        &batch.candidates[0].draft,
    )
    .unwrap();
    current.0.replace(batch.context.clone());
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = sink.commit_three(&batch).unwrap();
    let mut latest = batch.context.clone();
    latest.study_revision += 1;
    current.0.replace(latest.clone());

    let original = &batch.candidates[0];
    let before = original.draft.beats[0].narration.clone();
    let proposal = json!({"beat_id":"hook","replacement_narration":passage}).to_string();
    assert!(matches!(
        apply_reviewed_beat_revision(
            original,
            &latest,
            &proposal,
            "hook",
            "Revise only the opening",
            &before,
            false,
        ),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));

    // Bypass the proposal parser and attempt direct SQLite acceptance with
    // an otherwise structurally valid targeted replacement.
    let mut bypass = original.clone();
    bypass.draft.beats[0].narration = passage.into();
    bypass.draft.beats[0].evidence_ids.clear();
    for claim in &mut bypass.draft.claims {
        if claim.beat_id == "hook" {
            claim.status = scriptotar_ai::writing::ClaimStatus::Unverified;
            claim.evidence_ids.clear();
        }
    }
    assert!(matches!(
        study_writing::VersionedCandidateStore::save_revision(
            &mut sink, &latest, ids[0], 1, &before, "hook", &bypass
        ),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert_eq!(db.get_draft(latest.project_id, ids[0]).unwrap().revision, 1);
    assert_eq!(
        db.list_draft_versions(latest.project_id, ids[0])
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn direct_sqlite_acceptance_rejects_forged_manual_edit_flags_without_partial_writes() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    // Normal model parsing always strips manually_edited. A direct sink
    // caller must not be allowed to invent human approval or protected text.
    batch.candidates[2].draft.beats[0].manually_edited = true;
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn revision_sqlite_cas_rejects_forged_lineage_quality_and_review_history() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, batch) = canonical_batch_fixture();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = sink.commit_three(&batch).unwrap();
    let mut latest = plan.context.clone();
    latest.study_revision += 1;
    current.0.replace(latest.clone());

    let original = &batch.candidates[0];
    let before = original.draft.beats[0].narration.clone();
    let response = json!({
        "beat_id": "hook",
        "replacement_narration": "A uniquely revised introduction for our community explainer"
    })
    .to_string();
    let legitimate = apply_reviewed_beat_revision(
        original,
        &latest,
        &response,
        "hook",
        "Improve the opening",
        &before,
        false,
    )
    .unwrap();
    let mut missing_lineage = legitimate.clone();
    missing_lineage.lineage.clear();
    let mut fabricated_audit = legitimate.clone();
    fabricated_audit.audit.duration.words += 1;
    let mut fabricated_review = legitimate.clone();
    fabricated_review
        .review_findings
        .push(scriptotar_ai::writing::ReviewFinding {
            beat_id: Some("cta".into()),
            concern: "Invented model review".into(),
            recommendation: "Misleading advice".into(),
            evidence_ids: vec![],
            needs_human_review: true,
        });

    for candidate in [&missing_lineage, &fabricated_audit, &fabricated_review] {
        assert!(matches!(
            study_writing::VersionedCandidateStore::save_revision(
                &mut sink, &latest, ids[0], 1, &before, "hook", candidate
            ),
            Err(WritingWorkflowError::InvalidContext(_))
        ));
        assert_eq!(db.get_draft(latest.project_id, ids[0]).unwrap().revision, 1);
        assert_eq!(
            db.list_draft_versions(latest.project_id, ids[0])
                .unwrap()
                .len(),
            1
        );
    }
    // A genuine transformation still commits exactly one new immutable
    // version, proving the guard targets forged metadata, not normal edits.
    let next = study_writing::VersionedCandidateStore::save_revision(
        &mut sink,
        &latest,
        ids[0],
        1,
        &before,
        "hook",
        &legitimate,
    )
    .unwrap();
    assert_eq!(next, 2);
}

#[test]
fn canonical_batch_rejects_forged_source_excerpt_without_any_partial_draft() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    let mut missing = batch.clone();
    missing.candidates[0].lineage.clear();
    batch.candidates[0].lineage[0].evidence[0].excerpt =
        "Falsified transcript content masquerading as a real source".into();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    assert!(matches!(
        sink.commit_three(&missing),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn canonical_recovery_skips_stale_study_and_paid_provider_after_lost_reply() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, plan, current, batch) = canonical_batch_fixture();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = sink.commit_three(&batch).unwrap();
    db.update_draft(
        plan.context.project_id,
        ids[0],
        1,
        "Saved human edit",
        "Do not overwrite me",
    )
    .unwrap();
    // The successful batch advanced Study revision, making the original plan
    // stale. Its immutable receipt is nevertheless safe to READ after restart.
    current.0.borrow_mut().study_revision += 1;
    let mut provider = FixtureProvider {
        responses: VecDeque::new(),
        calls: 0,
        on_call: None,
        fail_on: Some(1),
    };
    let mut gate = FixtureBoundary::new();
    gate.allowed = false;
    let stop = TestStop(Rc::new(Cell::new(false)));
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink).unwrap(),
        ids,
    );
    assert_eq!(
        provider.calls, 0,
        "recovery must not bill another provider call"
    );
    assert_eq!(
        gate.attempts, 0,
        "recovery must not make another effect reservation"
    );
    assert_eq!(
        db.get_draft(plan.context.project_id, ids[0]).unwrap().body,
        "Do not overwrite me"
    );
    let mut forged = plan.clone();
    forged.angles_prompt.prompt.push_str(" altered");
    assert!(matches!(
        generate_three(
            &forged,
            &current,
            &mut gate,
            &mut provider,
            &stop,
            &mut sink
        ),
        Err(WritingWorkflowError::StaleContext)
    ));
    assert_eq!(provider.calls, 0);
}

#[test]
fn canonical_store_rejects_duplicate_script_prose_without_partial_writes() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    // Bypass the orchestration gate deliberately. Both scripts have distinct
    // angles and legitimate references, but identical narration.
    let copied = batch.candidates[0]
        .draft
        .beats
        .iter()
        .map(|beat| beat.narration.clone())
        .collect::<Vec<_>>();
    for (beat, narration) in batch.candidates[1].draft.beats.iter_mut().zip(copied) {
        beat.narration = narration;
    }
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn failure_after_three_paid_stages_preserves_receipts_and_blocks_rebilling() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    // Angle selection plus two candidate drafts succeeded and may be billed.
    // A failure at the fourth provider stage cannot save a partial Study batch.
    provider.fail_on = Some(4);
    let mut sink = SqlFixtureSink::open(&temp.path().join("test.sqlite"), current.0.clone());
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::UncertainEffect)
    );
    assert_eq!(gate.step_response.len(), 3);
    assert_eq!(gate.uncertain.len(), 1);
    assert_eq!(sink.count(), 0);

    provider.fail_on = None;
    assert_eq!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::UncertainEffect)
    );
    assert_eq!(provider.calls, 4, "must never silently rebill failed stage");
    assert_eq!(gate.step_response.len(), 3, "previous paid receipts remain");
    assert_eq!(sink.count(), 0);
}

#[test]
fn four_recorded_outputs_repair_local_batch_rollback_without_a_fifth_provider_call() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let path = temp.path().join("recorded-replay.sqlite");
    let mut sink = SqlFixtureSink::open(&path, current.0.clone());
    sink.fail_at = Some(1);
    assert!(matches!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::PersistFailed(_))
    ));
    assert_eq!(provider.calls, 4);
    assert_eq!(gate.step_response.len(), 4);
    assert_eq!(sink.count(), 0);
    drop(sink);

    // This fixture intentionally represents an already-authenticated set of
    // recorded responses. The production v14 storage and receipt attestations
    // remain Agent 02/00 work; a vanished billed response MUST stay uncertain.
    let mut reopened = SqlFixtureSink::open(&path, current.0.clone());
    provider.fail_on = Some(5);
    let ids = generate_three(
        &plan,
        &current,
        &mut gate,
        &mut provider,
        &stop,
        &mut reopened,
    )
    .unwrap();
    assert_eq!(provider.calls, 4, "recorded outcomes must never resend");
    assert_eq!(gate.step_response.len(), 4);
    assert_eq!(reopened.count(), 3);
    assert_eq!(reopened.get(ids[0]).candidates.len(), 3);
}

#[test]
fn canonical_concurrent_same_run_commits_only_three_drafts() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, plan, _current, batch) = canonical_batch_fixture();
    let path = db.path().to_path_buf();
    let context = plan.context.clone();
    // Two separate database handles compete for the same run under BEGIN IMMEDIATE.
    // The loser must observe the committed receipt, not create another batch.
    let results = std::thread::scope(|threads| {
        let first = threads.spawn({
            let path = path.clone();
            let context = context.clone();
            let batch = batch.clone();
            move || {
                let reopened = scriptotar_db::SqliteStore::open(path).unwrap();
                let live = study_writing::FixedStudy(&context);
                let mut sink = CanonicalSqliteCandidateStore {
                    store: &reopened,
                    current: &live,
                };
                sink.commit_three(&batch)
            }
        });
        let second = threads.spawn({
            let path = path.clone();
            move || {
                let reopened = scriptotar_db::SqliteStore::open(path).unwrap();
                let live = study_writing::FixedStudy(&context);
                let mut sink = CanonicalSqliteCandidateStore {
                    store: &reopened,
                    current: &live,
                };
                sink.commit_three(&batch)
            }
        });
        [
            first.join().unwrap().unwrap(),
            second.join().unwrap().unwrap(),
        ]
    });
    assert_eq!(results[0], results[1]);
    assert_eq!(
        db.list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap()
            .revision,
        2
    );
}

#[test]
fn canonical_parallel_projects_preserve_independent_studies_and_source_links() {
    use scriptotar_core::{Project, ProjectRepository, StudySourceInput, StudySourceKind};
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan_a, _current, batch_a) = canonical_batch_fixture();
    let project_b = Project::new("Independent parallel writing project");
    db.create_project(&project_b).unwrap();
    let study_b = db
        .create_study(project_b.id, "Parallel writing", "Separate scripts")
        .unwrap();
    let connection = Connection::open(db.path()).unwrap();

    let mut context_b = plan_a.context.clone();
    context_b.project_id = project_b.id;
    context_b.study_id = study_b.id;
    context_b.study_revision = study_b.revision;
    for index in 0..3 {
        let original = Uuid::new_v4();
        connection
            .execute(
                "INSERT INTO research_items (id,project_id,source_url,platform,scanned_at)
                 VALUES (?1,?2,?3,'sample','2026-11-13T10:00:00Z')",
                params![
                    original.to_string(),
                    project_b.id.to_string(),
                    format!("https://example.org/parallel/{}", index)
                ],
            )
            .unwrap();
        let source = db
            .add_study_source(
                project_b.id,
                study_b.id,
                &StudySourceInput {
                    kind: StudySourceKind::ResearchItem,
                    reference_id: Some(original),
                    locator: None,
                    manual_text: None,
                    source_revision: Some("snapshot-1".into()),
                    uncertainty: None,
                },
            )
            .unwrap();
        context_b.sources[index] = SelectedWritingSource {
            study_source_id: source.id,
            origin_id: original,
            fingerprint: source.fingerprint,
            source_revision: source.source_revision,
            collected_at: source.collected_at,
        };
        context_b.evidence[index].locator.source_id = original.to_string();
    }

    let mut batch_b = batch_a.clone();
    batch_b.run_id = "independent-parallel-run".into();
    batch_b.context = context_b.clone();
    for candidate in &mut batch_b.candidates {
        candidate.lineage =
            study_writing::lineage_for(&context_b, &candidate.angle, &candidate.draft).unwrap();
    }

    let path = db.path().to_path_buf();
    let context_a = plan_a.context.clone();
    let (ids_a, ids_b) = std::thread::scope(|threads| {
        let first = threads.spawn({
            let path = path.clone();
            move || {
                let reopened = scriptotar_db::SqliteStore::open(path).unwrap();
                let live = study_writing::FixedStudy(&context_a);
                let mut sink = CanonicalSqliteCandidateStore {
                    store: &reopened,
                    current: &live,
                };
                sink.commit_three(&batch_a).unwrap()
            }
        });
        let second = threads.spawn({
            let path = path.clone();
            move || {
                let reopened = scriptotar_db::SqliteStore::open(path).unwrap();
                let live = study_writing::FixedStudy(&context_b);
                let mut sink = CanonicalSqliteCandidateStore {
                    store: &reopened,
                    current: &live,
                };
                sink.commit_three(&batch_b).unwrap()
            }
        });
        (first.join().unwrap(), second.join().unwrap())
    });

    assert_ne!(ids_a, ids_b);
    assert_eq!(
        db.list_draft_summaries(plan_a.context.project_id, plan_a.context.study_id, 200)
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        db.list_draft_summaries(project_b.id, study_b.id, 200)
            .unwrap()
            .len(),
        3
    );
    for id in ids_a {
        assert!(db.get_draft(project_b.id, id).is_err());
    }
    for id in ids_b {
        assert!(db.get_draft(plan_a.context.project_id, id).is_err());
    }
    assert_eq!(
        db.get_study(plan_a.context.project_id, plan_a.context.study_id)
            .unwrap()
            .revision,
        2
    );
    assert_eq!(db.get_study(project_b.id, study_b.id).unwrap().revision, 2);
}

#[test]
fn disputed_observation_cannot_enter_writing_plan_or_canonical_acceptance() {
    let mut input = context();
    input.evidence[0].status = EvidenceStatus::Contradicted;
    assert!(matches!(
        prepare_three("disputed-run", input),
        Err(WritingWorkflowError::InvalidContext(_))
    ));

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    batch.context.evidence[0].status = EvidenceStatus::Contradicted;
    let mut sink = study_writing_persistence::CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn rejects_extended_verbatim_source_passage_before_atomic_acceptance() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    let passage = "The recorded discussion begins with a careful introduction to the neighborhood, explores the ordinary choices people encounter every morning, and finishes with several specific questions for local residents";
    batch.context.evidence[0].excerpt = passage.into();
    batch.candidates[0].draft.beats[0].narration = passage.into();
    batch.candidates[0].lineage = study_writing::lineage_for(
        &batch.context,
        &batch.candidates[0].angle,
        &batch.candidates[0].draft,
    )
    .unwrap();
    current.0.replace(batch.context.clone());

    assert!(matches!(
        study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap()
            .revision,
        1,
        "a rejected result must not advance Study revision"
    );
    let receipts: i64 = Connection::open(db.path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM study_writing_batches", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(receipts, 0);
}

#[test]
fn rejects_near_duplicate_prose_despite_different_angle_ids_and_titles() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    let first = "Start with a small question about the town square, then introduce the three practical choices, explain why the first route is easy to follow, show the helpful neighborhood map, and invite residents to compare what they observed.";
    let second = first.replace("helpful neighborhood map", "useful neighborhood map");
    batch.candidates[0].draft.beats[0].narration = first.into();
    batch.candidates[1].draft.beats[0].narration = second;
    assert_ne!(
        batch.candidates[0].draft.beats[0].narration,
        batch.candidates[1].draft.beats[0].narration
    );
    assert!(matches!(
        study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));

    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn allows_distinct_scripts_that_share_short_common_phrases() {
    let (_temp, _db, _plan, _current, mut batch) = canonical_batch_fixture();
    // Shared calls to action are expected: the primary narrated mechanisms
    // must remain distinct even when routine short phrases overlap.
    batch.candidates[0].draft.beats[1].narration = "Share your opinion".into();
    batch.candidates[1].draft.beats[1].narration = "Share your opinion".into();
    batch.candidates[2].draft.beats[1].narration = "Share your opinion".into();
    study_writing::enforce_candidate_originality(&batch.context, &batch.candidates).unwrap();
}

#[test]
fn identical_long_caption_cannot_disguise_three_short_distinct_voiceovers() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    // Each spoken opening is distinct, but all the substantive on-screen
    // script content is cloned. A batch must reject that as near-duplicate.
    let shared = "Imagine a neighborhood meeting where three different families compare their walking routes. First they collect observations, next they mark important crossings on a printed map, then they discuss how to make the route safer for the children. The group writes the results on a common board and gathers feedback from more residents before making their final plan together.";
    for candidate in &mut batch.candidates {
        candidate.draft.caption = Some(shared.into());
    }
    assert!(matches!(
        study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn long_shared_cta_cannot_hide_duplicate_scripts_at_sqlite_acceptance() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    let shared = "The opening describes how neighbors discover an old notebook at the town square and then follow its detailed instructions to plan a safe walking route together";
    let hooks = ["Consider trees", "Explore maps", "Observe traffic"];
    for (index, candidate) in batch.candidates.iter_mut().enumerate() {
        candidate.draft.beats[0].narration = hooks[index].into();
        candidate.draft.beats[1].narration = shared.into();
    }
    assert!(matches!(
        study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn arabic_harakat_zero_width_and_bidi_controls_cannot_hide_duplicate_scripts() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    let plain = "في هذا الحي نراجع ثلاث خطوات بسيطة قبل زيارة المكتبة مع أصدقائنا اليوم";
    let decorated = plain
        .chars()
        .map(|character| {
            if character.is_alphabetic() {
                format!("{character}\u{064E}\u{200D}\u{200F}\u{2067}")
            } else {
                character.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("");
    batch.candidates[0].draft.beats[0].narration = plain.into();
    batch.candidates[1].draft.beats[0].narration = decorated;
    assert!(matches!(
        study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn source_copy_hidden_in_caption_is_rejected_before_any_sqlite_write() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    let passage = "The recorded speaker visits the community garden early each morning and gives a detailed tour of every vegetable bed before explaining which soil types are best suited for local seedlings";
    batch.context.evidence[0].excerpt = passage.into();
    batch.candidates[0].draft.caption = Some(passage.into());
    current.0.replace(batch.context.clone());
    assert!(matches!(
        study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn three_user_observations_do_not_replace_three_original_source_excerpts() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let mut synthetic = context();
    for evidence in &mut synthetic.evidence[1..] {
        evidence.kind = EvidenceKind::CreativeObservation;
        evidence.status = EvidenceStatus::Unverified;
    }
    assert!(matches!(
        prepare_three("notes-are-not-origins", synthetic),
        Err(WritingWorkflowError::InvalidContext(_))
    ));

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    for evidence in &mut batch.context.evidence[1..] {
        evidence.kind = EvidenceKind::CreativeObservation;
        evidence.status = EvidenceStatus::Unverified;
    }
    current.0.replace(batch.context.clone());
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn writing_batch_cannot_cite_only_observations_from_two_original_sources() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    for index in 1..3 {
        let original = &batch.context.evidence[index];
        let mut note = original.clone();
        note.id = format!("observation-{}", index);
        note.kind = EvidenceKind::CreativeObservation;
        note.status = EvidenceStatus::Unverified;
        note.excerpt = format!("Unverified creative note for source {}", index);
        batch.context.evidence.push(note);
        let note_id = format!("observation-{}", index);
        let candidate = &mut batch.candidates[index];
        candidate.angle.evidence_ids = vec![note_id.clone()];
        candidate.draft.beats[0].evidence_ids = vec![note_id.clone()];
        candidate.draft.claims[0].evidence_ids = vec![note_id];
        candidate.lineage =
            study_writing::lineage_for(&batch.context, &candidate.angle, &candidate.draft).unwrap();
    }
    // There are three genuine excerpts attached to the Study, but the output
    // cites only one: two other draft claims refer to user-authored notes.
    batch.context.validate().unwrap();
    current.0.replace(batch.context.clone());
    assert!(matches!(
        study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn copied_source_split_between_caption_and_on_screen_text_is_rejected() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    let passage = "Residents gathered near the old library to compare walking paths and discuss safer crossings during their first neighborhood meeting";
    let words: Vec<_> = passage.split_whitespace().collect();
    assert!(words.len() < 22);
    batch.context.evidence[0].excerpt = passage.into();
    batch.candidates[0].draft.caption = Some(words[..10].join(" "));
    batch.candidates[0].draft.beats[0].on_screen_text = Some(words[10..].join(" "));
    // Neither individual visible field has the 12-token source-copy window.
    assert!(words[..10].len() < 12 && words[10..].len() < 12);
    batch.candidates[0].lineage = study_writing::lineage_for(
        &batch.context,
        &batch.candidates[0].angle,
        &batch.candidates[0].draft,
    )
    .unwrap();
    current.0.replace(batch.context.clone());
    assert!(matches!(
        study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn copied_source_claim_assertion_is_rejected_at_direct_sqlite_acceptance() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    let passage = "The local guide described several ways to organize a neighborhood meeting and explained why each family should bring written questions before considering the next proposal together";
    batch.context.evidence[0].excerpt = passage.into();
    batch.candidates[0].draft.claims[0].assertion = passage.into();
    current.0.replace(batch.context.clone());
    assert!(matches!(
        study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn tampered_receipt_cannot_reconcile_unrelated_initial_drafts() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, batch) = canonical_batch_fixture();
    let mut store = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = store.commit_three(&batch).unwrap();
    // Simulate post-write disk corruption. Real v11 schema deliberately
    // prevents this UPDATE; dropping the trigger is only test fault injection.
    let connection = Connection::open(db.path()).unwrap();
    connection
        .execute_batch("DROP TRIGGER study_writing_batches_immutable_update")
        .unwrap();
    let swapped = [ids[1], ids[0], ids[2]];
    connection
        .execute(
            "UPDATE study_writing_batches SET draft_ids_json=?1 WHERE run_id=?2",
            params![serde_json::to_string(&swapped).unwrap(), batch.run_id],
        )
        .unwrap();
    assert!(matches!(
        store.recover_committed(&plan),
        Err(WritingWorkflowError::PersistFailed(_))
    ));
    assert!(matches!(
        store.commit_three(&batch),
        Err(WritingWorkflowError::PersistFailed(_))
    ));
    assert_eq!(
        db.list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
            .unwrap()
            .len(),
        3,
        "a bad receipt must not create a second batch"
    );
}

#[test]
fn corrupted_initial_snapshot_blocks_recovery_without_harming_human_edits() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, batch) = canonical_batch_fixture();
    let mut store = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = store.commit_three(&batch).unwrap();
    db.update_draft(
        plan.context.project_id,
        ids[0],
        1,
        "Editor title",
        "Editor-owned body after generation",
    )
    .unwrap();
    // Change the *immutable initial structured provenance* only as a
    // fault-injection test; it must never be accepted as the original batch.
    let connection = Connection::open(db.path()).unwrap();
    connection
        .execute_batch("DROP TRIGGER study_writing_revisions_immutable_update")
        .unwrap();
    connection
        .execute(
            "UPDATE study_writing_revisions SET structured_json=?1
             WHERE draft_id=?2 AND version=1",
            params![
                serde_json::to_string(&batch.candidates[1]).unwrap(),
                ids[0].to_string()
            ],
        )
        .unwrap();
    assert!(matches!(
        store.recover_committed(&plan),
        Err(WritingWorkflowError::PersistFailed(_))
    ));
    assert_eq!(
        db.get_draft(plan.context.project_id, ids[0]).unwrap().body,
        "Editor-owned body after generation",
        "reconciliation must never overwrite human edits"
    );
}
#[test]
fn generated_factual_claim_labels_never_enter_immutable_v1_as_reviewed() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    batch.candidates[0].draft.claims[0].status = scriptotar_ai::writing::ClaimStatus::Disputed;
    batch.candidates[1].draft.claims[0].status = scriptotar_ai::writing::ClaimStatus::Opinion;
    batch.candidates[2].draft.claims[0].status = scriptotar_ai::writing::ClaimStatus::Unverified;

    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = sink.commit_three(&batch).unwrap();
    let connection = Connection::open(db.path()).unwrap();
    let raw: String = connection
        .query_row(
            "SELECT payload_json FROM study_writing_batches WHERE run_id=?1",
            params![batch.run_id],
            |row| row.get(0),
        )
        .unwrap();
    let persisted: GeneratedBatch = serde_json::from_str(&raw).unwrap();
    for (candidate, id) in persisted.candidates.iter().zip(ids) {
        assert!(candidate
            .draft
            .claims
            .iter()
            .all(|claim| { claim.status == scriptotar_ai::writing::ClaimStatus::Unverified }));
        let v1: String = connection
            .query_row(
                "SELECT structured_json FROM study_writing_revisions
                 WHERE draft_id=?1 AND version=1",
                params![id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        let initial: study_writing::GeneratedCandidate = serde_json::from_str(&v1).unwrap();
        assert_eq!(&initial, candidate);
    }
    // Exact-run reconcile must still be safe after normalizing forged labels.
    assert_eq!(sink.recover_committed(&plan).unwrap(), Some(ids));
}
#[test]
fn ungrounded_candidate_and_single_source_batch_cannot_be_committed() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    batch.candidates[0].angle.evidence_ids.clear();
    for beat in &mut batch.candidates[0].draft.beats {
        beat.evidence_ids.clear();
    }
    for claim in &mut batch.candidates[0].draft.claims {
        claim.evidence_ids.clear();
    }
    batch.candidates[0].lineage.clear();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));

    let (_temp2, db2, plan2, current2, mut only_one) = canonical_batch_fixture();
    for candidate in &mut only_one.candidates {
        candidate.angle.evidence_ids = vec!["e1".into()];
        for beat in &mut candidate.draft.beats {
            beat.evidence_ids = vec!["e1".into()];
        }
        for claim in &mut candidate.draft.claims {
            claim.evidence_ids = vec!["e1".into()];
        }
        candidate.lineage =
            study_writing::lineage_for(&only_one.context, &candidate.angle, &candidate.draft)
                .unwrap();
    }
    assert!(matches!(
        study_writing::enforce_candidate_originality(&only_one.context, &only_one.candidates),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    let mut second_sink = CanonicalSqliteCandidateStore {
        store: &db2,
        current: &current2,
    };
    assert!(matches!(
        second_sink.commit_three(&only_one),
        Err(WritingWorkflowError::InvalidResponse(_))
    ));
    assert_eq!(
        db.get_study(plan.context.project_id, plan.context.study_id)
            .unwrap()
            .revision,
        1
    );
    assert_eq!(
        db2.get_study(plan2.context.project_id, plan2.context.study_id)
            .unwrap()
            .revision,
        1
    );
}
#[test]
fn canonical_sink_revalidates_duplicate_beat_ids_without_partial_writes() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;
    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    let existing_beat_id = batch.candidates[0].draft.beats[0].id.clone();
    batch.candidates[0].draft.beats[1].id = existing_beat_id;
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn recovery_requires_original_source_links_but_accepts_later_human_attachments() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, batch) = canonical_batch_fixture();
    let mut store = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    let ids = store.commit_three(&batch).unwrap();
    let primary = batch.candidates[0].lineage[0].source_id;
    let additional = plan.context.sources[1].study_source_id;
    assert_ne!(primary, additional);

    // The editor may attach a new, available source after acceptance.
    db.attach_source_to_draft(plan.context.project_id, ids[0], additional)
        .unwrap();
    assert_eq!(store.recover_committed(&plan).unwrap(), Some(ids));

    // But removing an original generated citation must invalidate recovery;
    // a later human attachment cannot stand in for lost original provenance.
    let connection = Connection::open(db.path()).unwrap();
    connection
        .execute(
            "DELETE FROM draft_source_links
             WHERE project_id=?1 AND draft_id=?2 AND study_source_id=?3",
            params![
                plan.context.project_id.to_string(),
                ids[0].to_string(),
                primary.to_string()
            ],
        )
        .unwrap();
    assert!(matches!(
        store.recover_committed(&plan),
        Err(WritingWorkflowError::PersistFailed(_))
    ));
    assert_eq!(
        db.list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
            .unwrap()
            .len(),
        3,
        "failed reconciliation must never regenerate a batch"
    );
}

#[test]
fn direct_sqlite_acceptance_rejects_invalid_angle_outline() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    batch.candidates[0].angle.outline.clear();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn direct_sqlite_acceptance_refuses_forged_generated_review_locators() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
    batch.candidates[0]
        .review_findings
        .push(scriptotar_ai::writing::ReviewFinding {
            beat_id: Some("nonexistent-beat".into()),
            concern: "Invented evaluation".into(),
            recommendation: "Refer to a source outside the Study".into(),
            evidence_ids: vec!["foreign-evidence".into()],
            needs_human_review: false,
        });
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert!(matches!(
        sink.commit_three(&batch),
        Err(WritingWorkflowError::InvalidContext(_))
    ));
    assert!(db
        .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
        .unwrap()
        .is_empty());
}

#[test]
fn short_near_duplicate_prose_rejected_before_provider_batch_commit_and_retry() {
    let (temp, plan, current, mut gate, mut provider, stop) = setup();
    let first = "A neighbor finds the map and explains three easy ways to reach school.";
    let second = "A neighbor finds the map and explains three simple ways to reach school.";
    let updated = |index: usize, narration: &str| {
        let mut draft: serde_json::Value = serde_json::from_str(&draft_json(index)).unwrap();
        draft["beats"][0]["narration"] = json!(narration);
        draft.to_string()
    };
    provider.responses = [
        angles_json(),
        updated(1, first),
        updated(2, second),
        draft_json(3),
    ]
    .into();
    let mut sink =
        SqlFixtureSink::open(&temp.path().join("short-angles.sqlite"), current.0.clone());
    assert!(matches!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    assert_eq!(provider.calls, 4);
    assert_eq!(sink.count(), 0);
    // Four durable successful mock responses can be read again, but must
    // never cause another paid request or create an accepted bad batch.
    assert!(matches!(
        generate_three(&plan, &current, &mut gate, &mut provider, &stop, &mut sink),
        Err(WritingWorkflowError::DuplicateOrSimilar)
    ));
    assert_eq!(provider.calls, 4);
    assert_eq!(sink.count(), 0);
}

#[test]
fn short_near_duplicate_english_and_arabic_are_rejected_by_canonical_sqlite_sink() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let english = (
        "A neighbor finds the map and explains three easy ways to reach school.",
        "A neighbor finds the map and explains three simple ways to reach school.",
    );
    let arabic = (
        "في هذا الحي نراجع ثلاث خطوات بسيطة قبل زيارة المكتبة مع أصدقائنا اليوم",
        "في هذا الحي نراجع ثلاث خطوات سهلة قبل زيارة المكتبة مع أصدقائنا اليوم",
    );
    for (first, second) in [english, arabic] {
        let (_temp, db, plan, current, mut batch) = canonical_batch_fixture();
        batch.candidates[0].draft.beats[0].narration = first.into();
        batch.candidates[1].draft.beats[0].narration = second.into();
        assert!(matches!(
            study_writing::enforce_candidate_originality(&batch.context, &batch.candidates),
            Err(WritingWorkflowError::DuplicateOrSimilar)
        ));
        let mut sink = CanonicalSqliteCandidateStore {
            store: &db,
            current: &current,
        };
        assert!(matches!(
            sink.commit_three(&batch),
            Err(WritingWorkflowError::DuplicateOrSimilar)
        ));
        assert!(db
            .list_draft_summaries(plan.context.project_id, plan.context.study_id, 200)
            .unwrap()
            .is_empty());
        assert_eq!(
            db.get_study(plan.context.project_id, plan.context.study_id)
                .unwrap()
                .revision,
            1
        );
        let count: i64 = Connection::open(db.path())
            .unwrap()
            .query_row("SELECT COUNT(*) FROM study_writing_batches", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "rejected batch must not leave a receipt");
    }
}

#[test]
fn genuinely_different_short_scripts_can_share_a_common_cta() {
    use study_writing_persistence::CanonicalSqliteCandidateStore;

    let (_temp, db, _plan, current, mut batch) = canonical_batch_fixture();
    batch.candidates[0].draft.beats[0].narration =
        "A school volunteer shares a personal diary about making friends in the town.".into();
    batch.candidates[1].draft.beats[0].narration =
        "Compare ticket costs and write a practical checklist before taking the bus.".into();
    batch.candidates[2].draft.beats[0].narration =
        "Follow the old river bridge to discover a surprising community tradition.".into();
    for candidate in &mut batch.candidates {
        candidate.draft.beats[1].narration = "Share your opinion about this idea.".into();
    }
    study_writing::enforce_candidate_originality(&batch.context, &batch.candidates).unwrap();
    let mut sink = CanonicalSqliteCandidateStore {
        store: &db,
        current: &current,
    };
    assert_eq!(sink.commit_three(&batch).unwrap().len(), 3);
}
