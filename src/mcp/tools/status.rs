use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::db::project as project_db;
use crate::db::status as status_db;
use crate::db::Database;

use super::{optional_str, tool_error, tool_result};

pub(super) fn handle_get_status(
    args: &Value,
    db: &Database,
    default_project_id: Option<&str>,
) -> Value {
    let project_id = optional_str(args, "project_id")
        .or_else(|| default_project_id.map(String::from));

    let project_label = match &project_id {
        Some(pid) => match project_db::get_project(db, pid) {
            Ok(Some(p)) => p.name,
            Ok(None) => return tool_error(&format!("Project not found: {pid}")),
            Err(e) => {
                eprintln!("get_status error: {e:#}");
                return tool_error("Failed to get project");
            }
        },
        None => "All Projects".to_string(),
    };

    let epics_by_status = match status_db::count_epics_by_status(db, project_id.as_deref()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("get_status error: {e:#}");
            return tool_error("Failed to count epics");
        }
    };

    let tasks_by_status = match status_db::count_tasks_by_status(db, project_id.as_deref()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("get_status error: {e:#}");
            return tool_error("Failed to count tasks");
        }
    };

    let blocked_rows = match status_db::get_blocked_items(db, project_id.as_deref()) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("get_status error: {e:#}");
            return tool_error("Failed to get blocked items");
        }
    };

    // Group blocked rows by (item_type, item_id)
    let mut grouped: BTreeMap<(String, String), (String, Vec<String>)> = BTreeMap::new();
    for row in blocked_rows {
        let key = (row.item_type, row.item_id);
        grouped
            .entry(key)
            .or_insert_with(|| (row.title, Vec::new()))
            .1
            .push(row.blocker_id);
    }

    let blocked_items: Vec<Value> = grouped
        .into_iter()
        .map(|((item_type, item_id), (title, blocked_by))| {
            json!({
                "type": item_type,
                "id": item_id,
                "title": title,
                "blocked_by": blocked_by,
            })
        })
        .collect();

    let total_epics: i64 = epics_by_status.values().sum();
    let total_tasks: i64 = tasks_by_status.values().sum();

    tool_result(&json!({
        "project": project_label,
        "total_epics": total_epics,
        "epics_by_status": epics_by_status,
        "total_tasks": total_tasks,
        "tasks_by_status": tasks_by_status,
        "blocked_items": blocked_items,
    }))
}

#[cfg(test)]
mod tests {
    use super::super::dispatch_tool;
    use crate::db::dependency::add_dependency;
    use crate::db::epic::create_epic;
    use crate::db::project::create_project;
    use crate::db::task::{create_task, update_task};
    use crate::db::Database;
    use crate::models::{
        AddDependencyInput, CreateEpicInput, CreateProjectInput, CreateTaskInput, DependencyType,
        ItemStatus, UpdateTaskInput,
    };
    use serde_json::{json, Value};
    use tempfile::TempDir;

    fn test_db() -> (Database, TempDir) {
        let dir = TempDir::new().unwrap();
        let db = Database::open(&dir.path().join("test.db")).unwrap();
        db.migrate().unwrap();
        (db, dir)
    }

    fn parse_response(result: &Value) -> Value {
        let text = result["content"][0]["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn test_status_with_project_id() {
        let (db, _dir) = test_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "My Project".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E1".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let result = dispatch_tool("get_status", &json!({"project_id": project.id}), &db, None).unwrap();
        assert!(result.get("isError").is_none());

        let data = parse_response(&result);
        assert_eq!(data["project"], "My Project");
        assert_eq!(data["total_epics"], 1);
        assert_eq!(data["epics_by_status"]["todo"], 1);
        assert_eq!(data["total_tasks"], 0);
    }

    #[test]
    fn test_status_without_project_id() {
        let (db, _dir) = test_db();

        let result = dispatch_tool("get_status", &json!({}), &db, None).unwrap();
        assert!(result.get("isError").is_none());

        let data = parse_response(&result);
        assert_eq!(data["project"], "All Projects");
        assert_eq!(data["total_epics"], 0);
        assert_eq!(data["total_tasks"], 0);
    }

    #[test]
    fn test_status_project_not_found() {
        let (db, _dir) = test_db();

        let result =
            dispatch_tool("get_status", &json!({"project_id": "nonexistent"}), &db, None).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[test]
    fn test_status_empty_project() {
        let (db, _dir) = test_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Empty".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let result = dispatch_tool("get_status", &json!({"project_id": project.id}), &db, None).unwrap();
        let data = parse_response(&result);

        assert_eq!(data["total_epics"], 0);
        assert_eq!(data["epics_by_status"]["todo"], 0);
        assert_eq!(data["epics_by_status"]["in_progress"], 0);
        assert_eq!(data["epics_by_status"]["done"], 0);
        assert_eq!(data["total_tasks"], 0);
        assert!(data["blocked_items"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_status_full_lifecycle() {
        let (db, _dir) = test_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Full".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "Epic 1".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Task A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Task B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t3 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Task C".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        // t1 blocks t2, t1 blocks t3
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t2.id.clone(),
            },
        )
        .unwrap();
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t3.id.clone(),
            },
        )
        .unwrap();

        // Mark t1 as in_progress
        update_task(
            &db,
            &t1.id,
            UpdateTaskInput {
                status: Some(ItemStatus::InProgress),
                ..Default::default()
            },
        )
        .unwrap();

        let result = dispatch_tool("get_status", &json!({"project_id": project.id}), &db, None).unwrap();
        let data = parse_response(&result);

        assert_eq!(data["project"], "Full");
        assert_eq!(data["total_epics"], 1);
        assert_eq!(data["epics_by_status"]["todo"], 1);
        assert_eq!(data["total_tasks"], 3);
        assert_eq!(data["tasks_by_status"]["todo"], 2);
        assert_eq!(data["tasks_by_status"]["in_progress"], 1);
        assert_eq!(data["tasks_by_status"]["done"], 0);

        let blocked = data["blocked_items"].as_array().unwrap();
        assert_eq!(blocked.len(), 2);

        // Both t2 and t3 should be blocked by t1
        for item in blocked {
            assert_eq!(item["type"], "task");
            assert_eq!(item["blocked_by"].as_array().unwrap().len(), 1);
            assert_eq!(item["blocked_by"][0], t1.id);
        }
    }
}
