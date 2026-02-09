//! JSON storage models for file-based persistence.
//!
//! These structs represent the JSON file format used in `.blueprint/` storage.
//! They exclude computed/derived fields that are inferred from directory structure
//! or calculated at runtime.

use serde::{Deserialize, Serialize};

use super::{BlueTask, Dependency, DependencyType, Epic, ItemStatus, Project, ProjectStatus};

/// Project metadata stored in `.blueprint/project.json`.
///
/// This is identical to the runtime `Project` struct since projects
/// have no computed fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectJson {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: ProjectStatus,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProjectJson> for Project {
    fn from(json: ProjectJson) -> Self {
        Self {
            id: json.id,
            name: json.name,
            description: json.description,
            status: json.status,
            created_at: json.created_at,
            updated_at: json.updated_at,
        }
    }
}

impl From<Project> for ProjectJson {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            name: project.name,
            description: project.description,
            status: project.status,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}

/// Epic stored in `.blueprint/epics/E{n}.json`.
///
/// Excludes:
/// - `project_id`: derived from directory (single project per .blueprint/)
/// - `short_id`: derived from filename (E1.json → "E1")
/// - `task_count`: computed at runtime from tasks directory
/// - `done_count`: computed at runtime from tasks directory
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpicJson {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: ItemStatus,
    pub created_at: String,
    pub updated_at: String,
}

impl EpicJson {
    /// Convert to runtime Epic model by hydrating derived fields.
    ///
    /// # Arguments
    /// - `project_id`: The project this epic belongs to
    /// - `short_id`: Derived from filename (e.g., "E1" from "E1.json")
    /// - `task_count`: Number of tasks in this epic
    /// - `done_count`: Number of completed tasks in this epic
    pub fn into_epic(
        self,
        project_id: String,
        short_id: String,
        task_count: i64,
        done_count: i64,
    ) -> Epic {
        Epic {
            id: self.id,
            project_id,
            title: self.title,
            description: self.description,
            status: self.status,
            short_id: Some(short_id),
            created_at: self.created_at,
            updated_at: self.updated_at,
            task_count,
            done_count,
        }
    }
}

impl From<Epic> for EpicJson {
    fn from(epic: Epic) -> Self {
        Self {
            id: epic.id,
            title: epic.title,
            description: epic.description,
            status: epic.status,
            created_at: epic.created_at,
            updated_at: epic.updated_at,
        }
    }
}

/// Task stored in `.blueprint/tasks/E{n}-T{m}.json`.
///
/// Excludes:
/// - `epic_id`: derived from filename prefix (E1-T2.json → epic "E1")
/// - `short_id`: derived from filename (E1-T2.json → "E1-T2")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskJson {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: ItemStatus,
    pub session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl TaskJson {
    /// Convert to runtime BlueTask model by hydrating derived fields.
    ///
    /// # Arguments
    /// - `epic_id`: The ULID of the epic this task belongs to
    /// - `short_id`: Derived from filename (e.g., "E1-T2" from "E1-T2.json")
    pub fn into_task(self, epic_id: String, short_id: String) -> BlueTask {
        BlueTask {
            id: self.id,
            epic_id,
            title: self.title,
            description: self.description,
            status: self.status,
            short_id: Some(short_id),
            session_id: self.session_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl From<BlueTask> for TaskJson {
    fn from(task: BlueTask) -> Self {
        Self {
            id: task.id,
            title: task.title,
            description: task.description,
            status: task.status,
            session_id: task.session_id,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }
}

/// A single dependency entry using human-readable short IDs.
///
/// Both `blocker` and `blocked` use short IDs like "E1" or "E1-T2".
/// The type (epic vs task) is inferred from the format:
/// - Epic: "E{n}" (e.g., "E1", "E2")
/// - Task: "E{n}-T{m}" (e.g., "E1-T1", "E2-T3")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEntry {
    pub blocker: String,
    pub blocked: String,
}

impl DependencyEntry {
    /// Parse a short ID to determine its type.
    ///
    /// Returns `DependencyType::Task` if the ID contains "-T",
    /// otherwise returns `DependencyType::Epic`.
    pub fn parse_type(short_id: &str) -> DependencyType {
        if short_id.contains("-T") {
            DependencyType::Task
        } else {
            DependencyType::Epic
        }
    }

    /// Convert to runtime Dependency model.
    ///
    /// # Arguments
    /// - `id`: The database ID for this dependency
    /// - `resolve_id`: A function that resolves short IDs to ULIDs
    ///
    /// # Returns
    /// `None` if either short ID cannot be resolved.
    pub fn to_dependency<F>(&self, id: i64, resolve_id: F) -> Option<Dependency>
    where
        F: Fn(&str) -> Option<String>,
    {
        let blocker_id = resolve_id(&self.blocker)?;
        let blocked_id = resolve_id(&self.blocked)?;

        Some(Dependency {
            id,
            blocker_type: Self::parse_type(&self.blocker),
            blocker_id,
            blocked_type: Self::parse_type(&self.blocked),
            blocked_id,
        })
    }

    /// Create from a runtime Dependency using a resolver for short IDs.
    ///
    /// # Arguments
    /// - `dep`: The runtime dependency
    /// - `resolve_short_id`: A function that resolves ULIDs to short IDs
    ///
    /// # Returns
    /// `None` if either ULID cannot be resolved to a short ID.
    pub fn from_dependency<F>(dep: &Dependency, resolve_short_id: F) -> Option<Self>
    where
        F: Fn(&DependencyType, &str) -> Option<String>,
    {
        let blocker = resolve_short_id(&dep.blocker_type, &dep.blocker_id)?;
        let blocked = resolve_short_id(&dep.blocked_type, &dep.blocked_id)?;

        Some(Self { blocker, blocked })
    }
}

/// Dependencies file stored in `.blueprint/dependencies.json`.
///
/// Contains a version number for future schema evolution and a list
/// of all dependencies between epics and tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependenciesFile {
    pub version: u32,
    pub dependencies: Vec<DependencyEntry>,
}

impl Default for DependenciesFile {
    fn default() -> Self {
        Self {
            version: 1,
            dependencies: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_json_serialization() {
        let project = ProjectJson {
            id: "01KH0DPG02JCPAK1Y87Q4NKRMW".to_string(),
            name: "Test Project".to_string(),
            description: "A test project".to_string(),
            status: ProjectStatus::Active,
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
        };

        let json = serde_json::to_string_pretty(&project).unwrap();
        assert!(json.contains("\"id\": \"01KH0DPG02JCPAK1Y87Q4NKRMW\""));
        assert!(json.contains("\"status\": \"active\""));
    }

    #[test]
    fn test_project_json_deserialization() {
        let json = r#"{
            "id": "01KH0DPG02JCPAK1Y87Q4NKRMW",
            "name": "Test Project",
            "description": "A test project",
            "status": "active",
            "created_at": "2025-02-09T14:00:00Z",
            "updated_at": "2025-02-09T14:00:00Z"
        }"#;

        let project: ProjectJson = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "01KH0DPG02JCPAK1Y87Q4NKRMW");
        assert_eq!(project.status, ProjectStatus::Active);
    }

    #[test]
    fn test_project_json_conversion() {
        let json = ProjectJson {
            id: "01KH0DPG02JCPAK1Y87Q4NKRMW".to_string(),
            name: "Test".to_string(),
            description: "Desc".to_string(),
            status: ProjectStatus::Active,
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
        };

        let project: Project = json.clone().into();
        let back: ProjectJson = project.into();

        assert_eq!(json, back);
    }

    #[test]
    fn test_epic_json_serialization() {
        let epic = EpicJson {
            id: "01KH0DPG02JCPAK1Y87Q4NKRMX".to_string(),
            title: "Test Epic".to_string(),
            description: "An epic description".to_string(),
            status: ItemStatus::InProgress,
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
        };

        let json = serde_json::to_string_pretty(&epic).unwrap();
        assert!(json.contains("\"status\": \"in_progress\""));
        // Should NOT contain project_id, short_id, task_count, done_count
        assert!(!json.contains("project_id"));
        assert!(!json.contains("short_id"));
        assert!(!json.contains("task_count"));
        assert!(!json.contains("done_count"));
    }

    #[test]
    fn test_epic_json_deserialization() {
        let json = r#"{
            "id": "01KH0DPG02JCPAK1Y87Q4NKRMX",
            "title": "Test Epic",
            "description": "Epic desc",
            "status": "todo",
            "created_at": "2025-02-09T14:00:00Z",
            "updated_at": "2025-02-09T14:00:00Z"
        }"#;

        let epic: EpicJson = serde_json::from_str(json).unwrap();
        assert_eq!(epic.title, "Test Epic");
        assert_eq!(epic.status, ItemStatus::Todo);
    }

    #[test]
    fn test_epic_json_hydration() {
        let json = EpicJson {
            id: "01KH0DPG02JCPAK1Y87Q4NKRMX".to_string(),
            title: "Epic".to_string(),
            description: "Desc".to_string(),
            status: ItemStatus::Done,
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
        };

        let epic = json.into_epic(
            "proj-123".to_string(),
            "E1".to_string(),
            5,  // task_count
            3,  // done_count
        );

        assert_eq!(epic.project_id, "proj-123");
        assert_eq!(epic.short_id, Some("E1".to_string()));
        assert_eq!(epic.task_count, 5);
        assert_eq!(epic.done_count, 3);
    }

    #[test]
    fn test_task_json_serialization() {
        let task = TaskJson {
            id: "01KH0DPG02JCPAK1Y87Q4NKRMY".to_string(),
            title: "Test Task".to_string(),
            description: "Task description".to_string(),
            status: ItemStatus::Todo,
            session_id: None,
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
        };

        let json = serde_json::to_string_pretty(&task).unwrap();
        assert!(json.contains("\"session_id\": null"));
        // Should NOT contain epic_id, short_id
        assert!(!json.contains("epic_id"));
        assert!(!json.contains("short_id"));
    }

    #[test]
    fn test_task_json_with_session() {
        let task = TaskJson {
            id: "01KH0DPG02JCPAK1Y87Q4NKRMY".to_string(),
            title: "Active Task".to_string(),
            description: "Being worked on".to_string(),
            status: ItemStatus::InProgress,
            session_id: Some("session-abc".to_string()),
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&task).unwrap();
        let back: TaskJson = serde_json::from_str(&json).unwrap();

        assert_eq!(back.session_id, Some("session-abc".to_string()));
    }

    #[test]
    fn test_task_json_hydration() {
        let json = TaskJson {
            id: "01KH0DPG02JCPAK1Y87Q4NKRMY".to_string(),
            title: "Task".to_string(),
            description: "Desc".to_string(),
            status: ItemStatus::Todo,
            session_id: None,
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
        };

        let task = json.into_task("epic-ulid".to_string(), "E1-T2".to_string());

        assert_eq!(task.epic_id, "epic-ulid");
        assert_eq!(task.short_id, Some("E1-T2".to_string()));
    }

    #[test]
    fn test_dependency_entry_type_parsing() {
        assert_eq!(DependencyEntry::parse_type("E1"), DependencyType::Epic);
        assert_eq!(DependencyEntry::parse_type("E2"), DependencyType::Epic);
        assert_eq!(DependencyEntry::parse_type("E10"), DependencyType::Epic);
        assert_eq!(DependencyEntry::parse_type("E1-T1"), DependencyType::Task);
        assert_eq!(DependencyEntry::parse_type("E2-T5"), DependencyType::Task);
        assert_eq!(DependencyEntry::parse_type("E10-T20"), DependencyType::Task);
    }

    #[test]
    fn test_dependencies_file_serialization() {
        let deps = DependenciesFile {
            version: 1,
            dependencies: vec![
                DependencyEntry {
                    blocker: "E1-T1".to_string(),
                    blocked: "E1-T2".to_string(),
                },
                DependencyEntry {
                    blocker: "E1".to_string(),
                    blocked: "E2".to_string(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&deps).unwrap();
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"blocker\": \"E1-T1\""));
        assert!(json.contains("\"blocked\": \"E2\""));
    }

    #[test]
    fn test_dependencies_file_default() {
        let deps = DependenciesFile::default();
        assert_eq!(deps.version, 1);
        assert!(deps.dependencies.is_empty());
    }

    #[test]
    fn test_dependency_entry_to_dependency() {
        let entry = DependencyEntry {
            blocker: "E1-T1".to_string(),
            blocked: "E1-T2".to_string(),
        };

        // Mock resolver that returns predictable ULIDs
        let resolve = |short_id: &str| -> Option<String> {
            match short_id {
                "E1-T1" => Some("ulid-task-1".to_string()),
                "E1-T2" => Some("ulid-task-2".to_string()),
                _ => None,
            }
        };

        let dep = entry.to_dependency(42, resolve).unwrap();
        assert_eq!(dep.id, 42);
        assert_eq!(dep.blocker_type, DependencyType::Task);
        assert_eq!(dep.blocker_id, "ulid-task-1");
        assert_eq!(dep.blocked_type, DependencyType::Task);
        assert_eq!(dep.blocked_id, "ulid-task-2");
    }

    #[test]
    fn test_dependency_entry_from_dependency() {
        let dep = Dependency {
            id: 1,
            blocker_type: DependencyType::Epic,
            blocker_id: "ulid-epic-1".to_string(),
            blocked_type: DependencyType::Task,
            blocked_id: "ulid-task-1".to_string(),
        };

        let resolve = |typ: &DependencyType, id: &str| -> Option<String> {
            match (typ, id) {
                (DependencyType::Epic, "ulid-epic-1") => Some("E1".to_string()),
                (DependencyType::Task, "ulid-task-1") => Some("E1-T1".to_string()),
                _ => None,
            }
        };

        let entry = DependencyEntry::from_dependency(&dep, resolve).unwrap();
        assert_eq!(entry.blocker, "E1");
        assert_eq!(entry.blocked, "E1-T1");
    }

    #[test]
    fn test_invalid_json_missing_required_field() {
        let json = r#"{
            "id": "01KH0DPG02JCPAK1Y87Q4NKRMW",
            "name": "Test Project",
            "status": "active",
            "created_at": "2025-02-09T14:00:00Z",
            "updated_at": "2025-02-09T14:00:00Z"
        }"#;
        // Missing "description" field

        let result: Result<ProjectJson, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_json_wrong_type() {
        let json = r#"{
            "id": "01KH0DPG02JCPAK1Y87Q4NKRMW",
            "name": "Test Project",
            "description": "Desc",
            "status": "invalid_status",
            "created_at": "2025-02-09T14:00:00Z",
            "updated_at": "2025-02-09T14:00:00Z"
        }"#;

        let result: Result<ProjectJson, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_epic_status() {
        let json = r#"{
            "id": "01KH0DPG02JCPAK1Y87Q4NKRMX",
            "title": "Epic",
            "description": "Desc",
            "status": "completed",
            "created_at": "2025-02-09T14:00:00Z",
            "updated_at": "2025-02-09T14:00:00Z"
        }"#;
        // "completed" is not a valid ItemStatus (should be "done")

        let result: Result<EpicJson, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    /// End-to-end round-trip test for all entity types.
    #[test]
    fn test_e2e_roundtrip() {
        // 1. Create runtime models
        let project = Project {
            id: "proj-001".to_string(),
            name: "Round Trip Project".to_string(),
            description: "Testing full cycle".to_string(),
            status: ProjectStatus::Active,
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
        };

        let epic = Epic {
            id: "epic-001".to_string(),
            project_id: "proj-001".to_string(),
            title: "E2E Epic".to_string(),
            description: "Epic for round trip".to_string(),
            status: ItemStatus::InProgress,
            short_id: Some("E1".to_string()),
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
            task_count: 2,
            done_count: 1,
        };

        let task = BlueTask {
            id: "task-001".to_string(),
            epic_id: "epic-001".to_string(),
            title: "E2E Task".to_string(),
            description: "Task for round trip".to_string(),
            status: ItemStatus::Done,
            short_id: Some("E1-T1".to_string()),
            session_id: Some("session-xyz".to_string()),
            created_at: "2025-02-09T14:00:00Z".to_string(),
            updated_at: "2025-02-09T14:00:00Z".to_string(),
        };

        // 2. Convert to JSON structs
        let project_json: ProjectJson = project.clone().into();
        let epic_json: EpicJson = epic.clone().into();
        let task_json: TaskJson = task.clone().into();

        // 3. Serialize to JSON strings
        let project_str = serde_json::to_string(&project_json).unwrap();
        let epic_str = serde_json::to_string(&epic_json).unwrap();
        let task_str = serde_json::to_string(&task_json).unwrap();

        // 4. Deserialize back to JSON structs
        let project_json2: ProjectJson = serde_json::from_str(&project_str).unwrap();
        let epic_json2: EpicJson = serde_json::from_str(&epic_str).unwrap();
        let task_json2: TaskJson = serde_json::from_str(&task_str).unwrap();

        // 5. Convert back to runtime models (hydrating derived fields)
        let project2: Project = project_json2.into();
        let epic2 = epic_json2.into_epic(
            epic.project_id.clone(),
            epic.short_id.clone().unwrap(),
            epic.task_count,
            epic.done_count,
        );
        let task2 = task_json2.into_task(
            task.epic_id.clone(),
            task.short_id.clone().unwrap(),
        );

        // 6. Assert equality with originals
        assert_eq!(project.id, project2.id);
        assert_eq!(project.name, project2.name);
        assert_eq!(project.description, project2.description);
        assert_eq!(project.status, project2.status);

        assert_eq!(epic.id, epic2.id);
        assert_eq!(epic.project_id, epic2.project_id);
        assert_eq!(epic.title, epic2.title);
        assert_eq!(epic.status, epic2.status);
        assert_eq!(epic.short_id, epic2.short_id);
        assert_eq!(epic.task_count, epic2.task_count);
        assert_eq!(epic.done_count, epic2.done_count);

        assert_eq!(task.id, task2.id);
        assert_eq!(task.epic_id, task2.epic_id);
        assert_eq!(task.title, task2.title);
        assert_eq!(task.status, task2.status);
        assert_eq!(task.short_id, task2.short_id);
        assert_eq!(task.session_id, task2.session_id);
    }

    /// Test round-trip for dependencies.
    #[test]
    fn test_dependencies_roundtrip() {
        let deps_file = DependenciesFile {
            version: 1,
            dependencies: vec![
                DependencyEntry {
                    blocker: "E1-T1".to_string(),
                    blocked: "E1-T2".to_string(),
                },
                DependencyEntry {
                    blocker: "E1".to_string(),
                    blocked: "E2".to_string(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&deps_file).unwrap();
        let back: DependenciesFile = serde_json::from_str(&json).unwrap();

        assert_eq!(deps_file, back);
    }
}
