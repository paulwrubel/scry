use crate::error::ServiceError;
use crate::service::{
    ApplyMode, EventPatch, EventWindow, FollowingEvent, NewEvent, ProjectService, SplitEvents,
    new_operation,
};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use scry_core::error::StorageError;
use scry_core::event::split_series;
use scry_core::ids::{EventId, ProjectId};
use scry_core::models::Event;
use scry_core::recurrence::RecurError;
use scry_core::sync::operations::OperationKind;
use scry_core::temporal::When;
use uuid::Uuid;

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
            overrides: Vec::new(),
            external_metadata: None,
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
                overrides: event.overrides.clone(),
                external_metadata: event.external_metadata.clone(),
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
            overrides,
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
                overrides,
                external_metadata: None,
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
                if event_overlaps_window(&event, window, zone)? {
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

    /// Split a recurring event before `original_start`, truncating the existing
    /// series and creating a following one from `following`.
    pub async fn split_event(
        &self,
        id: EventId,
        original_start: When,
        following: FollowingEvent,
        zone: &TimeZone,
    ) -> Result<SplitEvents, ServiceError> {
        let existing = self.require_event(id).await?;
        let split = split_series(&existing, &original_start, zone)
            .map_err(recurrence_error)?
            .ok_or_else(|| {
                ServiceError::Storage(StorageError::Invalid(
                    "cannot split before the series start".to_string(),
                ))
            })?;

        let new_id = Uuid::new_v4();
        let now = Timestamp::now();

        let update = new_operation(
            Uuid::new_v4(),
            OperationKind::UpdateEvent {
                id: existing.id,
                title: None,
                description: None,
                interval: None,
                recurrence: Some(Some(split.truncated.clone())),
                overrides: Some(split.kept_overrides.clone()),
                external_metadata: None,
                updated_at: Some(now),
            },
        );
        let create = new_operation(
            Uuid::new_v4(),
            OperationKind::CreateEvent {
                id: new_id,
                project_id: existing.project_id,
                title: following.title,
                description: following.description.filter(|value| !value.is_empty()),
                interval: following.interval,
                recurrence: Some(split.following.clone()),
                overrides: split.following_overrides.clone(),
                external_metadata: None,
                created_at: now,
                updated_at: Some(now),
            },
        );

        self.apply_operations(&[update, create], ApplyMode::Atomic)
            .await?;

        Ok(SplitEvents {
            previous: self.require_event(id).await?,
            following: self.require_event(new_id).await?,
        })
    }

    /// Import a VCALENDAR into a project, creating one event per series.
    ///
    /// The batch is atomic because a series and its per-occurrence overrides
    /// must land together: a component that fails to apply imports nothing.
    pub async fn import_ics(
        &self,
        project_id: ProjectId,
        input: &str,
    ) -> Result<Vec<Event>, ServiceError> {
        self.require_project(project_id).await?;

        let imported = scry_ics::import(input)
            .map_err(|error| ServiceError::Storage(StorageError::Invalid(error.to_string())))?;

        let now = Timestamp::now();
        let mut events = Vec::with_capacity(imported.len());
        let mut operations = Vec::with_capacity(imported.len());

        for series in imported {
            let event = Event {
                id: Uuid::new_v4(),
                project_id,
                title: series.title,
                description: series
                    .description
                    .filter(|description| !description.is_empty()),
                interval: series.interval,
                recurrence: series.recurrence,
                overrides: series.overrides,
                external_metadata: Some(series.external_metadata),
                created_at: now,
                updated_at: now,
            };

            if let Some(rule) = &event.recurrence {
                rule.validate(event.interval.start())
                    .map_err(recurrence_error)?;
            }

            operations.push(new_operation(
                Uuid::new_v4(),
                OperationKind::CreateEvent {
                    id: event.id,
                    project_id: event.project_id,
                    title: event.title.clone(),
                    description: event.description.clone(),
                    interval: event.interval.clone(),
                    recurrence: event.recurrence.clone(),
                    overrides: event.overrides.clone(),
                    external_metadata: event.external_metadata.clone(),
                    created_at: event.created_at,
                    updated_at: Some(now),
                },
            ));
            events.push(event);
        }

        self.apply_operations(&operations, ApplyMode::Atomic)
            .await?;
        Ok(events)
    }

    /// Export a project's events as a VCALENDAR, read in UTC.
    ///
    /// Reading in UTC only affects how a date or floating time is anchored; no
    /// series can be lost because of it.
    pub async fn export_ics(&self, project_id: ProjectId) -> Result<String, ServiceError> {
        let events = self.list_events(project_id, None, &TimeZone::UTC).await?;
        scry_ics::export_events(&events)
            .map_err(|error| ServiceError::Storage(StorageError::Invalid(error.to_string())))
    }
}

/// Whether an event has any occurrence in `window`, expanding a recurring
/// series with its per-occurrence overrides.
fn event_overlaps_window(
    event: &Event,
    window: EventWindow,
    zone: &TimeZone,
) -> Result<bool, ServiceError> {
    event
        .occurrences(window.from, window.to, zone)
        .map(|occurrences| !occurrences.is_empty())
        .map_err(recurrence_error)
}

fn recurrence_error(error: RecurError) -> ServiceError {
    ServiceError::Storage(StorageError::Invalid(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;
    use scry_core::recurrence::RRule;
    use scry_core::temporal::{Interval, When};

    fn instant(value: &str) -> Timestamp {
        value.parse().unwrap()
    }

    fn all_day(start: i8, end: i8) -> Interval {
        Interval::new(
            When::Date {
                date: date(2026, 11, start),
            },
            When::Date {
                date: date(2026, 11, end),
            },
        )
        .unwrap()
    }

    fn event(interval: Interval, recurrence: Option<RRule>) -> Event {
        Event {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            title: "Event".to_string(),
            description: None,
            interval,
            recurrence,
            overrides: Vec::new(),
            external_metadata: None,
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        }
    }

    fn window(from: &str, to: &str) -> EventWindow {
        EventWindow {
            from: instant(from),
            to: instant(to),
        }
    }

    #[test]
    fn a_recurring_series_matches_a_later_occurrence() {
        let event = event(
            all_day(2, 3),
            Some(RRule {
                text: "FREQ=WEEKLY".to_string(),
                exdates: Vec::new(),
                rdates: Vec::new(),
            }),
        );

        assert!(
            event_overlaps_window(
                &event,
                window("2026-11-09T00:00:00Z", "2026-11-10T00:00:00Z"),
                &TimeZone::UTC,
            )
            .unwrap()
        );
    }

    #[test]
    fn a_non_recurring_event_does_not_match_a_later_window() {
        let event = event(all_day(2, 3), None);

        assert!(
            !event_overlaps_window(
                &event,
                window("2026-11-09T00:00:00Z", "2026-11-10T00:00:00Z"),
                &TimeZone::UTC,
            )
            .unwrap()
        );
    }

    #[test]
    fn an_excluded_occurrence_does_not_match() {
        let event = event(
            all_day(2, 3),
            Some(RRule {
                text: "FREQ=WEEKLY".to_string(),
                exdates: vec![When::Date {
                    date: date(2026, 11, 9),
                }],
                rdates: Vec::new(),
            }),
        );

        assert!(
            !event_overlaps_window(
                &event,
                window("2026-11-09T00:00:00Z", "2026-11-10T00:00:00Z"),
                &TimeZone::UTC,
            )
            .unwrap()
        );
    }
}
