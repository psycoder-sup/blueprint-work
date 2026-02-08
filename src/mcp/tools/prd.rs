use serde_json::{json, Value};

use crate::db::prd as prd_db;
use crate::db::project as project_db;
use crate::db::Database;
use crate::models::prd::CreatePrdInput;

use super::{optional_str, require_str, tool_error, tool_result};

pub(super) fn handle_feed_prd(
    args: &Value,
    db: &Database,
    default_project_id: Option<&str>,
) -> Value {
    let project_id = match optional_str(args, "project_id")
        .or_else(|| default_project_id.map(String::from))
    {
        Some(v) => v,
        None => return tool_error("Missing required parameter: project_id"),
    };
    let title = match require_str(args, "title") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let content = match require_str(args, "content") {
        Ok(v) => v,
        Err(e) => return e,
    };

    // Validate project exists
    match project_db::get_project(db, &project_id) {
        Ok(Some(_)) => {}
        Ok(None) => return tool_error(&format!("Project not found: {project_id}")),
        Err(e) => {
            eprintln!("feed_prd error: {e:#}");
            return tool_error("Failed to validate project");
        }
    }

    let prd = match prd_db::create_prd(
        db,
        CreatePrdInput {
            project_id: project_id.clone(),
            title,
            content,
        },
    ) {
        Ok(prd) => prd,
        Err(e) => {
            eprintln!("feed_prd error: {e:#}");
            return tool_error("Failed to store PRD");
        }
    };

    let guide = format!(
        "PRD stored successfully. Now break it down:\n\
         1. Analyze the PRD content above\n\
         2. Create epics using `create_epic` with project_id=\"{project_id}\"\n\
         3. Create tasks under each epic using `create_task`\n\
         4. Set up dependencies between tasks/epics using `add_dependency`\n\
         5. Use `get_status` with project_id=\"{project_id}\" to verify the breakdown"
    );

    tool_result(&json!({
        "message": "PRD stored successfully",
        "prd_id": prd.id,
        "guide": guide,
    }))
}

#[cfg(test)]
mod tests {
    use super::super::dispatch_tool;
    use crate::db::prd::get_prd;
    use crate::db::project::create_project;
    use crate::db::Database;
    use crate::models::CreateProjectInput;
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
    fn test_feed_prd_success() {
        let (db, _dir) = test_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "My Project".to_string(),
                description: "desc".to_string(),
            },
        )
        .unwrap();

        let result = dispatch_tool(
            "feed_prd",
            &json!({
                "project_id": project.id,
                "title": "Feature PRD",
                "content": "# Requirements\n\nBuild a widget"
            }),
            &db,
            None,
        )
        .unwrap();

        assert!(result.get("isError").is_none());
        let data = parse_response(&result);
        assert_eq!(data["message"], "PRD stored successfully");
        assert_eq!(data["prd_id"].as_str().unwrap().len(), 26);
        assert!(data["guide"].as_str().unwrap().contains("create_epic"));
        assert!(data["guide"]
            .as_str()
            .unwrap()
            .contains(&project.id));
    }

    #[test]
    fn test_feed_prd_invalid_project() {
        let (db, _dir) = test_db();
        let result = dispatch_tool(
            "feed_prd",
            &json!({
                "project_id": "nonexistent",
                "title": "PRD",
                "content": "content"
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
    fn test_feed_prd_missing_params() {
        let (db, _dir) = test_db();

        // Missing project_id
        let result = dispatch_tool(
            "feed_prd",
            &json!({"title": "T", "content": "C"}),
            &db,
            None,
        )
        .unwrap();
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter"));

        // Missing title
        let result = dispatch_tool(
            "feed_prd",
            &json!({"project_id": "p1", "content": "C"}),
            &db,
            None,
        )
        .unwrap();
        assert_eq!(result["isError"], true);

        // Missing content
        let result = dispatch_tool(
            "feed_prd",
            &json!({"project_id": "p1", "title": "T"}),
            &db,
            None,
        )
        .unwrap();
        assert_eq!(result["isError"], true);
    }

    #[test]
    fn test_feed_prd_verifies_db_storage() {
        let (db, _dir) = test_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Storage Test".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let result = dispatch_tool(
            "feed_prd",
            &json!({
                "project_id": project.id,
                "title": "Stored PRD",
                "content": "This should be in the DB"
            }),
            &db,
            None,
        )
        .unwrap();

        let data = parse_response(&result);
        let prd_id = data["prd_id"].as_str().unwrap();

        // Verify directly via DB
        let prd = get_prd(&db, prd_id).unwrap().unwrap();
        assert_eq!(prd.title, "Stored PRD");
        assert_eq!(prd.content, "This should be in the DB");
        assert_eq!(prd.project_id, project.id);
    }
}
