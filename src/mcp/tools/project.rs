use serde_json::{json, Value};

use crate::db::epic as epic_db;
use crate::db::project as project_db;
use crate::db::Database;
use crate::models::project::{CreateProjectInput, ProjectStatus, UpdateProjectInput};

use super::{optional_str, parse_optional_status, require_str, tool_error, tool_result};

pub(super) fn handle_create_project(args: &Value, db: &Database) -> Value {
    let name = match require_str(args, "name") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let description = match require_str(args, "description") {
        Ok(v) => v,
        Err(e) => return e,
    };

    match project_db::create_project(db, CreateProjectInput { name, description }) {
        Ok(project) => tool_result(&project),
        Err(e) => {
            eprintln!("create_project error: {e:#}");
            tool_error("Failed to create project")
        }
    }
}

pub(super) fn handle_list_projects(args: &Value, db: &Database) -> Value {
    let status = match parse_optional_status::<ProjectStatus>(args) {
        Ok(s) => s,
        Err(e) => return e,
    };

    match project_db::list_projects(db, status) {
        Ok(projects) => tool_result(&projects),
        Err(e) => {
            eprintln!("list_projects error: {e:#}");
            tool_error("Failed to list projects")
        }
    }
}

pub(super) fn handle_get_project(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let project = match project_db::get_project(db, &id) {
        Ok(Some(p)) => p,
        Ok(None) => return tool_error(&format!("Project not found: {id}")),
        Err(e) => {
            eprintln!("get_project error: {e:#}");
            return tool_error("Failed to get project");
        }
    };

    let epics = match epic_db::list_epics(db, Some(&id), None) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("list_epics error: {e:#}");
            return tool_error("Failed to get project");
        }
    };

    tool_result(&json!({ "project": project, "epics": epics }))
}

pub(super) fn handle_update_project(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let status = match parse_optional_status::<ProjectStatus>(args) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let input = UpdateProjectInput {
        name: optional_str(args, "name"),
        description: optional_str(args, "description"),
        status,
    };

    match project_db::update_project(db, &id, input) {
        Ok(project) => tool_result(&project),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") {
                tool_error(&format!("Project not found: {id}"))
            } else {
                eprintln!("update_project error: {e:#}");
                tool_error("Failed to update project")
            }
        }
    }
}

pub(super) fn handle_delete_project(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    match project_db::delete_project(db, &id) {
        Ok(true) => tool_result(&json!({ "deleted": true, "id": id })),
        Ok(false) => tool_error(&format!("Project not found: {id}")),
        Err(e) => {
            eprintln!("delete_project error: {e:#}");
            tool_error("Failed to delete project")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::dispatch_tool;
    use crate::db::epic as epic_db;
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

    // --- create_project tests ---

    #[test]
    fn test_create_project_success() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "create_project",
            &json!({"name": "Test", "description": "A project"}),
            &db,
            None,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let project = parse_response(&result);
        assert_eq!(project["name"], "Test");
        assert_eq!(project["description"], "A project");
        assert_eq!(project["status"], "active");
        assert_eq!(project["id"].as_str().unwrap().len(), 26); // ULID
    }

    #[test]
    fn test_create_project_missing_name() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "create_project",
            &json!({"description": "desc"}),
            &db,
            None,
        )
        .unwrap();

        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    #[test]
    fn test_create_project_missing_description() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "create_project",
            &json!({"name": "Test"}),
            &db,
            None,
        )
        .unwrap();

        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    // --- list_projects tests ---

    #[test]
    fn test_list_projects_empty() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("list_projects", &json!({}), &db, None).unwrap();

        assert!(result.get("isError").is_none());
        let projects = parse_response(&result);
        assert!(projects.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_list_projects_with_data() {
        let (db, _dir) = test_db();
        dispatch_tool(
            "create_project",
            &json!({"name": "P1", "description": "d1"}),
            &db,
            None,
        );
        dispatch_tool(
            "create_project",
            &json!({"name": "P2", "description": "d2"}),
            &db,
            None,
        );

        let result = dispatch_tool("list_projects", &json!({}), &db, None).unwrap();
        let projects = parse_response(&result);
        assert_eq!(projects.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_list_projects_with_status_filter() {
        let (db, _dir) = test_db();
        let create_result = dispatch_tool(
            "create_project",
            &json!({"name": "Active", "description": "d"}),
            &db,
            None,
        )
        .unwrap();
        let project = parse_response(&create_result);
        let id = project["id"].as_str().unwrap();

        // Archive it
        dispatch_tool(
            "update_project",
            &json!({"id": id, "status": "archived"}),
            &db,
            None,
        );

        // Create another active project
        dispatch_tool(
            "create_project",
            &json!({"name": "Still Active", "description": "d"}),
            &db,
            None,
        );

        let active = dispatch_tool("list_projects", &json!({"status": "active"}), &db, None).unwrap();
        let projects = parse_response(&active);
        assert_eq!(projects.as_array().unwrap().len(), 1);
        assert_eq!(projects[0]["name"], "Still Active");

        let archived =
            dispatch_tool("list_projects", &json!({"status": "archived"}), &db, None).unwrap();
        let projects = parse_response(&archived);
        assert_eq!(projects.as_array().unwrap().len(), 1);
        assert_eq!(projects[0]["name"], "Active");
    }

    #[test]
    fn test_list_projects_invalid_status() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("list_projects", &json!({"status": "bogus"}), &db, None).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Invalid status"));
    }

    // --- get_project tests ---

    #[test]
    fn test_get_project_success() {
        let (db, _dir) = test_db();
        let create_result = dispatch_tool(
            "create_project",
            &json!({"name": "Get Me", "description": "desc"}),
            &db,
            None,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool("get_project", &json!({"id": id}), &db, None).unwrap();
        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["project"]["name"], "Get Me");
        assert!(data["epics"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_get_project_with_epics() {
        let (db, _dir) = test_db();
        let create_result = dispatch_tool(
            "create_project",
            &json!({"name": "Parent", "description": "d"}),
            &db,
            None,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let project_id = created["id"].as_str().unwrap();

        // Insert an epic via DB layer directly
        epic_db::create_epic(
            &db,
            crate::models::epic::CreateEpicInput {
                project_id: project_id.to_string(),
                title: "Child Epic".to_string(),
                description: "desc".to_string(),
            },
        )
        .unwrap();

        let result = dispatch_tool("get_project", &json!({"id": project_id}), &db, None).unwrap();
        let data = parse_response(&result);
        assert_eq!(data["epics"].as_array().unwrap().len(), 1);
        assert_eq!(data["epics"][0]["title"], "Child Epic");
    }

    #[test]
    fn test_get_project_not_found() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("get_project", &json!({"id": "nonexistent"}), &db, None).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[test]
    fn test_get_project_missing_id() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("get_project", &json!({}), &db, None).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));
    }

    // --- update_project tests ---

    #[test]
    fn test_update_project_success() {
        let (db, _dir) = test_db();
        let create_result = dispatch_tool(
            "create_project",
            &json!({"name": "Original", "description": "old desc"}),
            &db,
            None,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool(
            "update_project",
            &json!({"id": id, "name": "Renamed", "status": "archived"}),
            &db,
            None,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let updated = parse_response(&result);
        assert_eq!(updated["name"], "Renamed");
        assert_eq!(updated["description"], "old desc");
        assert_eq!(updated["status"], "archived");
    }

    #[test]
    fn test_update_project_not_found() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "update_project",
            &json!({"id": "nonexistent", "name": "X"}),
            &db,
            None,
        )
        .unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    // --- delete_project tests ---

    #[test]
    fn test_delete_project_success() {
        let (db, _dir) = test_db();
        let create_result = dispatch_tool(
            "create_project",
            &json!({"name": "To Delete", "description": "d"}),
            &db,
            None,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool("delete_project", &json!({"id": id}), &db, None).unwrap();
        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["deleted"], true);

        // Verify it's gone
        let get_result = dispatch_tool("get_project", &json!({"id": id}), &db, None).unwrap();
        assert_eq!(get_result["isError"], true);
    }

    #[test]
    fn test_delete_project_not_found() {
        let (db, _dir) = test_db();
        let result =
            dispatch_tool("delete_project", &json!({"id": "nonexistent"}), &db, None).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[test]
    fn test_delete_project_cascades() {
        let (db, _dir) = test_db();
        let create_result = dispatch_tool(
            "create_project",
            &json!({"name": "Parent", "description": "d"}),
            &db,
            None,
        )
        .unwrap();
        let created = parse_response(&create_result);
        let project_id = created["id"].as_str().unwrap();

        // Create an epic under this project
        epic_db::create_epic(
            &db,
            crate::models::epic::CreateEpicInput {
                project_id: project_id.to_string(),
                title: "Child Epic".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        // Delete the project
        dispatch_tool("delete_project", &json!({"id": project_id}), &db, None).unwrap();

        // Verify epics are gone
        let epics = epic_db::list_epics(&db, Some(project_id), None).unwrap();
        assert!(epics.is_empty(), "epics should be cascade-deleted");
    }
}
