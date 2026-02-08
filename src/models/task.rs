use serde::{Deserialize, Serialize};

use super::ItemStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueTask {
    pub id: String,
    pub epic_id: String,
    pub title: String,
    pub description: String,
    pub status: ItemStatus,
    pub short_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct CreateTaskInput {
    pub epic_id: String,
    pub title: String,
    pub description: String,
}

#[derive(Default)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<ItemStatus>,
}
