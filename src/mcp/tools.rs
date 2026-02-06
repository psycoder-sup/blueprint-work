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
        "blocker_id": { "type": "integer", "description": "ID of the blocking item" },
        "blocked_type": {
            "type": "string",
            "enum": ["epic", "task"],
            "description": "Type of the blocked item"
        },
        "blocked_id": { "type": "integer", "description": "ID of the blocked item" }
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
                "id": { "type": "integer", "description": "Project ID" }
            }),
            &["id"],
        ),
        tool(
            "update_project",
            "Update a project",
            json!({
                "id": { "type": "integer", "description": "Project ID" },
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
                "id": { "type": "integer", "description": "Project ID" }
            }),
            &["id"],
        ),
        // Epic tools
        tool(
            "create_epic",
            "Create a new epic within a project",
            json!({
                "project_id": { "type": "integer", "description": "Parent project ID" },
                "title": { "type": "string", "description": "Epic title" },
                "description": { "type": "string", "description": "Epic description" }
            }),
            &["project_id", "title", "description"],
        ),
        tool(
            "list_epics",
            "List epics, optionally filtered by project or status",
            json!({
                "project_id": { "type": "integer", "description": "Filter by project ID" },
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
                "id": { "type": "integer", "description": "Epic ID" }
            }),
            &["id"],
        ),
        tool(
            "update_epic",
            "Update an epic",
            json!({
                "id": { "type": "integer", "description": "Epic ID" },
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
                "id": { "type": "integer", "description": "Epic ID" }
            }),
            &["id"],
        ),
        // Task tools
        tool(
            "create_task",
            "Create a new task within an epic",
            json!({
                "epic_id": { "type": "integer", "description": "Parent epic ID" },
                "title": { "type": "string", "description": "Task title" },
                "description": { "type": "string", "description": "Task description" }
            }),
            &["epic_id", "title", "description"],
        ),
        tool(
            "list_tasks",
            "List tasks, optionally filtered by epic or status",
            json!({
                "epic_id": { "type": "integer", "description": "Filter by epic ID" },
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
                "id": { "type": "integer", "description": "Task ID" }
            }),
            &["id"],
        ),
        tool(
            "update_task",
            "Update a task",
            json!({
                "id": { "type": "integer", "description": "Task ID" },
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
                "id": { "type": "integer", "description": "Task ID" }
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
                "project_id": { "type": "integer", "description": "Filter by project ID" }
            }),
            &[],
        ),
        // PRD tool
        tool(
            "feed_prd",
            "Feed a PRD document to break down into epics and tasks",
            json!({
                "project_id": { "type": "integer", "description": "Target project ID" },
                "content": { "type": "string", "description": "PRD content as text or markdown" },
                "title": { "type": "string", "description": "PRD title" }
            }),
            &["project_id", "content", "title"],
        ),
    ]
}

pub fn dispatch_tool(name: &str, _args: &Value, _db: &Database) -> Option<Value> {
    let is_known = tool_definitions()
        .iter()
        .any(|t| t["name"].as_str() == Some(name));

    if is_known {
        Some(json!({
            "content": [{ "type": "text", "text": format!("Tool '{name}' not yet implemented") }],
            "isError": true
        }))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tempfile::TempDir;

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

    #[test]
    fn test_dispatch_known_tool_returns_stub() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(&dir.path().join("test.db")).unwrap();
        db.migrate().unwrap();

        let result = dispatch_tool("create_project", &json!({}), &db);
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val["isError"], true);
        assert!(val["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("create_project"));
    }

    #[test]
    fn test_dispatch_unknown_tool_returns_none() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(&dir.path().join("test.db")).unwrap();
        db.migrate().unwrap();

        assert!(dispatch_tool("nonexistent_tool", &json!({}), &db).is_none());
    }
}
