use crate::error::ServiceError;
use crate::service::{ApplyMode, EventPatch, EventWindow, NewEvent, ProjectService, new_operation};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use scry_core::error::StorageError;
use scry_core::ids::{EventId, ProjectId};
use scry_core::models::Event;
use scry_core::recurrence::{RecurError, occurrences};
use scry_core::sync::operations::OperationKind;
use scry_core::temporal::TemporalError;
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
}

/// Whether an event has any occurrence in `window`. A recurring series is
/// expanded only far enough to answer that question; its occurrences are never
/// materialized.
fn event_overlaps_window(
    event: &Event,
    window: EventWindow,
    zone: &TimeZone,
) -> Result<bool, ServiceError> {
    if let Some(recurrence) = &event.recurrence {
        return occurrences(&event.interval, recurrence, window.from, window.to, zone)
            .map(|occurrences| !occurrences.is_empty())
            .map_err(recurrence_error);
    }

    event
        .interval
        .overlaps(window.from, window.to, zone)
        .map_err(temporal_error)
}

fn temporal_error(error: TemporalError) -> ServiceError {
    ServiceError::Storage(StorageError::Invalid(error.to_string()))
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
