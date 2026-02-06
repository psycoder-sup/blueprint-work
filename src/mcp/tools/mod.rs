mod dependency;
mod epic;
mod prd;
mod project;
mod status;
mod task;

use serde::Serialize;
use serde_json::{json, Value};

use crate::db::Database;

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
                "title": { "type": "string", "description": "PRD title" },
                "content": { "type": "string", "description": "PRD content as text or markdown" }
            }),
            &["project_id", "title", "content"],
        ),
    ]
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn tool_result(data: &impl Serialize) -> Value {
    let text = serde_json::to_string_pretty(data)
        .unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"));
    json!({ "content": [{ "type": "text", "text": text }] })
}

pub(crate) fn tool_error(msg: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": msg }], "isError": true })
}

pub(crate) fn require_str(args: &Value, field: &str) -> Result<String, Value> {
    args.get(field)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| tool_error(&format!("Missing required parameter: {field}")))
}

pub(crate) fn optional_str(args: &Value, field: &str) -> Option<String> {
    args.get(field).and_then(|v| v.as_str()).map(String::from)
}

pub(crate) fn parse_optional_status<T: std::str::FromStr>(args: &Value) -> Result<Option<T>, Value> {
    match optional_str(args, "status") {
        Some(s) => s
            .parse::<T>()
            .map(Some)
            .map_err(|_| tool_error(&format!("Invalid status: {s}"))),
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch_tool(name: &str, args: &Value, db: &Database) -> Option<Value> {
    let result = match name {
        "create_project" => project::handle_create_project(args, db),
        "list_projects" => project::handle_list_projects(args, db),
        "get_project" => project::handle_get_project(args, db),
        "update_project" => project::handle_update_project(args, db),
        "delete_project" => project::handle_delete_project(args, db),
        "create_epic" => epic::handle_create_epic(args, db),
        "list_epics" => epic::handle_list_epics(args, db),
        "get_epic" => epic::handle_get_epic(args, db),
        "update_epic" => epic::handle_update_epic(args, db),
        "delete_epic" => epic::handle_delete_epic(args, db),
        "create_task" => task::handle_create_task(args, db),
        "list_tasks" => task::handle_list_tasks(args, db),
        "get_task" => task::handle_get_task(args, db),
        "update_task" => task::handle_update_task(args, db),
        "delete_task" => task::handle_delete_task(args, db),
        "add_dependency" => dependency::handle_add_dependency(args, db),
        "remove_dependency" => dependency::handle_remove_dependency(args, db),
        "get_status" => status::handle_get_status(args, db),
        "feed_prd" => prd::handle_feed_prd(args, db),
        _ => {
            let is_known = tool_definitions()
                .iter()
                .any(|t| t["name"].as_str() == Some(name));
            return if is_known {
                Some(tool_error(&format!("Tool '{name}' not yet implemented")))
            } else {
                None
            };
        }
    };
    Some(result)
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
    fn test_dispatch_unknown_tool_returns_none() {
        let (db, _dir) = test_db();
        assert!(dispatch_tool("nonexistent_tool", &json!({}), &db).is_none());
    }
}
