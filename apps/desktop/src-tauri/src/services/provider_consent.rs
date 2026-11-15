//! Host-owned project/Study scope for explicit provider-consent previews.
//!
//! No Tauri command is registered here: Agent 00 owns the public IPC contract.
//! No paid execution can occur through this adapter until Agent 02 delivers
//! transactional remote-effect fencing AND bounded provider usage receipts.
use super::AppServices;
use scriptotar_ai::{
    provider_consent::{
        sha256_hex, ConsentBoundary, ConsentError, ConsentPreview, DisabledPaidEffectFence,
        ExistingAiServiceAdapter, NoTrustedPricing, ProviderIntent, RunBudget, ScopeAuthority,
        StudyScope,
    },
    ProviderConfig,
};
use scriptotar_core::{RepositoryError, RepositoryResult};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[allow(dead_code)] // Agent 00 has not yet registered the IPC lifecycle.
fn current_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn scope_error(_error: RepositoryError) -> ConsentError {
    // Do not leak project existence, source material, credentials or SQL errors.
    ConsentError::WrongProjectOrStudy
}

#[allow(dead_code)] // Used only when the owner wires the host approval lifecycle.
fn consent_error(error: ConsentError) -> RepositoryError {
    RepositoryError::Validation(format!("AI provider consent: {error:?}"))
}

impl ScopeAuthority for AppServices {
    fn current_scope(&self, project_id: &str, study_id: &str) -> Result<StudyScope, ConsentError> {
        let project = Uuid::parse_str(project_id).map_err(|_| ConsentError::WrongProjectOrStudy)?;
        let study = Uuid::parse_str(study_id).map_err(|_| ConsentError::WrongProjectOrStudy)?;
        self.require_active_project(project).map_err(scope_error)?;

        let current = self.store.get_study(project, study).map_err(scope_error)?;
        let brief = self
            .store
            .get_creative_brief(project, study)
            .map_err(scope_error)?;
        let sources = self
            .store
            .list_study_sources(project, study)
            .map_err(scope_error)?;
        let evidence = self
            .store
            .list_evidence_references(project, study)
            .map_err(scope_error)?;

        // The canonical writing service rehydrates the current transcript
        // excerpts and quarantines disputed observations. Its literal prompt
        // fingerprint invalidates consent even if the Study's revision was not
        // bumped by a new transcript excerpt.
        let prompt = self
            .prepare_studio_angle_prompt(project, study)
            .map_err(scope_error)?
            .prompt;
        self.require_active_project(project).map_err(scope_error)?;
        let material = serde_json::to_vec(&(&current, &brief, &sources, &evidence, &prompt))
            .map_err(|_| ConsentError::ChangedContext)?;
        let mut selected_source_ids = sources
            .iter()
            .filter(|source| source.available)
            .take(8)
            .map(|source| source.id.to_string())
            .collect::<Vec<_>>();
        selected_source_ids.sort();
        selected_source_ids.dedup();
        Ok(StudyScope {
            project_id: current.project_id.to_string(),
            study_id: current.id.to_string(),
            study_revision: current.revision,
            material_digest: sha256_hex(&material),
            expected_prompt_digest: sha256_hex(prompt.as_bytes()),
            selected_source_ids,
        })
    }
}

#[allow(dead_code)] // Rust-only integration surface reserved for Agent 00.
impl AppServices {
    /// Prepare the exact server-derived three-angle request, never an arbitrary
    /// model prompt supplied by the client. Approval remains blocked if the
    /// trusted catalog has no verified ceiling for this specific model.
    pub fn prepare_provider_angle_consent(
        &self,
        boundary: &ConsentBoundary,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        provider: ProviderConfig,
        max_output_tokens: u64,
        limits: RunBudget,
    ) -> RepositoryResult<ConsentPreview> {
        let prompt = self.prepare_studio_angle_prompt(project, study)?.prompt;
        let scope = self
            .current_scope(&project.to_string(), &study.to_string())
            .map_err(consent_error)?;
        let intent = ProviderIntent {
            run_id: run_id.to_owned(),
            scope,
            provider,
            prompt,
            max_output_tokens,
            limits,
        };
        boundary
            .prepare(intent, self, &NoTrustedPricing, current_ms())
            .map_err(consent_error)
    }

    pub fn decide_provider_angle_consent(
        &self,
        boundary: &ConsentBoundary,
        run_id: &str,
        fingerprint: &str,
        approve: bool,
    ) -> RepositoryResult<()> {
        boundary
            .decide(run_id, fingerprint, approve, self, current_ms())
            .map_err(consent_error)
    }

    /// Must not dispatch: the production adapter has no hard token ceiling or
    /// trustworthy usage receipt, and the paid effect fence is not integrated.
    /// This remains a callable deny-by-default boundary for the owner to wire
    /// after approving the typed API and Agent 02's ledger contract.
    pub fn dispatch_provider_angle_consent_disabled(
        &self,
        boundary: &ConsentBoundary,
        intent: &ProviderIntent,
        request_time_api_key: &str,
    ) -> RepositoryResult<String> {
        boundary
            .dispatch(
                intent,
                self,
                &NoTrustedPricing,
                &DisabledPaidEffectFence,
                &ExistingAiServiceAdapter,
                request_time_api_key,
                current_ms(),
            )
            .map(|result| result.text)
            .map_err(consent_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::UiStudioBrief;
    use tempfile::TempDir;

    #[test]
    fn host_rejects_foreign_project_and_never_approves_unknown_pricing() {
        let directory = TempDir::new().unwrap();
        let services = AppServices::new(directory.path()).unwrap();
        let project = services.active_project_id().unwrap();
        assert!(matches!(
            services.current_scope(&Uuid::new_v4().to_string(), &Uuid::new_v4().to_string()),
            Err(ConsentError::WrongProjectOrStudy)
        ));

        let study = services
            .create_studio_study(
                project,
                UiStudioBrief {
                    title: "Consent boundary".into(),
                    goal: "Draft original script".into(),
                    audience: "Viewers".into(),
                    platform: "Reels".into(),
                    language: "English".into(),
                    style: "Clear".into(),
                    duration: "30".into(),
                },
            )
            .unwrap();
        let boundary = ConsentBoundary::default();
        let result = services.prepare_provider_angle_consent(
            &boundary,
            project,
            Uuid::parse_str(&study.id).unwrap(),
            "provider-test",
            ProviderConfig {
                provider: scriptotar_ai::ProviderKind::OpenAi,
                model: "unsupported-model".into(),
                base_url: None,
            },
            500,
            RunBudget {
                max_tokens: 10_000,
                max_tool_calls: 1,
                max_elapsed_ms: 10_000,
                max_cost_microusd: 2_000,
            },
        );
        // No selected source is a valid input for agentic paid Study generation.
        assert!(result.is_err());
    }
}
