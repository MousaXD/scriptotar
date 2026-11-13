//! Project-scoped adapters from canonical SQLite Content Studies to Studio UI.
use super::AppServices;
use crate::dto::{
    UiStudioBrief, UiStudioDraftVersion, UiStudioSourceRef, UiStudioStudy, UiStudioStudyPatch,
    UiStudioStudySummary,
};
use scriptotar_core::{
    CreativeBriefInput, RepositoryError, RepositoryResult, StudyEdit, StudySourceInput,
    StudySourceKind,
};
use std::collections::HashSet;
use uuid::Uuid;

fn invalid(label: &str) -> RepositoryError {
    RepositoryError::Validation(format!("invalid {label}"))
}

fn brief_input(value: &UiStudioBrief) -> RepositoryResult<CreativeBriefInput> {
    let seconds = value
        .duration
        .trim()
        .parse::<u32>()
        .map_err(|_| invalid("duration"))?;
    if seconds == 0 || seconds > 3600 {
        return Err(invalid("duration"));
    }
    Ok(CreativeBriefInput {
        audience: value.audience.clone(),
        goal: value.goal.clone(),
        language: value.language.clone(),
        region: None,
        platform: value.platform.clone(),
        tone: value.style.clone(),
        duration_seconds: Some(seconds),
        objective: value.goal.clone(),
        cta: None,
        constraints: Vec::new(),
    })
}

fn source_input(source: &UiStudioSourceRef) -> RepositoryResult<StudySourceInput> {
    let kind = match source.kind.as_str() {
        "research" => StudySourceKind::ResearchItem,
        "transcript" => StudySourceKind::Transcript,
        _ => return Err(invalid("source kind")),
    };
    let id = Uuid::parse_str(&source.id).map_err(|_| invalid("source id"))?;
    Ok(StudySourceInput {
        kind,
        reference_id: Some(id),
        locator: None,
        manual_text: None,
        source_revision: None,
        uncertainty: None,
    })
}

fn ui_source(source: &scriptotar_core::StudySource) -> Option<UiStudioSourceRef> {
    if !source.available {
        return None;
    }
    let kind = match source.kind {
        StudySourceKind::ResearchItem => "research",
        StudySourceKind::Transcript => "transcript",
        _ => return None,
    };
    source.reference_id.map(|id| UiStudioSourceRef {
        kind: kind.to_owned(),
        id: id.to_string(),
    })
}

impl AppServices {
    fn lock_studio_project(
        &self,
        project_id: Uuid,
    ) -> RepositoryResult<std::sync::MutexGuard<'_, Uuid>> {
        let guard = self
            .active_project
            .lock()
            .map_err(|_| RepositoryError::Storage("active project lock poisoned".into()))?;
        if *guard != project_id {
            return Err(RepositoryError::NotFound("project is not active".into()));
        }
        Ok(guard)
    }

    fn read_studio_bundle(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<UiStudioStudy> {
        let study = self.store.get_study(project_id, study_id)?;
        let saved_brief = self.store.get_creative_brief(project_id, study_id)?;
        let brief = if let Some(ref value) = saved_brief {
            UiStudioBrief {
                title: study.title,
                goal: value.input.goal.clone(),
                audience: value.input.audience.clone(),
                platform: value.input.platform.clone(),
                language: value.input.language.clone(),
                duration: value
                    .input
                    .duration_seconds
                    .map_or(String::new(), |v| v.to_string()),
                style: value.input.tone.clone(),
            }
        } else {
            UiStudioBrief {
                title: study.title,
                goal: study.goal,
                audience: String::new(),
                platform: String::new(),
                language: String::new(),
                duration: "40".into(),
                style: String::new(),
            }
        };
        let sources = self
            .store
            .list_study_sources(project_id, study_id)?
            .iter()
            .filter_map(ui_source)
            .collect();
        let drafts = self
            .store
            .search_drafts(project_id, study_id, "", false, 200)?;
        let (draft, draft_versions) = if let Some(value) = drafts.first() {
            let versions = self
                .store
                .list_draft_versions(project_id, value.id)?
                .into_iter()
                .map(|v| UiStudioDraftVersion {
                    id: v.id.to_string(),
                    revision: v.version,
                    content: v.body,
                    created_at: v.saved_at,
                })
                .collect();
            (value.body.clone(), versions)
        } else {
            (String::new(), Vec::new())
        };
        Ok(UiStudioStudy {
            id: study_id.to_string(),
            project_id: project_id.to_string(),
            revision: study.revision,
            brief,
            sources,
            draft,
            draft_versions,
        })
    }

    pub fn list_studio_studies(
        &self,
        project_id: Uuid,
    ) -> RepositoryResult<Vec<UiStudioStudySummary>> {
        let _project_guard = self.lock_studio_project(project_id)?;
        self.store
            .search_studies(project_id, "", false, 200)?
            .into_iter()
            .map(|study| {
                let row = self.read_studio_bundle(project_id, study.id)?;
                Ok(UiStudioStudySummary {
                    id: row.id,
                    project_id: row.project_id,
                    revision: row.revision,
                    brief: row.brief,
                })
            })
            .collect()
    }

    pub fn get_studio_study(
        &self,
        project_id: Uuid,
        study_id: Uuid,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        self.read_studio_bundle(project_id, study_id)
    }

    pub fn create_studio_study(
        &self,
        project_id: Uuid,
        brief: UiStudioBrief,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        let input = brief_input(&brief)?;
        let study_id =
            self.store
                .create_studio_bundle(project_id, &brief.title, &brief.goal, &input)?;
        self.read_studio_bundle(project_id, study_id)
    }

    pub fn save_studio_study(
        &self,
        project_id: Uuid,
        study_id: Uuid,
        expected_revision: i64,
        patch: UiStudioStudyPatch,
    ) -> RepositoryResult<UiStudioStudy> {
        let _project_guard = self.lock_studio_project(project_id)?;
        let study = self.store.get_study(project_id, study_id)?;
        if study.revision != expected_revision {
            return Err(RepositoryError::Conflict(
                "study changed by another editor".into(),
            ));
        }
        let old = self.read_studio_bundle(project_id, study_id)?;
        let brief = patch.brief.unwrap_or_else(|| old.brief.clone());
        let sources = patch.sources.unwrap_or_else(|| old.sources.clone());
        let draft = patch.draft.unwrap_or_else(|| old.draft.clone());
        let input = brief_input(&brief)?;
        if sources.len() > 100 {
            return Err(invalid("source count"));
        }
        let prior_sources = self.store.list_study_sources(project_id, study_id)?;
        let present: HashSet<(String, String)> = prior_sources
            .iter()
            .filter_map(ui_source)
            .map(|item| (item.kind, item.id))
            .collect();
        let mut unique = HashSet::new();
        for source in &sources {
            let input = source_input(source)?;
            if !unique.insert((source.kind.as_str(), source.id.as_str())) {
                return Err(invalid("duplicate source"));
            }
            if !present.contains(&(source.kind.clone(), source.id.clone())) {
                self.store.validate_study_origin(
                    project_id,
                    input.kind,
                    input.reference_id.ok_or_else(|| invalid("source id"))?,
                )?;
            }
        }

        let canonical_sources = sources
            .iter()
            .map(source_input)
            .collect::<RepositoryResult<Vec<_>>>()?;
        let selected_sources = canonical_sources
            .iter()
            .map(|value| {
                Ok((
                    value.kind,
                    value.reference_id.ok_or_else(|| invalid("source id"))?,
                ))
            })
            .collect::<RepositoryResult<Vec<_>>>()?;
        self.store.save_studio_bundle(
            project_id,
            study_id,
            scriptotar_db::StudioBundleUpdate {
                expected_revision,
                edit: &StudyEdit {
                    title: brief.title.clone(),
                    goal: brief.goal.clone(),
                    status: study.status,
                },
                brief: &input,
                brief_changed: brief != old.brief,
                sources: &selected_sources,
                draft: &draft,
            },
        )?;
        self.read_studio_bundle(project_id, study_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn example_brief() -> UiStudioBrief {
        UiStudioBrief {
            title: "Content study".into(),
            goal: "Write an original script about a product".into(),
            audience: "Regional creators".into(),
            platform: "TikTok".into(),
            language: "Arabic".into(),
            duration: "40".into(),
            style: "Clear".into(),
        }
    }

    #[test]
    fn title_only_study_can_be_saved_and_reopened_without_inventing_brief_fields() {
        let dir = TempDir::new().unwrap();
        let project_id;
        let study_id;
        let mut partial = example_brief();
        partial.title = "A new idea".into();
        partial.goal.clear();
        partial.audience.clear();
        partial.style.clear();
        {
            let services = AppServices::new(dir.path()).unwrap();
            project_id = services.active_project_id().unwrap();
            let created = services
                .create_studio_study(project_id, partial.clone())
                .unwrap();
            study_id = Uuid::parse_str(&created.id).unwrap();
            assert_eq!(created.brief, partial);
            assert!(created.draft.is_empty());
            assert_eq!(created.revision, 1);
        }
        let services = AppServices::new(dir.path()).unwrap();
        let opened = services.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(opened.brief, partial);
        assert_eq!(opened.revision, 1);
    }

    /// A late SQL error must not consume the study's optimistic-concurrency
    /// revision or strand half-written brief/draft state. Restart-proof.
    #[test]
    fn studio_save_rolls_back_all_resources_after_late_sql_failure() {
        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project_id = services.active_project_id().unwrap();
        let initial = services
            .create_studio_study(project_id, example_brief())
            .unwrap();
        let study_id = Uuid::parse_str(&initial.id).unwrap();
        let db = rusqlite::Connection::open(services.store.path()).unwrap();
        db.execute_batch(
            "CREATE TRIGGER test_fail_draft_write BEFORE UPDATE ON creative_drafts
             BEGIN SELECT RAISE(ABORT, 'simulated draft write interruption'); END;",
        )
        .unwrap();
        let mut revised = example_brief();
        revised.title = "Changed title".into();
        revised.audience = "Creators and producers".into();
        assert!(services
            .save_studio_study(
                project_id,
                study_id,
                initial.revision,
                UiStudioStudyPatch {
                    brief: Some(revised.clone()),
                    sources: Some(Vec::new()),
                    draft: Some("new script that must roll back".into()),
                },
            )
            .is_err());
        let unchanged = services.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(unchanged.revision, initial.revision);
        assert_eq!(unchanged.brief, initial.brief);
        assert_eq!(unchanged.draft, "");
        assert_eq!(unchanged.draft_versions.len(), 1);
        db.execute_batch("DROP TRIGGER test_fail_draft_write;")
            .unwrap();
        let saved = services
            .save_studio_study(
                project_id,
                study_id,
                initial.revision,
                UiStudioStudyPatch {
                    brief: Some(revised.clone()),
                    sources: Some(Vec::new()),
                    draft: Some("persisted after recovery".into()),
                },
            )
            .unwrap();
        assert_eq!(saved.revision, initial.revision + 1);
        assert_eq!(saved.brief, revised);
        assert_eq!(saved.draft_versions.len(), 2);
    }

    #[test]
    fn studio_create_rolls_back_study_when_initial_draft_fails() {
        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project_id = services.active_project_id().unwrap();
        let db = rusqlite::Connection::open(services.store.path()).unwrap();
        db.execute_batch(
            "CREATE TRIGGER test_fail_initial_version BEFORE INSERT ON draft_versions
             BEGIN SELECT RAISE(ABORT, 'simulated failed first version'); END;",
        )
        .unwrap();
        assert!(services
            .create_studio_study(project_id, example_brief())
            .is_err());
        assert!(services.list_studio_studies(project_id).unwrap().is_empty());
        db.execute_batch("DROP TRIGGER test_fail_initial_version;")
            .unwrap();
        let created = services
            .create_studio_study(project_id, example_brief())
            .unwrap();
        assert_eq!(created.draft_versions.len(), 1);
        assert_eq!(services.list_studio_studies(project_id).unwrap().len(), 1);
    }

    #[test]
    fn studio_persists_draft_versions_and_reopens_after_restart() {
        let dir = TempDir::new().unwrap();
        let (project_id, study_id) = {
            let services = AppServices::new(dir.path()).unwrap();
            let project_id = services.active_project_id().unwrap();
            let initial = services
                .create_studio_study(project_id, example_brief())
                .unwrap();
            let study_id = Uuid::parse_str(&initial.id).unwrap();
            assert_eq!(initial.revision, 1);
            assert_eq!(initial.brief, example_brief());
            let saved = services
                .save_studio_study(
                    project_id,
                    study_id,
                    1,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: Some(Vec::new()),
                        draft: Some("An original saved script".into()),
                    },
                )
                .unwrap();
            assert_eq!(saved.revision, 2);
            assert_eq!(saved.draft, "An original saved script");
            assert_eq!(
                saved.draft_versions.last().unwrap().content,
                "An original saved script"
            );
            assert!(matches!(
                services.save_studio_study(
                    project_id,
                    study_id,
                    1,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: None,
                        draft: Some("stale writer".into()),
                    },
                ),
                Err(RepositoryError::Conflict(_))
            ));
            assert!(services.get_studio_study(Uuid::new_v4(), study_id).is_err());
            (project_id, study_id)
        };
        let reopened = AppServices::new(dir.path()).unwrap();
        let restored = reopened.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(restored.revision, 2);
        assert_eq!(restored.draft, "An original saved script");
        assert_eq!(restored.draft_versions.len(), 2);
    }

    #[test]
    fn studio_keeps_real_source_provenance_and_rejects_linked_source_removal_atomically() {
        let dir = TempDir::new().unwrap();
        let project_id;
        let study_id;
        let research_id = Uuid::new_v4();
        let transcript_id = Uuid::new_v4();
        let evidence_id;
        {
            let services = AppServices::new(dir.path()).unwrap();
            project_id = services.active_project_id().unwrap();
            let source_id = Uuid::new_v4();
            let media_id = Uuid::new_v4();
            let conn = rusqlite::Connection::open(services.store.path()).unwrap();
            conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
            conn.execute(
                "INSERT INTO research_items(id,project_id,source_url,platform,scanned_at)
                 VALUES(?1,?2,'https://example.org/study/source','sample','2026-11-13T10:00:00Z')",
                rusqlite::params![research_id.to_string(), project_id.to_string()],
            ).unwrap();
            conn.execute(
                "INSERT INTO sources(id,project_id,creator_id,source_type,locator,created_at)
                 VALUES(?1,?2,NULL,'url','https://example.org/study/source','2026-11-13T10:00:00Z')",
                rusqlite::params![source_id.to_string(), project_id.to_string()],
            ).unwrap();
            conn.execute(
                "INSERT INTO media(id,source_id,local_path,created_at)
                 VALUES(?1,?2,'/tmp/study-fixture.mp4','2026-11-13T10:00:00Z')",
                rusqlite::params![media_id.to_string(), source_id.to_string()],
            ).unwrap();
            conn.execute(
                "INSERT INTO transcripts(id,media_id,text,created_at,updated_at)
                 VALUES(?1,?2,'Source with a timecode','2026-11-13T10:00:00Z','2026-11-13T10:00:00Z')",
                rusqlite::params![transcript_id.to_string(), media_id.to_string()],
            ).unwrap();
            let study = services.create_studio_study(project_id, example_brief()).unwrap();
            study_id = Uuid::parse_str(&study.id).unwrap();
            let sources = vec![
                UiStudioSourceRef { kind: "research".into(), id: research_id.to_string() },
                UiStudioSourceRef { kind: "transcript".into(), id: transcript_id.to_string() },
            ];
            let saved = services.save_studio_study(
                project_id, study_id, study.revision,
                UiStudioStudyPatch {
                    brief: None,
                    sources: Some(sources.clone()),
                    draft: Some("Original referenced draft".into()),
                },
            ).unwrap();
            assert_eq!(saved.sources, sources);
            assert_eq!(saved.draft, "Original referenced draft");
            let stored_sources = services.store.list_study_sources(project_id, study_id).unwrap();
            assert_eq!(stored_sources.len(), 2);
            let transcript_source = stored_sources.iter()
                .find(|item| item.reference_id == Some(transcript_id)).unwrap();
            let evidence = services.store.add_evidence_reference(
                project_id, study_id,
                &scriptotar_core::EvidenceInput {
                    study_source_id: transcript_source.id,
                    claim: "Transcript contains a literal spoken example".into(),
                    locator: "0:00-0:05".into(),
                    timestamp_seconds: Some(0.0),
                    certainty: scriptotar_core::EvidenceCertainty::Unverified,
                    uncertainty_note: None,
                },
            ).unwrap();
            evidence_id = evidence.id;
            assert!(matches!(
                services.save_studio_study(
                    project_id, study_id, saved.revision,
                    UiStudioStudyPatch {
                        brief: None,
                        sources: Some(vec![sources[0].clone()]),
                        draft: Some("Must not commit this edit".into()),
                    },
                ),
                Err(RepositoryError::Conflict(_))
            ));
            let unchanged = services.get_studio_study(project_id, study_id).unwrap();
            assert_eq!(unchanged.revision, saved.revision);
            assert_eq!(unchanged.sources, sources);
            assert_eq!(unchanged.draft, saved.draft);
        }
        let reopened = AppServices::new(dir.path()).unwrap();
        let restored = reopened.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(restored.sources.len(), 2);
        assert_eq!(restored.draft, "Original referenced draft");
        let evidence = reopened.store.get_evidence_reference(project_id, study_id, evidence_id).unwrap();
        assert_eq!(evidence.input.locator, "0:00-0:05");
    }

    #[test]
    fn invalid_source_is_rejected_before_study_revision_changes() {
        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project_id = services.active_project_id().unwrap();
        let study = services
            .create_studio_study(project_id, example_brief())
            .unwrap();
        let study_id = Uuid::parse_str(&study.id).unwrap();
        let attempt = services.save_studio_study(
            project_id,
            study_id,
            study.revision,
            UiStudioStudyPatch {
                brief: None,
                sources: Some(vec![UiStudioSourceRef {
                    kind: "transcript".into(),
                    id: Uuid::new_v4().to_string(),
                }]),
                draft: Some("must not be saved".into()),
            },
        );
        assert!(attempt.is_err());
        let unchanged = services.get_studio_study(project_id, study_id).unwrap();
        assert_eq!(unchanged.revision, study.revision);
        assert_eq!(unchanged.draft, "");
        assert!(unchanged.sources.is_empty());
    }
}
