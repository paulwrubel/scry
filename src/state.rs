use chrono::DateTime;
use chrono::Utc;
use scry_core::models::{
    Lifecycle, Note, Priority, Project, ProjectId, Section, SectionId, Tags, Task, TaskId,
    TaskSortingMode,
};
use scry_core::store::ProjectSnapshot;
use serde::Serialize;

// Aug 9, 2026 at 4:40pm
pub const DATETIME_FORMAT_STR: &str = "%b %-d, %Y at %-I:%M%P";

#[derive(Debug, Clone)]
pub struct ProjectState {
    project: Project,
    pub(crate) sections_with_tasks: Vec<SectionWithTasks>,
    pub(crate) closed_tasks: Vec<TaskWithNotes>,
}

#[derive(Debug, Clone)]
pub struct SectionWithTasks {
    // None represents the No section group
    pub(crate) section: Option<Section>,
    pub(crate) tasks_with_notes: Vec<TaskWithNotes>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskWithNotes {
    pub(crate) id: TaskId,
    pub(crate) project_id: ProjectId,
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) priority: Priority,
    pub(crate) section_id: Option<SectionId>,
    pub(crate) lifecycle: Lifecycle,
    pub(crate) position: i32,
    pub(crate) tags: Tags,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) notes: Vec<Note>,
}

impl TaskWithNotes {
    pub fn new(task: &Task, notes: impl IntoIterator<Item = Note>) -> Self {
        Self {
            id: task.id,
            project_id: task.project_id,
            title: task.title.clone(),
            description: task.description.clone(),
            priority: task.priority,
            section_id: task.section_id,
            lifecycle: task.lifecycle.clone(),
            position: task.position,
            tags: task.tags.clone(),
            created_at: task.created_at,
            notes: notes.into_iter().collect(),
        }
    }
}

impl From<TaskWithNotes> for Task {
    fn from(value: TaskWithNotes) -> Self {
        (&value).into()
    }
}

impl From<&TaskWithNotes> for Task {
    fn from(value: &TaskWithNotes) -> Self {
        Self {
            id: value.id,
            project_id: value.project_id,
            title: value.title.clone(),
            description: value.description.clone(),
            priority: value.priority,
            section_id: value.section_id,
            position: value.position,
            lifecycle: value.lifecycle.clone(),
            tags: value.tags.clone(),
            created_at: value.created_at,
        }
    }
}

impl ProjectState {
    pub fn from_snapshot(snapshot: ProjectSnapshot) -> Self {
        let ProjectSnapshot {
            project,
            sections,
            tasks,
            notes,
        } = snapshot;

        let task_with_notes = |task: &Task| {
            TaskWithNotes::new(
                task,
                notes.iter().filter(|note| note.task_id == task.id).cloned(),
            )
        };

        let mut sections_with_tasks: Vec<SectionWithTasks> = sections
            .iter()
            .map(|section| {
                let mut tasks_with_notes: Vec<_> = tasks
                    .iter()
                    .filter(|task| {
                        !task.lifecycle.is_closed() && task.section_id == Some(section.id)
                    })
                    .map(task_with_notes)
                    .collect();
                sort_open_tasks(&mut tasks_with_notes, project.task_sorting_mode);
                SectionWithTasks {
                    section: Some(section.clone()),
                    tasks_with_notes,
                }
            })
            .collect();

        let mut unsectioned: Vec<_> = tasks
            .iter()
            .filter(|task| !task.lifecycle.is_closed() && task.section_id.is_none())
            .map(task_with_notes)
            .collect();
        sort_open_tasks(&mut unsectioned, project.task_sorting_mode);
        if !unsectioned.is_empty() {
            sections_with_tasks.push(SectionWithTasks {
                section: None,
                tasks_with_notes: unsectioned,
            });
        }

        let mut closed_tasks: Vec<_> = tasks
            .iter()
            .filter(|task| task.lifecycle.is_closed())
            .map(task_with_notes)
            .collect();
        closed_tasks.sort_by(|a, b| {
            b.lifecycle
                .closed_at()
                .cmp(&a.lifecycle.closed_at())
                .then_with(|| b.id.cmp(&a.id))
        });

        Self {
            project,
            sections_with_tasks,
            closed_tasks,
        }
    }

    pub fn with_substring_filter(self, filter: String) -> Self {
        let filter = filter.to_lowercase();
        let matches = |task: &TaskWithNotes| {
            task.title.to_lowercase().contains(&filter)
                || task
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&filter))
        };

        let sections_with_tasks: Vec<SectionWithTasks> = self
            .sections_with_tasks
            .into_iter()
            .filter_map(|swt| {
                let tasks_with_notes: Vec<TaskWithNotes> =
                    swt.tasks_with_notes.into_iter().filter(matches).collect();

                // drop the No section group once filtering empties it, but keep
                // real sections even when they end up empty
                if swt.section.is_none() && tasks_with_notes.is_empty() {
                    None
                } else {
                    Some(SectionWithTasks {
                        section: swt.section,
                        tasks_with_notes,
                    })
                }
            })
            .collect();

        let closed_tasks: Vec<TaskWithNotes> =
            self.closed_tasks.into_iter().filter(matches).collect();

        Self {
            project: self.project,
            sections_with_tasks,
            closed_tasks,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn get_task_by_id(&self, task_id: TaskId) -> Option<&TaskWithNotes> {
        self.tasks().find(|t| t.id == task_id)
    }

    pub fn get_section_by_id(&self, section_id: SectionId) -> Option<&Section> {
        self.sections().find(|s| s.id == section_id)
    }

    pub fn tasks(&self) -> impl DoubleEndedIterator<Item = &TaskWithNotes> + '_ {
        // flatten into a single ordered stream of open tasks, then closed tasks
        self.sections_with_tasks
            .iter()
            .flat_map(|swt| swt.tasks_with_notes.iter())
            .chain(self.closed_tasks.iter())
    }

    pub fn sections(&self) -> impl DoubleEndedIterator<Item = &Section> + '_ {
        self.sections_with_tasks
            .iter()
            .filter_map(|swt| swt.section.as_ref())
    }

    pub fn section_groups(&self) -> impl DoubleEndedIterator<Item = &SectionWithTasks> + '_ {
        self.sections_with_tasks.iter()
    }

    pub fn closed_tasks(&self) -> &[TaskWithNotes] {
        &self.closed_tasks
    }
}

fn sort_open_tasks(tasks: &mut [TaskWithNotes], mode: TaskSortingMode) {
    tasks.sort_by(|a, b| {
        let ordering = match mode {
            TaskSortingMode::Alphabetical => a.title.cmp(&b.title),
            TaskSortingMode::AlphabeticalCaseInsensitive => {
                a.title.to_lowercase().cmp(&b.title.to_lowercase())
            }
            TaskSortingMode::Id => a.id.cmp(&b.id),
            TaskSortingMode::Manual => a.position.cmp(&b.position),
            TaskSortingMode::Priority => a.priority.cmp(&b.priority),
        };
        // tie-break by id so every mode is deterministic
        ordering.then_with(|| a.id.cmp(&b.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use scry_core::models::CloseReason;

    fn ts(rfc3339: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc3339)
            .expect("valid timestamp")
            .with_timezone(&Utc)
    }

    fn project(mode: TaskSortingMode) -> Project {
        Project {
            id: 1,
            name: "Test Project".to_string(),
            description: None,
            task_sorting_mode: mode,
            show_priority: false,
            created_at: ts("2026-01-01T00:00:00Z"),
        }
    }

    fn section(id: SectionId, position: i32) -> Section {
        Section {
            id,
            project_id: 1,
            name: format!("Section {id}"),
            position,
            color: None,
            created_at: ts("2026-01-01T00:00:00Z"),
        }
    }

    fn task(
        id: TaskId,
        title: &str,
        section_id: Option<SectionId>,
        position: i32,
        priority: Priority,
        lifecycle: Lifecycle,
    ) -> Task {
        Task {
            id,
            project_id: 1,
            title: title.to_string(),
            description: None,
            priority,
            section_id,
            position,
            lifecycle,
            tags: Tags::default(),
            created_at: ts("2026-01-01T00:00:00Z"),
        }
    }

    fn open_task(
        id: TaskId,
        title: &str,
        section_id: Option<SectionId>,
        position: i32,
        priority: Priority,
    ) -> Task {
        task(id, title, section_id, position, priority, Lifecycle::Open)
    }

    fn closed_task(
        id: TaskId,
        title: &str,
        section_id: Option<SectionId>,
        closed_at: &str,
        reason: CloseReason,
    ) -> Task {
        task(
            id,
            title,
            section_id,
            0,
            Priority::Medium,
            Lifecycle::Closed {
                closed_at: ts(closed_at),
                reason,
            },
        )
    }

    fn group_ids(state: &ProjectState) -> Vec<(Option<SectionId>, Vec<TaskId>)> {
        state
            .sections_with_tasks
            .iter()
            .map(|group| {
                let ids = group.tasks_with_notes.iter().map(|task| task.id).collect();
                (group.section.as_ref().map(|section| section.id), ids)
            })
            .collect()
    }

    fn section_open_ids(state: &ProjectState, section_id: Option<SectionId>) -> Vec<TaskId> {
        state
            .section_groups()
            .find(|group| group.section.as_ref().map(|section| section.id) == section_id)
            .map(|group| group.tasks_with_notes.iter().map(|task| task.id).collect())
            .unwrap_or_default()
    }

    #[test]
    fn groups_open_tasks_by_section_and_keeps_empty_sections() {
        let snapshot = ProjectSnapshot {
            project: project(TaskSortingMode::Manual),
            sections: vec![section(10, 0), section(11, 1)],
            tasks: vec![
                open_task(1, "second", Some(10), 1, Priority::Medium),
                open_task(2, "first", Some(10), 0, Priority::Medium),
                open_task(3, "unsectioned", None, 0, Priority::Medium),
                open_task(4, "also unsectioned", None, 1, Priority::Medium),
            ],
            notes: vec![Note {
                id: 100,
                task_id: 2,
                contents: "belongs to first".to_string(),
                created_at: ts("2026-01-02T00:00:00Z"),
            }],
        };

        let state = ProjectState::from_snapshot(snapshot);

        assert_eq!(
            group_ids(&state),
            vec![
                (Some(10), vec![2, 1]),
                (Some(11), vec![]),
                (None, vec![3, 4]),
            ]
        );

        let first = state.get_task_by_id(2).expect("task 2 exists");
        assert_eq!(first.notes.len(), 1);
        assert_eq!(first.notes[0].contents, "belongs to first");
        assert!(
            state
                .get_task_by_id(1)
                .expect("task 1 exists")
                .notes
                .is_empty()
        );
    }

    #[test]
    fn no_section_group_is_only_present_for_open_unsectioned_tasks() {
        let snapshot = ProjectSnapshot {
            project: project(TaskSortingMode::Manual),
            sections: vec![section(10, 0)],
            tasks: vec![
                open_task(1, "sectioned", Some(10), 0, Priority::Medium),
                closed_task(
                    2,
                    "closed unsectioned",
                    None,
                    "2026-01-02T00:00:00Z",
                    CloseReason::Completed,
                ),
            ],
            notes: vec![],
        };

        let state = ProjectState::from_snapshot(snapshot);

        assert_eq!(group_ids(&state), vec![(Some(10), vec![1])]);
        assert!(state.section_groups().all(|group| group.section.is_some()));
    }

    #[test]
    fn closed_tasks_are_kept_out_of_sections_and_sorted_by_closed_at_then_id() {
        let snapshot = ProjectSnapshot {
            project: project(TaskSortingMode::Manual),
            sections: vec![section(10, 0)],
            tasks: vec![
                open_task(1, "open", Some(10), 0, Priority::Medium),
                closed_task(
                    3,
                    "closed early",
                    Some(10),
                    "2026-01-01T00:00:00Z",
                    CloseReason::Completed,
                ),
                closed_task(
                    4,
                    "closed late a",
                    Some(10),
                    "2026-01-02T00:00:00Z",
                    CloseReason::Cancelled,
                ),
                closed_task(
                    5,
                    "closed late b",
                    None,
                    "2026-01-02T00:00:00Z",
                    CloseReason::Completed,
                ),
            ],
            notes: vec![],
        };

        let state = ProjectState::from_snapshot(snapshot);

        assert_eq!(group_ids(&state), vec![(Some(10), vec![1])]);
        assert_eq!(
            state
                .closed_tasks()
                .iter()
                .map(|task| task.id)
                .collect::<Vec<_>>(),
            vec![5, 4, 3]
        );
    }

    fn sorting_snapshot(mode: TaskSortingMode) -> ProjectSnapshot {
        ProjectSnapshot {
            project: project(mode),
            sections: vec![section(10, 0)],
            tasks: vec![
                open_task(1, "beta", Some(10), 2, Priority::High),
                open_task(2, "Alpha", Some(10), 1, Priority::Critical),
                open_task(3, "alpha", Some(10), 1, Priority::High),
            ],
            notes: vec![],
        }
    }

    #[test]
    fn manual_sorting_orders_by_position_then_id() {
        let state = ProjectState::from_snapshot(sorting_snapshot(TaskSortingMode::Manual));

        assert_eq!(section_open_ids(&state, Some(10)), vec![2, 3, 1]);
    }

    #[test]
    fn priority_sorting_orders_by_priority_then_id() {
        let state = ProjectState::from_snapshot(sorting_snapshot(TaskSortingMode::Priority));

        assert_eq!(section_open_ids(&state, Some(10)), vec![2, 1, 3]);
    }

    #[test]
    fn case_insensitive_alphabetical_sorting_breaks_ties_by_id() {
        let state = ProjectState::from_snapshot(sorting_snapshot(
            TaskSortingMode::AlphabeticalCaseInsensitive,
        ));

        assert_eq!(section_open_ids(&state, Some(10)), vec![2, 3, 1]);
    }

    #[test]
    fn substring_filter_matches_titles_and_tags_case_insensitively() {
        let mut tagged = open_task(2, "other", Some(10), 1, Priority::Medium);
        tagged.tags = Tags::from("Urgent,backend");

        let snapshot = ProjectSnapshot {
            project: project(TaskSortingMode::Manual),
            sections: vec![section(10, 0), section(11, 1)],
            tasks: vec![
                open_task(1, "Fix LOGIN", Some(10), 0, Priority::Medium),
                tagged,
                open_task(3, "read docs", None, 0, Priority::Medium),
            ],
            notes: vec![],
        };

        let by_title = ProjectState::from_snapshot(snapshot.clone())
            .with_substring_filter("login".to_string());
        assert_eq!(
            group_ids(&by_title),
            vec![(Some(10), vec![1]), (Some(11), vec![])]
        );

        let by_tag =
            ProjectState::from_snapshot(snapshot).with_substring_filter("Urgent".to_string());
        assert_eq!(
            group_ids(&by_tag),
            vec![(Some(10), vec![2]), (Some(11), vec![])]
        );
    }

    #[test]
    fn get_task_by_id_finds_open_and_closed_tasks() {
        let snapshot = ProjectSnapshot {
            project: project(TaskSortingMode::Manual),
            sections: vec![section(10, 0)],
            tasks: vec![
                open_task(1, "open task", Some(10), 0, Priority::Medium),
                closed_task(
                    2,
                    "closed task",
                    Some(10),
                    "2026-01-02T00:00:00Z",
                    CloseReason::Completed,
                ),
            ],
            notes: vec![],
        };

        let state = ProjectState::from_snapshot(snapshot);

        assert_eq!(
            state.get_task_by_id(1).map(|task| task.title.as_str()),
            Some("open task")
        );
        assert_eq!(
            state.get_task_by_id(2).map(|task| task.title.as_str()),
            Some("closed task")
        );
        assert!(state.get_task_by_id(3).is_none());
    }
}
