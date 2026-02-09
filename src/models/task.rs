use serde::{Deserialize, Serialize};

use super::ItemStatus;

#[derive(Debug, Clone, Serialize)]
pub struct TaskSummary {
    pub id: String,
    pub short_id: Option<String>,
    pub epic_id: String,
    pub title: String,
    pub status: ItemStatus,
    pub blockers: Vec<String>,
}

impl TaskSummary {
    pub fn from_task(task: BlueTask, blockers: Vec<String>) -> Self {
        Self {
            id: task.id,
            short_id: task.short_id,
            epic_id: task.epic_id,
            title: task.title,
            status: task.status,
            blockers,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueTask {
    pub id: String,
    pub epic_id: String,
    pub title: String,
    pub description: String,
    pub status: ItemStatus,
    pub short_id: Option<String>,
    pub session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct CreateTaskInput {
    pub epic_id: String,
    pub title: String,
    pub description: String,
    pub session_id: Option<String>,
}

#[derive(Default)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<ItemStatus>,
    pub session_id: Option<Option<String>>,
}
