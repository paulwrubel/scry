//! Read-time expansion of an event's occurrences with per-occurrence overrides
//! applied. The series stays the source of truth: an override only replaces the
//! title, description, or interval of the occurrence whose generated start it
//! names, and an override that matches nothing is hidden.

use crate::models::{Event, EventOverride};
use crate::recurrence::{RRule, RecurError, Until, occurrences};
use crate::temporal::{Interval, TemporalError, When};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

impl Event {
    /// Expand the occurrences overlapping the half-open window `[from, to)`,
    /// applying per-occurrence overrides and skipping the ones with no
    /// generated occurrence to attach to.
    pub fn occurrences(
        &self,
        from: Timestamp,
        to: Timestamp,
        tz: &TimeZone,
    ) -> Result<Vec<EffectiveOccurrence>, RecurError> {
        let Some(rrule) = &self.recurrence else {
            let overlaps = self.interval.overlaps(from, to, tz).map_err(temporal)?;
            return Ok(if overlaps {
                vec![EffectiveOccurrence {
                    original_start: self.interval.start().clone(),
                    interval: self.interval.clone(),
                    title: self.title.clone(),
                    description: self.description.clone(),
                    overridden: false,
                }]
            } else {
                Vec::new()
            });
        };

        let mut found = Vec::new();
        let mut matched: Vec<Timestamp> = Vec::new();

        for interval in occurrences(&self.interval, rrule, from, to, tz)? {
            let original_start = interval.start().clone();
            let original_instant = original_start.start_instant(tz).map_err(temporal)?;
            let mut occurrence = EffectiveOccurrence {
                original_start,
                interval,
                title: self.title.clone(),
                description: self.description.clone(),
                overridden: false,
            };

            if let Some(overridden) = self.overrides.iter().find(|overridden| {
                overridden
                    .original_start
                    .start_instant(tz)
                    .is_ok_and(|instant| instant == original_instant)
            }) {
                matched.push(original_instant);
                if let Some(interval) = &overridden.interval {
                    occurrence.interval = interval.clone();
                }
                if let Some(title) = &overridden.title {
                    occurrence.title = title.clone();
                }
                if let Some(description) = &overridden.description {
                    occurrence.description = Some(description.clone());
                }
                occurrence.overridden = true;
            }

            found.push(occurrence);
        }

        // an override whose generated start lies outside the window is still
        // shown when its edited interval moves it into the window
        for overridden in &self.overrides {
            let Ok(original) = overridden.original_start.start_instant(tz) else {
                continue;
            };
            if matched.contains(&original) {
                continue;
            }

            let Some(generated) = generated_start(self, rrule, original, tz)? else {
                continue;
            };
            let interval = overridden
                .interval
                .clone()
                .unwrap_or_else(|| generated.clone());
            if !interval.overlaps(from, to, tz).map_err(temporal)? {
                continue;
            }

            found.push(EffectiveOccurrence {
                original_start: generated.start().clone(),
                interval,
                title: overridden
                    .title
                    .clone()
                    .unwrap_or_else(|| self.title.clone()),
                description: overridden
                    .description
                    .clone()
                    .or_else(|| self.description.clone()),
                overridden: true,
            });
        }

        sort_by_start(found, tz)
    }
}

/// One occurrence of an event as it reads after overrides are applied.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveOccurrence {
    /// The start the series generates, which an override names.
    pub original_start: When,
    /// The interval shown, which an override may have moved.
    pub interval: Interval,
    pub title: String,
    pub description: Option<String>,
    /// Whether an override replaced any field of this occurrence.
    pub overridden: bool,
}

/// The two series an event becomes when it is split before one occurrence.
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesSplit {
    /// The last occurrence the truncated series keeps.
    pub previous_start: When,
    /// How many candidates the rule generates before the split, including
    /// starts an EXDATE later removes; this is the truncated series' COUNT.
    pub occurrences_before: u32,
    pub truncated: RRule,
    pub following: RRule,
    pub kept_overrides: Vec<EventOverride>,
    pub following_overrides: Vec<EventOverride>,
}

/// Split a recurring event immediately before `original_start`.
///
/// The truncated series keeps every occurrence before the split (with its
/// COUNT or UNTIL adjusted) and the following series keeps the split and
/// everything after it. `None` means there is nothing before the split to keep.
pub fn split_series(
    event: &Event,
    original_start: &When,
    tz: &TimeZone,
) -> Result<Option<SeriesSplit>, RecurError> {
    let Some(rrule) = &event.recurrence else {
        return Ok(None);
    };

    let base = event.interval.start_instant(tz).map_err(temporal)?;
    let target = original_start.start_instant(tz).map_err(temporal)?;
    if target <= base {
        return Ok(None);
    }

    let mut previous_start = None;
    for interval in occurrences(&event.interval, rrule, base, target, tz)? {
        if interval.start_instant(tz).map_err(temporal)? < target {
            previous_start = Some(interval.start().clone());
        }
    }
    let Some(previous_start) = previous_start else {
        return Ok(None);
    };

    // COUNT budgets every candidate the rule generates, including starts an
    // EXDATE removes later, so count those before the split without exdates or
    // added starts
    let generated_rule = RRule {
        text: rrule.text.clone(),
        exdates: Vec::new(),
        rdates: Vec::new(),
    };
    let occurrences_before = occurrences(&event.interval, &generated_rule, base, target, tz)?
        .into_iter()
        .filter(|interval| {
            interval
                .start_instant(tz)
                .is_ok_and(|instant| instant < target)
        })
        .count() as u32;

    if generated_start(event, rrule, target, tz)?.is_none() {
        return Err(RecurError::Invalid(
            "no occurrence starts at the chosen time".to_string(),
        ));
    }

    let rule = rrule.validate(event.interval.start())?;
    let mut truncated = rule.clone();
    let mut following = rule;

    if let Some(count) = truncated.count {
        truncated.count = Some(occurrences_before.min(count));
    } else {
        truncated.until = Some(match &previous_start {
            When::Date { date } => Until::Date(*date),
            When::Floating { datetime } => Until::Date(datetime.date()),
            When::Instant { timestamp } => Until::Instant(*timestamp),
            When::Zoned(zoned) => Until::Instant(zoned.to_instant().map_err(temporal)?),
        });
    }

    if let Some(count) = following.count {
        following.count = Some(count - occurrences_before);
    }

    let (kept_exdates, following_exdates) = partition_whens(&rrule.exdates, target, tz);
    let (kept_rdates, following_rdates) = partition_whens(&rrule.rdates, target, tz);
    let (kept_overrides, following_overrides) = partition_overrides(event, target, tz);

    Ok(Some(SeriesSplit {
        previous_start,
        occurrences_before,
        truncated: RRule {
            text: truncated.to_rrule(),
            exdates: kept_exdates,
            rdates: kept_rdates,
        },
        following: RRule {
            text: following.to_rrule(),
            exdates: following_exdates,
            rdates: following_rdates,
        },
        kept_overrides,
        following_overrides,
    }))
}

/// The generated interval that starts exactly at `original`, found by expanding
/// a one-second probe window so a start removed by an EXDATE stays hidden.
fn generated_start(
    event: &Event,
    rrule: &RRule,
    original: Timestamp,
    tz: &TimeZone,
) -> Result<Option<Interval>, RecurError> {
    let to = original
        .checked_add(Span::new().seconds(1))
        .map_err(|error| RecurError::Invalid(error.to_string()))?;
    for interval in occurrences(&event.interval, rrule, original, to, tz)? {
        if interval.start_instant(tz).map_err(temporal)? == original {
            return Ok(Some(interval));
        }
    }
    Ok(None)
}

/// Split stored starts at the target instant; an unresolvable value stays with
/// the truncated series.
fn partition_whens(values: &[When], target: Timestamp, tz: &TimeZone) -> (Vec<When>, Vec<When>) {
    let mut kept = Vec::new();
    let mut following = Vec::new();
    for value in values {
        match value.start_instant(tz) {
            Ok(instant) if instant < target => kept.push(value.clone()),
            Ok(_) => following.push(value.clone()),
            Err(_) => kept.push(value.clone()),
        }
    }
    (kept, following)
}

/// Split overrides at the target instant; an unresolvable start stays with the
/// truncated series.
fn partition_overrides(
    event: &Event,
    target: Timestamp,
    tz: &TimeZone,
) -> (Vec<EventOverride>, Vec<EventOverride>) {
    let mut kept = Vec::new();
    let mut following = Vec::new();
    for overridden in &event.overrides {
        match overridden.original_start.start_instant(tz) {
            Ok(instant) if instant < target => kept.push(overridden.clone()),
            Ok(_) => following.push(overridden.clone()),
            Err(_) => kept.push(overridden.clone()),
        }
    }
    (kept, following)
}

/// Sort by the start actually shown, then by title.
fn sort_by_start(
    occurrences: Vec<EffectiveOccurrence>,
    tz: &TimeZone,
) -> Result<Vec<EffectiveOccurrence>, RecurError> {
    let mut keyed = Vec::with_capacity(occurrences.len());
    for occurrence in occurrences {
        let start = occurrence.interval.start_instant(tz).map_err(temporal)?;
        keyed.push((start, occurrence.title.clone(), occurrence));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    Ok(keyed
        .into_iter()
        .map(|(_, _, occurrence)| occurrence)
        .collect())
}

/// Map a temporal failure to the recurrence error a window read reports.
fn temporal(error: TemporalError) -> RecurError {
    RecurError::Invalid(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::{DateTime, date};

    fn event(interval: Interval, recurrence: Option<RRule>) -> Event {
        Event {
            id: uuid::Uuid::nil(),
            project_id: uuid::Uuid::nil(),
            title: "series".to_string(),
            description: None,
            interval,
            recurrence,
            overrides: Vec::new(),
            external_metadata: None,
            created_at: Timestamp::from_second(0).unwrap(),
            updated_at: Timestamp::from_second(0).unwrap(),
        }
    }

    fn all_day(start: jiff::civil::Date, end: jiff::civil::Date) -> Interval {
        Interval::new(When::Date { date: start }, When::Date { date: end }).unwrap()
    }

    fn floating(start: DateTime, end: DateTime) -> Interval {
        Interval::new(
            When::Floating { datetime: start },
            When::Floating { datetime: end },
        )
        .unwrap()
    }

    fn instant(value: &str) -> Timestamp {
        value.parse().unwrap()
    }

    fn rule(text: &str) -> RRule {
        RRule {
            text: text.to_owned(),
            exdates: Vec::new(),
            rdates: Vec::new(),
        }
    }

    #[test]
    fn an_override_changes_the_title_and_interval() {
        let mut series = event(
            all_day(date(2026, 11, 16), date(2026, 11, 17)),
            Some(rule("FREQ=DAILY;COUNT=3")),
        );
        let edited = all_day(date(2026, 11, 17), date(2026, 11, 19));
        series.overrides.push(EventOverride {
            original_start: When::Date {
                date: date(2026, 11, 17),
            },
            title: Some("edited".to_string()),
            description: None,
            interval: Some(edited.clone()),
        });

        let occurrences = series
            .occurrences(
                instant("2026-11-16T00:00:00Z"),
                instant("2026-11-20T00:00:00Z"),
                &TimeZone::UTC,
            )
            .unwrap();

        assert_eq!(occurrences.len(), 3);
        let edited_occurrence = &occurrences[1];
        assert_eq!(edited_occurrence.title, "edited");
        assert_eq!(edited_occurrence.interval, edited);
        assert!(edited_occurrence.overridden);
        assert!(!occurrences[0].overridden);
    }

    #[test]
    fn an_override_without_a_generated_start_is_hidden() {
        let mut series = event(
            all_day(date(2026, 11, 16), date(2026, 11, 17)),
            Some(rule("FREQ=DAILY;COUNT=2")),
        );
        // the series never generates 2026-11-20, so moving its interval into
        // the window must not invent an occurrence
        series.overrides.push(EventOverride {
            original_start: When::Date {
                date: date(2026, 11, 20),
            },
            title: Some("ghost".to_string()),
            description: None,
            interval: Some(all_day(date(2026, 11, 18), date(2026, 11, 19))),
        });

        let occurrences = series
            .occurrences(
                instant("2026-11-16T00:00:00Z"),
                instant("2026-11-20T00:00:00Z"),
                &TimeZone::UTC,
            )
            .unwrap();

        assert_eq!(occurrences.len(), 2);
        assert!(occurrences.iter().all(|occurrence| !occurrence.overridden));
    }

    #[test]
    fn an_override_moved_into_the_window_is_included() {
        let mut series = event(
            floating(
                date(2026, 11, 16).at(9, 0, 0, 0),
                date(2026, 11, 16).at(10, 0, 0, 0),
            ),
            Some(rule("FREQ=DAILY;COUNT=10")),
        );
        // the generated start on 2026-11-20 is outside the read window, but the
        // edited interval lands inside it
        series.overrides.push(EventOverride {
            original_start: When::Floating {
                datetime: date(2026, 11, 20).at(9, 0, 0, 0),
            },
            title: Some("moved".to_string()),
            description: None,
            interval: Some(floating(
                date(2026, 11, 17).at(14, 0, 0, 0),
                date(2026, 11, 17).at(15, 0, 0, 0),
            )),
        });

        let occurrences = series
            .occurrences(
                instant("2026-11-17T00:00:00Z"),
                instant("2026-11-18T00:00:00Z"),
                &TimeZone::UTC,
            )
            .unwrap();

        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].title, "series");
        assert_eq!(occurrences[1].title, "moved");
        assert!(occurrences[1].overridden);
    }

    #[test]
    fn split_truncates_before_the_chosen_occurrence() {
        let series = event(
            all_day(date(2026, 11, 16), date(2026, 11, 17)),
            Some(rule("FREQ=DAILY")),
        );

        let split = split_series(
            &series,
            &When::Date {
                date: date(2026, 11, 19),
            },
            &TimeZone::UTC,
        )
        .unwrap()
        .expect("split should have occurrences before it");

        assert_eq!(split.occurrences_before, 3);
        assert_eq!(
            split.previous_start,
            When::Date {
                date: date(2026, 11, 18)
            }
        );
        assert_eq!(split.truncated.text, "FREQ=DAILY;UNTIL=20261118");
        assert_eq!(split.following.text, "FREQ=DAILY");
    }

    #[test]
    fn split_counts_generated_candidates_not_excluded_ones() {
        let series = event(
            all_day(date(2026, 11, 16), date(2026, 11, 17)),
            Some(RRule {
                text: "FREQ=DAILY;COUNT=5".to_string(),
                exdates: vec![When::Date {
                    date: date(2026, 11, 17),
                }],
                rdates: Vec::new(),
            }),
        );

        let split = split_series(
            &series,
            &When::Date {
                date: date(2026, 11, 19),
            },
            &TimeZone::UTC,
        )
        .unwrap()
        .expect("split should have occurrences before it");

        // 16, 17, and 18 were generated before the split; 17 is then excluded
        assert_eq!(split.occurrences_before, 3);
        assert_eq!(split.truncated.text, "FREQ=DAILY;COUNT=3");
        assert_eq!(
            split.truncated.exdates,
            vec![When::Date {
                date: date(2026, 11, 17)
            }]
        );
        assert_eq!(split.following.text, "FREQ=DAILY;COUNT=2");
    }

    #[test]
    fn split_preserves_a_count_budget() {
        let series = event(
            all_day(date(2026, 11, 16), date(2026, 11, 17)),
            Some(rule("FREQ=DAILY;COUNT=10")),
        );

        let split = split_series(
            &series,
            &When::Date {
                date: date(2026, 11, 19),
            },
            &TimeZone::UTC,
        )
        .unwrap()
        .expect("split should have occurrences before it");

        assert_eq!(split.occurrences_before, 3);
        assert_eq!(split.truncated.text, "FREQ=DAILY;COUNT=3");
        assert_eq!(split.following.text, "FREQ=DAILY;COUNT=7");
    }

    #[test]
    fn split_moves_rdates_to_their_side_of_the_split() {
        let series = event(
            all_day(date(2026, 11, 16), date(2026, 11, 17)),
            Some(RRule {
                text: "FREQ=WEEKLY".to_string(),
                exdates: Vec::new(),
                rdates: vec![
                    When::Date {
                        date: date(2026, 11, 18),
                    },
                    When::Date {
                        date: date(2026, 11, 25),
                    },
                ],
            }),
        );

        let split = split_series(
            &series,
            &When::Date {
                date: date(2026, 11, 23),
            },
            &TimeZone::UTC,
        )
        .unwrap()
        .expect("split should have occurrences before it");

        assert_eq!(
            split.truncated.rdates,
            vec![When::Date {
                date: date(2026, 11, 18)
            }]
        );
        assert_eq!(
            split.following.rdates,
            vec![When::Date {
                date: date(2026, 11, 25)
            }]
        );
    }
}
