use serde_json::{json, Value};

use crate::db::epic as epic_db;
use crate::db::task as task_db;
use crate::db::{dependency as dep_db, Database};
use crate::models::dependency::{AddDependencyInput, DependencyType};

use super::{require_str, tool_error, tool_result};

fn parse_dependency_type(args: &Value, field: &str) -> Result<DependencyType, Value> {
    let s = require_str(args, field)?;
    s.parse::<DependencyType>()
        .map_err(|_| tool_error(&format!("Invalid {field}: {s}")))
}

/// Resolve a raw ID string to a ULID based on the item type.
fn resolve_id(
    dep_type: &DependencyType,
    raw_id: &str,
    db: &Database,
    default_project_id: Option<&str>,
) -> Result<String, Value> {
    let result = match dep_type {
        DependencyType::Epic => epic_db::resolve_epic_id(db, raw_id, default_project_id),
        DependencyType::Task => task_db::resolve_task_id(db, raw_id, default_project_id),
    };
    result.map_err(|e| tool_error(&e.to_string()))
}

fn parse_and_resolve_input(
    args: &Value,
    db: &Database,
    default_project_id: Option<&str>,
) -> Result<AddDependencyInput, Value> {
    let blocker_type = parse_dependency_type(args, "blocker_type")?;
    let raw_blocker_id = require_str(args, "blocker_id")?;
    let blocked_type = parse_dependency_type(args, "blocked_type")?;
    let raw_blocked_id = require_str(args, "blocked_id")?;

    let blocker_id = resolve_id(&blocker_type, &raw_blocker_id, db, default_project_id)?;
    let blocked_id = resolve_id(&blocked_type, &raw_blocked_id, db, default_project_id)?;

    Ok(AddDependencyInput {
        blocker_type,
        blocker_id,
        blocked_type,
        blocked_id,
    })
}

pub(super) fn handle_add_dependency(args: &Value, db: &Database, default_project_id: Option<&str>) -> Value {
    let input = match parse_and_resolve_input(args, db, default_project_id) {
        Ok(v) => v,
        Err(e) => return e,
    };

    match dep_db::add_dependency(db, input) {
        Ok(dep) => tool_result(&dep),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("self-referencing")
                || msg.contains("already exists")
                || msg.contains("not found")
            {
                tool_error(&msg)
            } else {
                eprintln!("add_dependency error: {e:#}");
                tool_error("Failed to add dependency")
            }
        }
    }
}

pub(super) fn handle_remove_dependency(args: &Value, db: &Database, default_project_id: Option<&str>) -> Value {
    let input = match parse_and_resolve_input(args, db, default_project_id) {
        Ok(v) => v,
        Err(e) => return e,
    };

    match dep_db::remove_dependency(
        db,
        &input.blocker_type,
        &input.blocker_id,
        &input.blocked_type,
        &input.blocked_id,
    ) {
        Ok(true) => tool_result(&json!({ "removed": true })),
        Ok(false) => tool_result(&json!({ "removed": false, "message": "Dependency not found" })),
        Err(e) => {
            eprintln!("remove_dependency error: {e:#}");
            tool_error("Failed to remove dependency")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::dispatch_tool;
    use serde_json::{json, Value};
    use tempfile::TempDir;

    use crate::db::Database;

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
            &json!({"name": "Test Project", "description": "for dep tests"}),
            db,
            None,
        )
        .unwrap();
        parse_response(&result)["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn create_test_epic(db: &Database, project_id: &str) -> String {
        let result = dispatch_tool(
            "create_epic",
            &json!({"project_id": project_id, "title": "Test Epic", "description": "for dep tests"}),
            db,
            None,
        )
        .unwrap();
        parse_response(&result)["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn create_test_task(db: &Database, epic_id: &str) -> String {
        let result = dispatch_tool(
            "create_task",
            &json!({"epic_id": epic_id, "title": "Test Task", "description": "for dep tests"}),
            db,
            None,
        )
        .unwrap();
        parse_response(&result)["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    // --- add_dependency tests ---

    #[test]
    fn test_add_dependency_between_epics() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let e1 = create_test_epic(&db, &pid);
        let e2 = create_test_epic(&db, &pid);

        let result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": e1,
                "blocked_type": "epic", "blocked_id": e2,
            }),
            &db,
            None,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let dep = parse_response(&result);
        assert_eq!(dep["blocker_type"], "epic");
        assert_eq!(dep["blocker_id"], e1);
        assert_eq!(dep["blocked_type"], "epic");
        assert_eq!(dep["blocked_id"], e2);
    }

    #[test]
    fn test_add_dependency_between_tasks() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let eid = create_test_epic(&db, &pid);
        let t1 = create_test_task(&db, &eid);
        let t2 = create_test_task(&db, &eid);

        let result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "task", "blocker_id": t1,
                "blocked_type": "task", "blocked_id": t2,
            }),
            &db,
            None,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let dep = parse_response(&result);
        assert_eq!(dep["blocker_type"], "task");
        assert_eq!(dep["blocker_id"], t1);
    }

    #[test]
    fn test_add_dependency_cross_type() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let eid = create_test_epic(&db, &pid);
        let tid = create_test_task(&db, &eid);

        let result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": eid,
                "blocked_type": "task", "blocked_id": tid,
            }),
            &db,
            None,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let dep = parse_response(&result);
        assert_eq!(dep["blocker_type"], "epic");
        assert_eq!(dep["blocked_type"], "task");
    }

    #[test]
    fn test_add_dependency_self_reference_rejected() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let eid = create_test_epic(&db, &pid);

        let result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": eid,
                "blocked_type": "epic", "blocked_id": eid,
            }),
            &db,
            None,
        )
        .unwrap();

        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("self-referencing"));
    }

    #[test]
    fn test_add_dependency_duplicate_rejected() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let e1 = create_test_epic(&db, &pid);
        let e2 = create_test_epic(&db, &pid);

        let args = json!({
            "blocker_type": "epic", "blocker_id": e1,
            "blocked_type": "epic", "blocked_id": e2,
        });

        dispatch_tool("add_dependency", &args, &db, None).unwrap();
        let result = dispatch_tool("add_dependency", &args, &db, None).unwrap();

        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("already exists"));
    }

    #[test]
    fn test_add_dependency_invalid_blocker_type() {
        let (db, _dir) = test_db();

        let result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "foo", "blocker_id": "x",
                "blocked_type": "epic", "blocked_id": "y",
            }),
            &db,
            None,
        )
        .unwrap();

        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Invalid blocker_type"));
    }

    #[test]
    fn test_add_dependency_nonexistent_blocker() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let eid = create_test_epic(&db, &pid);

        let result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": "nonexistent",
                "blocked_type": "epic", "blocked_id": eid,
            }),
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

    #[test]
    fn test_add_dependency_missing_param() {
        let (db, _dir) = test_db();

        let result = dispatch_tool(
            "add_dependency",
            &json!({"blocker_type": "epic"}),
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

    // --- remove_dependency tests ---

    #[test]
    fn test_remove_dependency_success() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let e1 = create_test_epic(&db, &pid);
        let e2 = create_test_epic(&db, &pid);

        dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": e1,
                "blocked_type": "epic", "blocked_id": e2,
            }),
            &db,
            None,
        )
        .unwrap();

        let result = dispatch_tool(
            "remove_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": e1,
                "blocked_type": "epic", "blocked_id": e2,
            }),
            &db,
            None,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["removed"], true);
    }

    #[test]
    fn test_remove_dependency_nonexistent() {
        let (db, _dir) = test_db();

        let result = dispatch_tool(
            "remove_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": "a",
                "blocked_type": "epic", "blocked_id": "b",
            }),
            &db,
            None,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["removed"], false);
        assert_eq!(data["message"], "Dependency not found");
    }

    // --- Short ID integration tests ---

    #[test]
    fn test_add_dependency_with_short_ids() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let e1 = create_test_epic(&db, &pid);
        let e2 = create_test_epic(&db, &pid);

        let result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": "E1",
                "blocked_type": "epic", "blocked_id": "E2",
            }),
            &db,
            Some(&pid),
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let dep = parse_response(&result);
        assert_eq!(dep["blocker_id"], e1);
        assert_eq!(dep["blocked_id"], e2);
    }

    #[test]
    fn test_add_dependency_with_task_short_ids() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let eid = create_test_epic(&db, &pid);
        let t1 = create_test_task(&db, &eid);
        let t2 = create_test_task(&db, &eid);

        let result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "task", "blocker_id": "E1-T1",
                "blocked_type": "task", "blocked_id": "E1-T2",
            }),
            &db,
            Some(&pid),
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let dep = parse_response(&result);
        assert_eq!(dep["blocker_id"], t1);
        assert_eq!(dep["blocked_id"], t2);
    }

    #[test]
    fn test_remove_dependency_with_short_ids() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let e1 = create_test_epic(&db, &pid);
        let e2 = create_test_epic(&db, &pid);

        // Add using ULIDs
        dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": e1,
                "blocked_type": "epic", "blocked_id": e2,
            }),
            &db,
            None,
        )
        .unwrap();

        // Remove using short IDs
        let result = dispatch_tool(
            "remove_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": "E1",
                "blocked_type": "epic", "blocked_id": "E2",
            }),
            &db,
            Some(&pid),
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["removed"], true);
    }

    #[test]
    fn test_add_dependency_mixed_short_and_ulid() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let e1 = create_test_epic(&db, &pid);
        let eid = create_test_epic(&db, &pid);
        let tid = create_test_task(&db, &eid);

        let result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "epic", "blocker_id": "E1",
                "blocked_type": "task", "blocked_id": tid,
            }),
            &db,
            Some(&pid),
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let dep = parse_response(&result);
        assert_eq!(dep["blocker_id"], e1);
        assert_eq!(dep["blocked_id"], tid);
    }

    // --- Full lifecycle test ---

    #[test]
    fn test_dependency_full_lifecycle() {
        let (db, _dir) = test_db();
        let pid = create_test_project(&db);
        let eid = create_test_epic(&db, &pid);
        let t1 = create_test_task(&db, &eid);
        let t2 = create_test_task(&db, &eid);

        // Add dependency: t1 blocks t2
        let add_result = dispatch_tool(
            "add_dependency",
            &json!({
                "blocker_type": "task", "blocker_id": t1,
                "blocked_type": "task", "blocked_id": t2,
            }),
            &db,
            None,
        )
        .unwrap();
        assert!(add_result.get("isError").is_none());

        // Verify via get_task: t2 should show t1 as a blocker
        let get_result = dispatch_tool("get_task", &json!({"id": t2}), &db, None).unwrap();
        let data = parse_response(&get_result);
        let blockers = data["blockers"].as_array().unwrap();
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0]["blocker_id"], t1);

        // Verify via get_task: t1 should show t2 in blocks
        let get_result = dispatch_tool("get_task", &json!({"id": t1}), &db, None).unwrap();
        let data = parse_response(&get_result);
        let blocks = data["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["blocked_id"], t2);

        // Remove dependency
        let remove_result = dispatch_tool(
            "remove_dependency",
            &json!({
                "blocker_type": "task", "blocker_id": t1,
                "blocked_type": "task", "blocked_id": t2,
            }),
            &db,
            None,
        )
        .unwrap();
        let removed = parse_response(&remove_result);
        assert_eq!(removed["removed"], true);

        // Verify dependency is gone
        let get_result = dispatch_tool("get_task", &json!({"id": t2}), &db, None).unwrap();
        let data = parse_response(&get_result);
        assert!(data["blockers"].as_array().unwrap().is_empty());
    }
}
