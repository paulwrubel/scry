use crate::error::ServiceError;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use scry_core::error::StorageError;
use scry_core::ids::{EventId, ProjectId};
use scry_core::models::Event;
use scry_core::recurrence::RecurError;
use scry_core::sync::operations::OperationKind;
use scry_core::temporal::TemporalError;
use uuid::Uuid;

use super::super::{ApplyMode, EventPatch, EventWindow, NewEvent, ProjectService, new_operation};

impl ProjectService {
    /// Create an event in a project.
    pub async fn create_event_in_project(
        &self,
        project_id: ProjectId,
        new: NewEvent,
    ) -> Result<Event, ServiceError> {
        self.require_project(project_id).await?;

        let NewEvent {
            title,
            description,
            interval,
            recurrence,
        } = new;

        if let Some(rule) = &recurrence {
            rule.validate(interval.start()).map_err(recurrence_error)?;
        }

        let created_at = Timestamp::now();
        let event = Event {
            id: Uuid::new_v4(),
            project_id,
            title,
            description: description.filter(|description| !description.is_empty()),
            interval,
            recurrence,
            created_at,
            updated_at: created_at,
        };
        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateEvent {
                id: event.id,
                project_id: event.project_id,
                title: event.title.clone(),
                description: event.description.clone(),
                interval: event.interval.clone(),
                recurrence: event.recurrence.clone(),
                created_at: event.created_at,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(event)
    }

    /// Update an event's title, description, interval, or recurrence.
    pub async fn update_event(
        &self,
        id: EventId,
        patch: EventPatch,
    ) -> Result<Event, ServiceError> {
        let existing = self.require_event(id).await?;

        let EventPatch {
            title,
            description,
            interval,
            recurrence,
        } = patch;

        let description = description.map(|value| Some(value).filter(|value| !value.is_empty()));

        let effective_interval = interval.as_ref().unwrap_or(&existing.interval);
        let effective_recurrence = match &recurrence {
            Some(value) => value.clone(),
            None => existing.recurrence.clone(),
        };
        if let Some(rule) = &effective_recurrence {
            rule.validate(effective_interval.start())
                .map_err(recurrence_error)?;
        }

        let op = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateEvent {
                id,
                title,
                description,
                interval,
                recurrence,
                updated_at: None,
            },
        );
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        self.require_event(id).await
    }

    /// Delete an event, returning the deleted row.
    pub async fn delete_event(&self, id: EventId) -> Result<Event, ServiceError> {
        let existing = self.require_event(id).await?;
        let op = new_operation(Uuid::new_v4(), OperationKind::DeleteEvent { id });
        self.apply_operations(&[op], ApplyMode::Atomic).await?;
        Ok(existing)
    }

    pub async fn get_event(&self, id: EventId) -> Result<Option<Event>, ServiceError> {
        Ok(self.store.get_event_by_id(id, None).await?)
    }

    /// List a project's events ordered by their start, optionally keeping only
    /// those that overlap `window`. A date or floating value is read in `zone`,
    /// because it carries no instant of its own.
    pub async fn list_events(
        &self,
        project_id: ProjectId,
        window: Option<EventWindow>,
        zone: &TimeZone,
    ) -> Result<Vec<Event>, ServiceError> {
        let mut events = self
            .store
            .get_all_events_by_project_id(project_id, None)
            .await?;

        if let Some(window) = window {
            let mut kept = Vec::with_capacity(events.len());
            for event in events {
                if event
                    .interval
                    .overlaps(window.from, window.to, zone)
                    .map_err(temporal_error)?
                {
                    kept.push(event);
                }
            }
            events = kept;
        }

        events.sort_by(|left, right| {
            left.interval
                .start_instant(zone)
                .ok()
                .cmp(&right.interval.start_instant(zone).ok())
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(events)
    }
}

fn temporal_error(error: TemporalError) -> ServiceError {
    ServiceError::Storage(StorageError::Invalid(error.to_string()))
}

fn recurrence_error(error: RecurError) -> ServiceError {
    ServiceError::Storage(StorageError::Invalid(error.to_string()))
}
