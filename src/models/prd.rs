use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prd {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
}

pub struct CreatePrdInput {
    pub project_id: String,
    pub title: String,
    pub content: String,
}
