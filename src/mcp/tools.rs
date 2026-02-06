use serde::Serialize;
use serde_json::{json, Value};

use crate::db::epic as epic_db;
use crate::db::project as project_db;
use crate::db::Database;
use crate::models::project::{CreateProjectInput, ProjectStatus, UpdateProjectInput};

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}

const DEPENDENCY_REQUIRED: [&str; 4] =
    ["blocker_type", "blocker_id", "blocked_type", "blocked_id"];

fn dependency_properties() -> Value {
    json!({
        "blocker_type": {
            "type": "string",
            "enum": ["epic", "task"],
            "description": "Type of the blocking item"
        },
        "blocker_id": { "type": "string", "description": "ID of the blocking item" },
        "blocked_type": {
            "type": "string",
            "enum": ["epic", "task"],
            "description": "Type of the blocked item"
        },
        "blocked_id": { "type": "string", "description": "ID of the blocked item" }
    })
}

pub fn tool_definitions() -> Vec<Value> {
    vec![
        // Project tools
        tool(
            "create_project",
            "Create a new project",
            json!({
                "name": { "type": "string", "description": "Project name" },
                "description": { "type": "string", "description": "Project description" }
            }),
            &["name", "description"],
        ),
        tool(
            "list_projects",
            "List all projects",
            json!({
                "status": {
                    "type": "string",
                    "enum": ["active", "archived"],
                    "description": "Filter by status"
                }
            }),
            &[],
        ),
        tool(
            "get_project",
            "Get a project by ID",
            json!({
                "id": { "type": "string", "description": "Project ID" }
            }),
            &["id"],
        ),
        tool(
            "update_project",
            "Update a project",
            json!({
                "id": { "type": "string", "description": "Project ID" },
                "name": { "type": "string", "description": "New name" },
                "description": { "type": "string", "description": "New description" },
                "status": {
                    "type": "string",
                    "enum": ["active", "archived"],
                    "description": "New status"
                }
            }),
            &["id"],
        ),
        tool(
            "delete_project",
            "Delete a project",
            json!({
                "id": { "type": "string", "description": "Project ID" }
            }),
            &["id"],
        ),
        // Epic tools
        tool(
            "create_epic",
            "Create a new epic within a project",
            json!({
                "project_id": { "type": "string", "description": "Parent project ID" },
                "title": { "type": "string", "description": "Epic title" },
                "description": { "type": "string", "description": "Epic description" }
            }),
            &["project_id", "title", "description"],
        ),
        tool(
            "list_epics",
            "List epics, optionally filtered by project or status",
            json!({
                "project_id": { "type": "string", "description": "Filter by project ID" },
                "status": {
                    "type": "string",
                    "enum": ["todo", "in_progress", "done"],
                    "description": "Filter by status"
                }
            }),
            &[],
        ),
        tool(
            "get_epic",
            "Get an epic by ID",
            json!({
                "id": { "type": "string", "description": "Epic ID" }
            }),
            &["id"],
        ),
        tool(
            "update_epic",
            "Update an epic",
            json!({
                "id": { "type": "string", "description": "Epic ID" },
                "title": { "type": "string", "description": "New title" },
                "description": { "type": "string", "description": "New description" },
                "status": {
                    "type": "string",
                    "enum": ["todo", "in_progress", "done"],
                    "description": "New status"
                }
            }),
            &["id"],
        ),
        tool(
            "delete_epic",
            "Delete an epic",
            json!({
                "id": { "type": "string", "description": "Epic ID" }
            }),
            &["id"],
        ),
        // Task tools
        tool(
            "create_task",
            "Create a new task within an epic",
            json!({
                "epic_id": { "type": "string", "description": "Parent epic ID" },
                "title": { "type": "string", "description": "Task title" },
                "description": { "type": "string", "description": "Task description" }
            }),
            &["epic_id", "title", "description"],
        ),
        tool(
            "list_tasks",
            "List tasks, optionally filtered by epic or status",
            json!({
                "epic_id": { "type": "string", "description": "Filter by epic ID" },
                "status": {
                    "type": "string",
                    "enum": ["todo", "in_progress", "done"],
                    "description": "Filter by status"
                }
            }),
            &[],
        ),
        tool(
            "get_task",
            "Get a task by ID",
            json!({
                "id": { "type": "string", "description": "Task ID" }
            }),
            &["id"],
        ),
        tool(
            "update_task",
            "Update a task",
            json!({
                "id": { "type": "string", "description": "Task ID" },
                "title": { "type": "string", "description": "New title" },
                "description": { "type": "string", "description": "New description" },
                "status": {
                    "type": "string",
                    "enum": ["todo", "in_progress", "done"],
                    "description": "New status"
                }
            }),
            &["id"],
        ),
        tool(
            "delete_task",
            "Delete a task",
            json!({
                "id": { "type": "string", "description": "Task ID" }
            }),
            &["id"],
        ),
        // Dependency tools
        tool(
            "add_dependency",
            "Add a dependency between epics or tasks",
            dependency_properties(),
            &DEPENDENCY_REQUIRED,
        ),
        tool(
            "remove_dependency",
            "Remove a dependency between epics or tasks",
            dependency_properties(),
            &DEPENDENCY_REQUIRED,
        ),
        // Status tool
        tool(
            "get_status",
            "Get project status overview with progress summaries",
            json!({
                "project_id": { "type": "string", "description": "Filter by project ID" }
            }),
            &[],
        ),
        // PRD tool
        tool(
            "feed_prd",
            "Feed a PRD document to break down into epics and tasks",
            json!({
                "project_id": { "type": "string", "description": "Target project ID" },
                "content": { "type": "string", "description": "PRD content as text or markdown" },
                "title": { "type": "string", "description": "PRD title" }
            }),
            &["project_id", "content", "title"],
        ),
    ]
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tool_result(data: &impl Serialize) -> Value {
    let text = serde_json::to_string_pretty(data).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"));
    json!({ "content": [{ "type": "text", "text": text }] })
}

fn tool_error(msg: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": msg }], "isError": true })
}

fn require_str(args: &Value, field: &str) -> Result<String, Value> {
    args.get(field)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| tool_error(&format!("Missing required parameter: {field}")))
}

fn optional_str(args: &Value, field: &str) -> Option<String> {
    args.get(field).and_then(|v| v.as_str()).map(String::from)
}

fn parse_optional_project_status(args: &Value) -> Result<Option<ProjectStatus>, Value> {
    match optional_str(args, "status") {
        Some(s) => s
            .parse::<ProjectStatus>()
            .map(Some)
            .map_err(|_| tool_error(&format!("Invalid status: {s}"))),
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Project handlers
// ---------------------------------------------------------------------------

fn handle_create_project(args: &Value, db: &Database) -> Value {
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

fn handle_list_projects(args: &Value, db: &Database) -> Value {
    let status = match parse_optional_project_status(args) {
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

fn handle_get_project(args: &Value, db: &Database) -> Value {
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

fn handle_update_project(args: &Value, db: &Database) -> Value {
    let id = match require_str(args, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let status = match parse_optional_project_status(args) {
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

fn handle_delete_project(args: &Value, db: &Database) -> Value {
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

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch_tool(name: &str, args: &Value, db: &Database) -> Option<Value> {
    match name {
        "create_project" => Some(handle_create_project(args, db)),
        "list_projects" => Some(handle_list_projects(args, db)),
        "get_project" => Some(handle_get_project(args, db)),
        "update_project" => Some(handle_update_project(args, db)),
        "delete_project" => Some(handle_delete_project(args, db)),
        _ => {
            let is_known = tool_definitions()
                .iter()
                .any(|t| t["name"].as_str() == Some(name));
            if is_known {
                Some(tool_error(&format!("Tool '{name}' not yet implemented")))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tempfile::TempDir;

    fn test_db() -> (Database, TempDir) {
        let dir = TempDir::new().unwrap();
        let db = Database::open(&dir.path().join("test.db")).unwrap();
        db.migrate().unwrap();
        (db, dir)
    }

    // --- Tool definition tests ---

    #[test]
    fn test_tool_definitions_count() {
        assert_eq!(tool_definitions().len(), 19);
    }

    #[test]
    fn test_tool_definitions_have_required_fields() {
        for tool in tool_definitions() {
            assert!(tool.get("name").is_some(), "missing name");
            assert!(tool.get("description").is_some(), "missing description");
            assert!(tool.get("inputSchema").is_some(), "missing inputSchema");
        }
    }

    #[test]
    fn test_tool_definitions_unique_names() {
        let defs = tool_definitions();
        let names: Vec<&str> = defs.iter().map(|t| t["name"].as_str().unwrap()).collect();
        let unique: HashSet<&str> = names.iter().copied().collect();
        assert_eq!(names.len(), unique.len(), "duplicate tool names found");
    }

    // --- Dispatch tests ---

    #[test]
    fn test_dispatch_known_tool_returns_stub() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("create_epic", &json!({}), &db);
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val["isError"], true);
        assert!(val["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("create_epic"));
    }

    #[test]
    fn test_dispatch_unknown_tool_returns_none() {
        let (db, _dir) = test_db();
        assert!(dispatch_tool("nonexistent_tool", &json!({}), &db).is_none());
    }

    // --- create_project tests ---

    #[test]
    fn test_create_project_success() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "create_project",
            &json!({"name": "Test", "description": "A project"}),
            &db,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let text = result["content"][0]["text"].as_str().unwrap();
        let project: Value = serde_json::from_str(text).unwrap();
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
        let result = dispatch_tool("list_projects", &json!({}), &db).unwrap();

        assert!(result.get("isError").is_none());
        let text = result["content"][0]["text"].as_str().unwrap();
        let projects: Vec<Value> = serde_json::from_str(text).unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_list_projects_with_data() {
        let (db, _dir) = test_db();
        dispatch_tool(
            "create_project",
            &json!({"name": "P1", "description": "d1"}),
            &db,
        );
        dispatch_tool(
            "create_project",
            &json!({"name": "P2", "description": "d2"}),
            &db,
        );

        let result = dispatch_tool("list_projects", &json!({}), &db).unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        let projects: Vec<Value> = serde_json::from_str(text).unwrap();
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn test_list_projects_with_status_filter() {
        let (db, _dir) = test_db();
        let create_result = dispatch_tool(
            "create_project",
            &json!({"name": "Active", "description": "d"}),
            &db,
        )
        .unwrap();
        let text = create_result["content"][0]["text"].as_str().unwrap();
        let project: Value = serde_json::from_str(text).unwrap();
        let id = project["id"].as_str().unwrap();

        // Archive it
        dispatch_tool(
            "update_project",
            &json!({"id": id, "status": "archived"}),
            &db,
        );

        // Create another active project
        dispatch_tool(
            "create_project",
            &json!({"name": "Still Active", "description": "d"}),
            &db,
        );

        let active = dispatch_tool("list_projects", &json!({"status": "active"}), &db).unwrap();
        let text = active["content"][0]["text"].as_str().unwrap();
        let projects: Vec<Value> = serde_json::from_str(text).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0]["name"], "Still Active");

        let archived =
            dispatch_tool("list_projects", &json!({"status": "archived"}), &db).unwrap();
        let text = archived["content"][0]["text"].as_str().unwrap();
        let projects: Vec<Value> = serde_json::from_str(text).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0]["name"], "Active");
    }

    #[test]
    fn test_list_projects_invalid_status() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("list_projects", &json!({"status": "bogus"}), &db).unwrap();
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
        )
        .unwrap();
        let text = create_result["content"][0]["text"].as_str().unwrap();
        let created: Value = serde_json::from_str(text).unwrap();
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool("get_project", &json!({"id": id}), &db).unwrap();
        assert!(result.get("isError").is_none());
        let text = result["content"][0]["text"].as_str().unwrap();
        let data: Value = serde_json::from_str(text).unwrap();
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
        )
        .unwrap();
        let text = create_result["content"][0]["text"].as_str().unwrap();
        let created: Value = serde_json::from_str(text).unwrap();
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

        let result = dispatch_tool("get_project", &json!({"id": project_id}), &db).unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        let data: Value = serde_json::from_str(text).unwrap();
        assert_eq!(data["epics"].as_array().unwrap().len(), 1);
        assert_eq!(data["epics"][0]["title"], "Child Epic");
    }

    #[test]
    fn test_get_project_not_found() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("get_project", &json!({"id": "nonexistent"}), &db).unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[test]
    fn test_get_project_missing_id() {
        let (db, _dir) = test_db();
        let result = dispatch_tool("get_project", &json!({}), &db).unwrap();
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
        )
        .unwrap();
        let text = create_result["content"][0]["text"].as_str().unwrap();
        let created: Value = serde_json::from_str(text).unwrap();
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool(
            "update_project",
            &json!({"id": id, "name": "Renamed", "status": "archived"}),
            &db,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let text = result["content"][0]["text"].as_str().unwrap();
        let updated: Value = serde_json::from_str(text).unwrap();
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
        )
        .unwrap();
        let text = create_result["content"][0]["text"].as_str().unwrap();
        let created: Value = serde_json::from_str(text).unwrap();
        let id = created["id"].as_str().unwrap();

        let result = dispatch_tool("delete_project", &json!({"id": id}), &db).unwrap();
        assert!(result.get("isError").is_none());
        let text = result["content"][0]["text"].as_str().unwrap();
        let data: Value = serde_json::from_str(text).unwrap();
        assert_eq!(data["deleted"], true);

        // Verify it's gone
        let get_result = dispatch_tool("get_project", &json!({"id": id}), &db).unwrap();
        assert_eq!(get_result["isError"], true);
    }

    #[test]
    fn test_delete_project_not_found() {
        let (db, _dir) = test_db();
        let result =
            dispatch_tool("delete_project", &json!({"id": "nonexistent"}), &db).unwrap();
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
        )
        .unwrap();
        let text = create_result["content"][0]["text"].as_str().unwrap();
        let created: Value = serde_json::from_str(text).unwrap();
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
        dispatch_tool("delete_project", &json!({"id": project_id}), &db).unwrap();

        // Verify epics are gone
        let epics = epic_db::list_epics(&db, Some(project_id), None).unwrap();
        assert!(epics.is_empty(), "epics should be cascade-deleted");
    }
}
