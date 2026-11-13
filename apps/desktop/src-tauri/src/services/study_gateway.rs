//! Project-scoped adapters from canonical SQLite Content Studies to Studio UI.
use std::collections::HashSet;
use scriptotar_core::{
    CreativeBriefInput, RepositoryError, RepositoryResult, StudyEdit, StudySourceInput,
    StudySourceKind,
};
use uuid::Uuid;
use super::AppServices;
use crate::dto::{
    UiStudioBrief, UiStudioDraftVersion, UiStudioSourceRef, UiStudioStudy, UiStudioStudyPatch,
    UiStudioStudySummary,
};

fn invalid(label: &str) -> RepositoryError {
    RepositoryError::Validation(format!("invalid {label}"))
}

fn brief_input(value: &UiStudioBrief) -> RepositoryResult<CreativeBriefInput> {
    let seconds = value.duration.trim().parse::<u32>().map_err(|_| invalid("duration"))?;
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
        let drafts = self.store.search_drafts(project_id, study_id, "", false, 200)?;
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
        let study = self.store.create_study(project_id, &brief.title, &brief.goal)?;
        self.store.save_creative_brief(project_id, study.id, 0, &input)?;
        self.store.create_draft(project_id, study.id, &brief.title, "")?;
        self.read_studio_bundle(project_id, study.id)
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
            return Err(RepositoryError::Conflict("study changed by another editor".into()));
        }
        let old = self.read_studio_bundle(project_id, study_id)?;
        let brief = patch.brief.unwrap_or_else(|| old.brief.clone());
        let sources = patch.sources.unwrap_or_else(|| old.sources.clone());
        let draft = patch.draft.unwrap_or_else(|| old.draft.clone());
        let input = brief_input(&brief)?;
        if sources.len() > 100 {
            return Err(invalid("source count"));
        }
        let mut unique = HashSet::new();
        for source in &sources {
            source_input(source)?;
            if !unique.insert((source.kind.as_str(), source.id.as_str())) {
                return Err(invalid("duplicate source"));
            }
        }

        // Claim the aggregate optimistic-concurrency token. The underlying
        // brief and draft keep independent versions in the canonical DB.
        self.store.update_study(
            project_id,
            study_id,
            expected_revision,
            &StudyEdit {
                title: brief.title.clone(),
                goal: brief.goal.clone(),
                status: study.status,
            },
        )?;
        if brief != old.brief {
            let saved = self.store.get_creative_brief(project_id, study_id)?;
            self.store.save_creative_brief(
                project_id,
                study_id,
                saved.map_or(0, |value| value.revision),
                &input,
            )?;
        }
        let existing = self.store.list_study_sources(project_id, study_id)?;
        let wanted: HashSet<(String, String)> = sources
            .iter()
            .map(|item| (item.kind.clone(), item.id.clone()))
            .collect();
        for source in &existing {
            if let Some(item) = ui_source(source) {
                if !wanted.contains(&(item.kind, item.id)) {
                    self.store.remove_study_source(project_id, study_id, source.id)?;
                }
            }
        }
        let present: HashSet<(String, String)> = existing
            .iter()
            .filter_map(ui_source)
            .map(|item| (item.kind, item.id))
            .collect();
        for source in sources {
            if !present.contains(&(source.kind.clone(), source.id.clone())) {
                self.store.add_study_source(project_id, study_id, &source_input(&source)?)?;
            }
        }
        let drafts = self.store.search_drafts(project_id, study_id, "", false, 200)?;
        if let Some(value) = drafts.first() {
            if value.body != draft || value.title != brief.title {
                self.store.update_draft(
                    project_id,
                    value.id,
                    value.revision,
                    &brief.title,
                    &draft,
                )?;
            }
        } else {
            self.store.create_draft(project_id, study_id, &brief.title, &draft)?;
        }
        self.read_studio_bundle(project_id, study_id)
    }
}
