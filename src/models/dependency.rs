use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Epic,
    Task,
}

impl DependencyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Epic => "epic",
            Self::Task => "task",
        }
    }
}

impl fmt::Display for DependencyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DependencyType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "epic" => Ok(Self::Epic),
            "task" => Ok(Self::Task),
            other => anyhow::bail!("invalid dependency type: {other}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub id: i64,
    pub blocker_type: DependencyType,
    pub blocker_id: String,
    pub blocked_type: DependencyType,
    pub blocked_id: String,
}

pub struct AddDependencyInput {
    pub blocker_type: DependencyType,
    pub blocker_id: String,
    pub blocked_type: DependencyType,
    pub blocked_id: String,
}
