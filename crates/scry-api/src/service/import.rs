use crate::error::ServiceError;
use scry_core::error::StorageError;
use scry_core::ids::ProjectId;
use scry_core::models::EntityType;
use scry_core::snapshot::{
    self, ImportMode, ImportReport, SNAPSHOT_VERSION, Snapshot, SnapshotFile,
};
use sqlx::{Postgres, Transaction};
use std::collections::HashSet;

use super::{ApplyMode, AutomationEvaluation, ProjectService};

impl ProjectService {
    /// Read one consistent server-wide snapshot plus its cursor.
    pub async fn snapshot(&self, project: Option<ProjectId>) -> Result<Snapshot, ServiceError> {
        Ok(snapshot::build(self.store.as_ref(), project).await?)
    }

    /// Wrap a snapshot in the exportable file envelope. `include_seq` adds the
    /// cursor to the file; ordinary exports leave it out.
    pub async fn snapshot_file(
        &self,
        project: Option<ProjectId>,
        include_seq: bool,
    ) -> Result<SnapshotFile, ServiceError> {
        let snapshot = self.snapshot(project).await?;
        Ok(SnapshotFile {
            version: SNAPSHOT_VERSION,
            app_version: String::new(),
            projects: snapshot.projects,
            last_seen_seq: include_seq.then_some(snapshot.last_seen_seq),
        })
    }

    /// Apply a snapshot file in one atomic batch.
    pub async fn import(
        &self,
        file: &SnapshotFile,
        mode: ImportMode,
    ) -> Result<ImportReport, ServiceError> {
        let mut tx = self.store.begin_apply().await?;
        let state = self.load_import_state(&mut tx, file).await?;
        let plan = snapshot::plan_import(file, &state, mode)?;
        self.apply_batch(
            &mut tx,
            &plan.operations,
            ApplyMode::Atomic,
            AutomationEvaluation::Skip,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|error| StorageError::Database(format!("failed to commit import: {error}")))?;
        Ok(plan.report)
    }

    /// Load the live rows and file-referenced tombstones the planner needs,
    /// inside the import's own transaction.
    async fn load_import_state(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        file: &SnapshotFile,
    ) -> Result<snapshot::ImportState, ServiceError> {
        let projects = self.store.get_all_projects(Some(&mut **tx)).await?;
        let mut sections = Vec::new();
        let mut tasks = Vec::new();
        let mut comments = Vec::new();
        let mut automations = Vec::new();
        let mut events = Vec::new();

        for project in &projects {
            sections.extend(
                self.store
                    .get_all_sections_by_project_id(project.id, Some(&mut **tx))
                    .await?,
            );
            tasks.extend(
                self.store
                    .get_all_tasks_by_project_id(project.id, Some(&mut **tx))
                    .await?,
            );
            comments.extend(
                self.store
                    .get_all_comments_by_project_id(project.id, Some(&mut **tx))
                    .await?,
            );
            automations.extend(
                self.store
                    .get_all_automations_by_project_id(project.id, Some(&mut **tx))
                    .await?,
            );
            events.extend(
                self.store
                    .get_all_events_by_project_id(project.id, Some(&mut **tx))
                    .await?,
            );
        }

        let mut referenced = Vec::new();
        for document in &file.projects {
            referenced.push((EntityType::Project, document.project.id));
            referenced.extend(
                document
                    .sections
                    .iter()
                    .map(|section| (EntityType::Section, section.id)),
            );
            referenced.extend(
                document
                    .tasks
                    .iter()
                    .map(|task| (EntityType::Task, task.id)),
            );
            referenced.extend(
                document
                    .comments
                    .iter()
                    .map(|comment| (EntityType::Comment, comment.id)),
            );
            referenced.extend(
                document
                    .automations
                    .iter()
                    .map(|automation| (EntityType::Automation, automation.id)),
            );
            referenced.extend(
                document
                    .events
                    .iter()
                    .map(|event| (EntityType::Event, event.id)),
            );
        }

        let mut tombstones = HashSet::new();
        for (entity_type, id) in referenced {
            if self
                .store
                .is_tombstoned(entity_type, id, Some(&mut **tx))
                .await?
            {
                tombstones.insert((entity_type, id));
            }
        }

        Ok(snapshot::ImportState {
            projects,
            sections,
            tasks,
            comments,
            automations,
            events,
            tombstones,
        })
    }
}
