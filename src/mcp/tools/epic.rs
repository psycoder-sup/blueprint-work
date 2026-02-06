use serde_json::{json, Value};

use crate::db::dependency as dep_db;
use crate::db::epic as epic_db;
use crate::db::project as project_db;
use crate::db::task as task_db;
use crate::db::Database;
use crate::models::dependency::DependencyType;
use crate::models::epic::{CreateEpicInput, ItemStatus, UpdateEpicInput};

use super::{optional_str, parse_optional_status, require_str, tool_error, tool_result};

pub(super) fn handle_create_epic(args: &Value, db: &Database) -> Value {
    let project_id = match require_str(args, "project_id") {
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

    // Validate project exists
    match project_db::get_project(db, &project_id) {
        Ok(Some(_)) => {}
        Ok(None) => return tool_error(&format!("Project not found: {project_id}")),
        Err(e) => {
            eprintln!("create_epic error: {e:#}");
            return tool_error("Failed to create epic");
        }
    }

    match epic_db::create_epic(db, CreateEpicInput { project_id, title, description }) {
        Ok(epic) => tool_result(&epic),
        Err(e) => {
            eprintln!("create_epic error: {e:#}");
            tool_error("Failed to create epic")
        }
    }
}

pub(super) fn handle_list_epics(args: &Value, db: &Database) -> Value {
    let project_id = optional_str(args, "project_id");
    let status = match parse_optional_status::<ItemStatus>(args) {
        Ok(s) => s,
        Err(e) => return e,
    };

    match epic_db::list_epics(db, project_id.as_deref(), status) {
        Ok(epics) => tool_result(&epics),
        Err(e) => {
            eprintln!("list_epics error: {e:#}");
            tool_error("Failed to list epics")
        }
    }
}

pub(super) fn handle_get_epic(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let epic = match epic_db::get_epic(db, &id) {
        Ok(Some(e)) => e,
        Ok(None) => return tool_error(&format!("Epic not found: {id}")),
        Err(e) => {
            eprintln!("get_epic error: {e:#}");
            return tool_error("Failed to get epic");
        }
    };

    let tasks = match task_db::list_tasks(db, Some(&id), None) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("list_tasks error: {e:#}");
            return tool_error("Failed to get epic");
        }
    };

    let blockers = match dep_db::get_blockers(db, &DependencyType::Epic, &id) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("get_blockers error: {e:#}");
            return tool_error("Failed to get epic");
        }
    };

    let blocks = match dep_db::get_blocked_by(db, &DependencyType::Epic, &id) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("get_blocked_by error: {e:#}");
            return tool_error("Failed to get epic");
        }
    };

    tool_result(&json!({ "epic": epic, "tasks": tasks, "blockers": blockers, "blocks": blocks }))
}

pub(super) fn handle_update_epic(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let status = match parse_optional_status::<ItemStatus>(args) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let input = UpdateEpicInput {
        title: optional_str(args, "title"),
        description: optional_str(args, "description"),
        status,
    };

    match epic_db::update_epic(db, &id, input) {
        Ok(epic) => tool_result(&epic),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") {
                tool_error(&format!("Epic not found: {id}"))
            } else {
                eprintln!("update_epic error: {e:#}");
                tool_error("Failed to update epic")
            }
        }
    }
}

pub(super) fn handle_delete_epic(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    match epic_db::delete_epic(db, &id) {
        Ok(true) => tool_result(&json!({ "deleted": true, "id": id })),
        Ok(false) => tool_error(&format!("Epic not found: {id}")),
        Err(e) => {
            eprintln!("delete_epic error: {e:#}");
            tool_error("Failed to delete epic")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::dispatch_tool;
    use crate::db::task as task_db;
    use crate::db::Database;
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
            &json!({"name": "Test Project", "description": "for epic tests"}),
            db,
        )
        .unwrap();
        let project = parse_response(&result);
        project["id"].as_str().unwrap().to_string()
    }

    // --- create_epic tests ---

    #[test]
    fn test_create_epic_success() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);

        let result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "My Epic", "description": "desc"}),
            &db,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let epic = parse_response(&result);
        assert_eq!(epic["title"], "My Epic");
        assert_eq!(epic["description"], "desc");
        assert_eq!(epic["status"], "todo");
        assert_eq!(epic["project_id"], project_id);
        assert_eq!(epic["id"].as_str().unwrap().len(), 26);
    }

    #[test]
    fn test_create_epic_missing_project_id() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "create_epic",
            &json!({"title": "E", "description": "d"}),
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
    fn test_create_epic_missing_title() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        let result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "description": "d"}),
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
    fn test_create_epic_invalid_project_id() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "create_epic",
            &json!({"project_id": "nonexistent", "title": "E", "description": "d"}),
            &db,
        )
        .unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    // --- list_epics tests ---

    #[test]
    fn test_list_epics_empty() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("list_epics", &json!({}), &db).unwrap();
        assert!(result.get("isError").is_none());
        let epics = parse_response(&result);
        assert!(epics.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_list_epics_with_data() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);
        dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "E1", "description": "d"}),
            &db,
        );
        dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "E2", "description": "d"}),
            &db,
        );

        let result = dispatch_tool("list_epics", &json!({}), &db).unwrap();
        let epics = parse_response(&result);
        assert_eq!(epics.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_list_epics_project_id_filter() {
        let (db, _dir) = test_db();
        let p1 = create_test_project(&db);
        let p2 = create_test_project(&db);
        dispatch_tool(
            "create_epic",
            &json!({"project_id": p1, "title": "E1", "description": "d"}),
            &db,
        );
        dispatch_tool(
            "create_epic",
            &json!({"project_id": p2, "title": "E2", "description": "d"}),
            &db,
        );

        let result = dispatch_tool("list_epics", &json!({"project_id": p1}), &db).unwrap();
        let epics = parse_response(&result);
        assert_eq!(epics.as_array().unwrap().len(), 1);
        assert_eq!(epics[0]["title"], "E1");
    }

    #[test]
    fn test_list_epics_status_filter() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);

        let create_result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "E1", "description": "d"}),
            &db,
        )
        .unwrap();
        let epic = parse_response(&create_result);
        let epic_id = epic["id"].as_str().unwrap();

        dispatch_tool(
            "update_epic",
            &json!({"id": epic_id, "status": "in_progress"}),
            &db,
        );

        dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "E2", "description": "d"}),
            &db,
        );

        let result = dispatch_tool("list_epics", &json!({"status": "in_progress"}), &db).unwrap();
        let epics = parse_response(&result);
        assert_eq!(epics.as_array().unwrap().len(), 1);
        assert_eq!(epics[0]["title"], "E1");
    }

    #[test]
    fn test_list_epics_invalid_status() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("list_epics", &json!({"status": "bogus"}), &db).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Invalid status"));
    }

    // --- get_epic tests ---

    #[test]
    fn test_get_epic_success_with_nested_data() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);

        let create_result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "Get Me", "description": "desc"}),
            &db,
        )
        .unwrap();
        let epic = parse_response(&create_result);
        let epic_id = epic["id"].as_str().unwrap();

        // Add a task via DB layer
        task_db::create_task(
            &db,
            crate::models::task::CreateTaskInput {
                epic_id: epic_id.to_string(),
                title: "Child Task".to_string(),
                description: "task desc".to_string(),
            },
        )
        .unwrap();

        let result = dispatch_tool("get_epic", &json!({"id": epic_id}), &db).unwrap();
        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["epic"]["title"], "Get Me");
        assert_eq!(data["tasks"].as_array().unwrap().len(), 1);
        assert_eq!(data["tasks"][0]["title"], "Child Task");
        assert!(data["blockers"].as_array().unwrap().is_empty());
        assert!(data["blocks"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_get_epic_not_found() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("get_epic", &json!({"id": "nonexistent"}), &db).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[test]
    fn test_get_epic_missing_id() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("get_epic", &json!({}), &db).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    // --- update_epic tests ---

    #[test]
    fn test_update_epic_success() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);

        let create_result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "Original", "description": "old desc"}),
            &db,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool(
            "update_epic",
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
    fn test_update_epic_not_found() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "update_epic",
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

    // --- delete_epic tests ---

    #[test]
    fn test_delete_epic_success() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);

        let create_result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "To Delete", "description": "d"}),
            &db,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool("delete_epic", &json!({"id": id}), &db).unwrap();
        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["deleted"], true);

        // Verify it's gone
        let get_result = dispatch_tool("get_epic", &json!({"id": id}), &db).unwrap();
        assert_eq!(get_result["isError"], true);
    }

    #[test]
    fn test_delete_epic_not_found() {
        let (db, _dir) = test_db();
        let result =
            dispatch_tool("delete_epic", &json!({"id": "nonexistent"}), &db).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[test]
    fn test_delete_epic_cascades_to_tasks() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);

        let create_result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "Parent", "description": "d"}),
            &db,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let epic_id = created["id"].as_str().unwrap();

        // Create a task under this epic
        task_db::create_task(
            &db,
            crate::models::task::CreateTaskInput {
                epic_id: epic_id.to_string(),
                title: "Child Task".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        // Delete the epic
        dispatch_tool("delete_epic", &json!({"id": epic_id}), &db).unwrap();

        // Verify tasks are gone
        let tasks = task_db::list_tasks(&db, Some(epic_id), None).unwrap();
        assert!(tasks.is_empty(), "tasks should be cascade-deleted");
    }

    // --- Full CRUD lifecycle test ---

    #[test]
    fn test_epic_full_crud_lifecycle() {
        let (db, _dir) = test_db();
        let project_id = create_test_project(&db);

        // Create
        let create_result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "Lifecycle", "description": "testing"}),
            &db,
        )
        .unwrap();
        assert!(create_result.get("isError").is_none());
        let created = parse_response(&create_result);
        let id = created["id"].as_str().unwrap();
        assert_eq!(created["status"], "todo");

        // Read
        let get_result = dispatch_tool("get_epic", &json!({"id": id}), &db).unwrap();
        let data = parse_response(&get_result);
        assert_eq!(data["epic"]["title"], "Lifecycle");

        // List
        let list_result = dispatch_tool(
            "list_epics",
            &json!({"project_id": project_id}),
            &db,
        )
        .unwrap();
        let epics = parse_response(&list_result);
        assert_eq!(epics.as_array().unwrap().len(), 1);

        // Update
        let update_result = dispatch_tool(
            "update_epic",
            &json!({"id": id, "title": "Updated", "status": "done"}),
            &db,
        )
        .unwrap();
        let updated = parse_response(&update_result);
        assert_eq!(updated["title"], "Updated");
        assert_eq!(updated["status"], "done");

        // Delete
        let delete_result = dispatch_tool("delete_epic", &json!({"id": id}), &db).unwrap();
        let deleted = parse_response(&delete_result);
        assert_eq!(deleted["deleted"], true);

        // Verify gone
        let get_result = dispatch_tool("get_epic", &json!({"id": id}), &db).unwrap();
        assert_eq!(get_result["isError"], true);
    }
}
