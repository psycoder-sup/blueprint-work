use serde_json::{json, Value};

use crate::db::dependency as dep_db;
use crate::db::epic as epic_db;
use crate::db::task as task_db;
use crate::db::Database;
use crate::models::dependency::DependencyType;
use crate::models::epic::ItemStatus;
use crate::models::task::{CreateTaskInput, UpdateTaskInput};

use super::{optional_str, parse_optional_status, require_str, tool_error, tool_result};

pub(super) fn handle_create_task(args: &Value, db: &Database) -> Value {
    let epic_id = match require_str(args, "epic_id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let title = match require_str(args, "title") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let description = match require_str(args, "description") {
        Ok(v) => v,
        Err(e) => return e,
    };

    // Validate epic exists
    match epic_db::get_epic(db, &epic_id) {
        Ok(Some(_)) => {}
        Ok(None) => return tool_error(&format!("Epic not found: {epic_id}")),
        Err(e) => {
            eprintln!("create_task error: {e:#}");
            return tool_error("Failed to create task");
        }
    }

    match task_db::create_task(db, CreateTaskInput { epic_id, title, description }) {
        Ok(task) => tool_result(&task),
        Err(e) => {
            eprintln!("create_task error: {e:#}");
            tool_error("Failed to create task")
        }
    }
}

pub(super) fn handle_list_tasks(args: &Value, db: &Database) -> Value {
    let epic_id = optional_str(args, "epic_id");
    let status = match parse_optional_status::<ItemStatus>(args) {
        Ok(s) => s,
        Err(e) => return e,
    };

    match task_db::list_tasks(db, epic_id.as_deref(), status) {
        Ok(tasks) => tool_result(&tasks),
        Err(e) => {
            eprintln!("list_tasks error: {e:#}");
            tool_error("Failed to list tasks")
        }
    }
}

pub(super) fn handle_get_task(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let task = match task_db::get_task(db, &id) {
        Ok(Some(t)) => t,
        Ok(None) => return tool_error(&format!("Task not found: {id}")),
        Err(e) => {
            eprintln!("get_task error: {e:#}");
            return tool_error("Failed to get task");
        }
    };

    let blockers = match dep_db::get_blockers(db, &DependencyType::Task, &id) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("get_blockers error: {e:#}");
            return tool_error("Failed to get task");
        }
    };

    let blocks = match dep_db::get_blocked_by(db, &DependencyType::Task, &id) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("get_blocked_by error: {e:#}");
            return tool_error("Failed to get task");
        }
    };

    tool_result(&json!({ "task": task, "blockers": blockers, "blocks": blocks }))
}

pub(super) fn handle_update_task(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let status = match parse_optional_status::<ItemStatus>(args) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let input = UpdateTaskInput {
        title: optional_str(args, "title"),
        description: optional_str(args, "description"),
        status,
    };

    match task_db::update_task(db, &id, input) {
        Ok(task) => tool_result(&task),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") {
                tool_error(&format!("Task not found: {id}"))
            } else {
                eprintln!("update_task error: {e:#}");
                tool_error("Failed to update task")
            }
        }
    }
}

pub(super) fn handle_delete_task(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    match task_db::delete_task(db, &id) {
        Ok(true) => tool_result(&json!({ "deleted": true, "id": id })),
        Ok(false) => tool_error(&format!("Task not found: {id}")),
        Err(e) => {
            eprintln!("delete_task error: {e:#}");
            tool_error("Failed to delete task")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::dispatch_tool;
    use crate::db::dependency as dep_db;
    use crate::db::Database;
    use crate::models::dependency::{AddDependencyInput, DependencyType};
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

    fn create_test_project(db: &Database) -> String {
        let result = dispatch_tool(
            "create_project",
            &json!({"name": "Test Project", "description": "for task tests"}),
            db,
        )
        .unwrap();
        let project = parse_response(&result);
        project["id"].as_str().unwrap().to_string()
    }

    fn create_test_epic(db: &Database, project_id: &str) -> String {
        let result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "Test Epic", "description": "for task tests"}),
            db,
        )
        .unwrap();
        let epic = parse_response(&result);
        epic["id"].as_str().unwrap().to_string()
    }

    // --- create_task tests ---

    #[test]
    fn test_create_task_success() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let epic_id = create_test_epic(&db, &project_id);

        let result = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "My Task", "description": "desc"}),
            &db,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let task = parse_response(&result);
        assert_eq!(task["title"], "My Task");
        assert_eq!(task["description"], "desc");
        assert_eq!(task["status"], "todo");
        assert_eq!(task["epic_id"], epic_id);
        assert_eq!(task["id"].as_str().unwrap().len(), 26);
    }

    #[test]
    fn test_create_task_missing_epic_id() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "create_task",
            &json!({"title": "T", "description": "d"}),
            &db,
        )
        .unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    #[test]
    fn test_create_task_missing_title() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let epic_id = create_test_epic(&db, &project_id);
        let result = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "description": "d"}),
            &db,
        )
        .unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    #[test]
    fn test_create_task_invalid_epic_id() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "create_task",
            &json!({"epic_id": "nonexistent", "title": "T", "description": "d"}),
            &db,
        )
        .unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    // --- list_tasks tests ---

    #[test]
    fn test_list_tasks_empty() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("list_tasks", &json!({}), &db).unwrap();
        assert!(result.get("isError").is_none());
        let tasks = parse_response(&result);
        assert!(tasks.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_list_tasks_with_data() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let epic_id = create_test_epic(&db, &project_id);
        dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "T1", "description": "d"}),
            &db,
        );
        dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "T2", "description": "d"}),
            &db,
        );

        let result = dispatch_tool("list_tasks", &json!({}), &db).unwrap();
        let tasks = parse_response(&result);
        assert_eq!(tasks.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_list_tasks_epic_id_filter() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let e1 = create_test_epic(&db, &project_id);
        let e2 = create_test_epic(&db, &project_id);
        dispatch_tool(
            "create_task",
            &json!({"epic_id": e1, "title": "T1", "description": "d"}),
            &db,
        );
        dispatch_tool(
            "create_task",
            &json!({"epic_id": e2, "title": "T2", "description": "d"}),
            &db,
        );

        let result = dispatch_tool("list_tasks", &json!({"epic_id": e1}), &db).unwrap();
        let tasks = parse_response(&result);
        assert_eq!(tasks.as_array().unwrap().len(), 1);
        assert_eq!(tasks[0]["title"], "T1");
    }

    #[test]
    fn test_list_tasks_status_filter() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let epic_id = create_test_epic(&db, &project_id);

        let create_result = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "T1", "description": "d"}),
            &db,
        )
        .unwrap();
        let task = parse_response(&create_result);
        let task_id = task["id"].as_str().unwrap();

        dispatch_tool(
            "update_task",
            &json!({"id": task_id, "status": "in_progress"}),
            &db,
        );

        dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "T2", "description": "d"}),
            &db,
        );

        let result = dispatch_tool("list_tasks", &json!({"status": "in_progress"}), &db).unwrap();
        let tasks = parse_response(&result);
        assert_eq!(tasks.as_array().unwrap().len(), 1);
        assert_eq!(tasks[0]["title"], "T1");
    }

    #[test]
    fn test_list_tasks_invalid_status() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("list_tasks", &json!({"status": "bogus"}), &db).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Invalid status"));
    }

    // --- get_task tests ---

    #[test]
    fn test_get_task_success_with_dependency_info() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let epic_id = create_test_epic(&db, &project_id);

        let create_result = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "Get Me", "description": "desc"}),
            &db,
        )
        .unwrap();
        let task = parse_response(&create_result);
        let task_id = task["id"].as_str().unwrap();

        let result = dispatch_tool("get_task", &json!({"id": task_id}), &db).unwrap();
        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["task"]["title"], "Get Me");
        assert!(data["blockers"].as_array().unwrap().is_empty());
        assert!(data["blocks"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_get_task_not_found() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("get_task", &json!({"id": "nonexistent"}), &db).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[test]
    fn test_get_task_missing_id() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("get_task", &json!({}), &db).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    // --- update_task tests ---

    #[test]
    fn test_update_task_success() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let epic_id = create_test_epic(&db, &project_id);

        let create_result = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "Original", "description": "old desc"}),
            &db,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool(
            "update_task",
            &json!({"id": id, "title": "Renamed", "status": "done"}),
            &db,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let updated = parse_response(&result);
        assert_eq!(updated["title"], "Renamed");
        assert_eq!(updated["description"], "old desc");
        assert_eq!(updated["status"], "done");
    }

    #[test]
    fn test_update_task_not_found() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "update_task",
            &json!({"id": "nonexistent", "title": "X"}),
            &db,
        )
        .unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    // --- delete_task tests ---

    #[test]
    fn test_delete_task_success() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let epic_id = create_test_epic(&db, &project_id);

        let create_result = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "To Delete", "description": "d"}),
            &db,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool("delete_task", &json!({"id": id}), &db).unwrap();
        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["deleted"], true);

        // Verify it's gone
        let get_result = dispatch_tool("get_task", &json!({"id": id}), &db).unwrap();
        assert_eq!(get_result["isError"], true);
    }

    #[test]
    fn test_delete_task_not_found() {
        let (db, _dir) = test_db();
        let result =
            dispatch_tool("delete_task", &json!({"id": "nonexistent"}), &db).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[test]
    fn test_delete_task_cascades_dependencies() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let epic_id = create_test_epic(&db, &project_id);

        let r1 = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "Blocker", "description": "d"}),
            &db,
        )
        .unwrap();
        let t1 = parse_response(&r1);
        let t1_id = t1["id"].as_str().unwrap();

        let r2 = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "Blocked", "description": "d"}),
            &db,
        )
        .unwrap();
        let t2 = parse_response(&r2);
        let t2_id = t2["id"].as_str().unwrap();

        // Add dependency: t1 blocks t2
        dep_db::add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1_id.to_string(),
                blocked_type: DependencyType::Task,
                blocked_id: t2_id.to_string(),
            },
        )
        .unwrap();

        // Delete t1 — should cascade-remove the dependency
        dispatch_tool("delete_task", &json!({"id": t1_id}), &db).unwrap();

        // Verify dependency is gone
        let blockers = dep_db::get_blockers(&db, &DependencyType::Task, t2_id).unwrap();
        assert!(blockers.is_empty(), "dependency should be cascade-deleted");
    }

    // --- Full CRUD lifecycle test ---

    #[test]
    fn test_task_full_crud_lifecycle() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let epic_id = create_test_epic(&db, &project_id);

        // Create
        let create_result = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "Lifecycle", "description": "testing"}),
            &db,
        )
        .unwrap();
        assert!(create_result.get("isError").is_none());
        let created = parse_response(&create_result);
        let id = created["id"].as_str().unwrap();
        assert_eq!(created["status"], "todo");

        // Read
        let get_result = dispatch_tool("get_task", &json!({"id": id}), &db).unwrap();
        let data = parse_response(&get_result);
        assert_eq!(data["task"]["title"], "Lifecycle");

        // List
        let list_result = dispatch_tool(
            "list_tasks",
            &json!({"epic_id": epic_id}),
            &db,
        )
        .unwrap();
        let tasks = parse_response(&list_result);
        assert_eq!(tasks.as_array().unwrap().len(), 1);

        // Update
        let update_result = dispatch_tool(
            "update_task",
            &json!({"id": id, "title": "Updated", "status": "done"}),
            &db,
        )
        .unwrap();
        let updated = parse_response(&update_result);
        assert_eq!(updated["title"], "Updated");
        assert_eq!(updated["status"], "done");

        // Delete
        let delete_result = dispatch_tool("delete_task", &json!({"id": id}), &db).unwrap();
        let deleted = parse_response(&delete_result);
        assert_eq!(deleted["deleted"], true);

        // Verify gone
        let get_result = dispatch_tool("get_task", &json!({"id": id}), &db).unwrap();
        assert_eq!(get_result["isError"], true);
    }
}
