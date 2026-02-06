use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Todo,
    InProgress,
    Done,
}

impl ItemStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
        }
    }
}

impl fmt::Display for ItemStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ItemStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "todo" => Ok(Self::Todo),
            "in_progress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            other => anyhow::bail!("invalid item status: {other}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Epic {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub status: ItemStatus,
    pub created_at: String,
    pub updated_at: String,
    pub task_count: i64,
}

pub struct CreateEpicInput {
    pub project_id: String,
    pub title: String,
    pub description: String,
}

#[derive(Default)]
pub struct UpdateEpicInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<ItemStatus>,
}
