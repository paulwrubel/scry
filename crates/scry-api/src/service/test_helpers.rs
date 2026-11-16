use jiff::Timestamp;
use scry_core::ids::{SectionId, TaskId};
use scry_core::models::{Priority, Tags, Task, TaskState};
use scry_core::position::Position;
use uuid::Uuid;

pub(super) fn task_at(id: TaskId, section_id: Option<SectionId>, position: &str) -> Task {
    let created_at = Timestamp::now();
    Task {
        id,
        project_id: Uuid::new_v4(),
        title: "task".to_string(),
        description: None,
        priority: Priority::Normal,
        section_id,
        position: Position::parse(position).expect("valid position"),
        state: TaskState::Open,
        tags: Tags::default(),
        due: None,
        created_at,
        updated_at: created_at,
        external_identity: None,
        external_metadata: None,
    }
}
