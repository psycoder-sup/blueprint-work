use anyhow::{Context, Result};
use rusqlite::{params_from_iter, OptionalExtension, Row};

use crate::db::Database;
use crate::db::resolve::{classify_id, IdKind};
use crate::models::{BlueTask, CreateTaskInput, ItemStatus, UpdateTaskInput};

const SELECT_COLUMNS: &str = "id, epic_id, title, description, status, short_id, created_at, updated_at";

fn row_to_task(row: &Row) -> rusqlite::Result<BlueTask> {
    let status_str: String = row.get("status")?;
    let status: ItemStatus = status_str.parse().map_err(|e: anyhow::Error| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        )
    })?;

    Ok(BlueTask {
        id: row.get("id")?,
        epic_id: row.get("epic_id")?,
        title: row.get("title")?,
        description: row.get("description")?,
        status,
        short_id: row.get("short_id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn create_task(db: &Database, input: CreateTaskInput) -> Result<BlueTask> {
    let id = ulid::Ulid::new().to_string();

    let tx = db
        .conn()
        .unchecked_transaction()
        .context("failed to begin transaction for task creation")?;

    let epic_short_id: String = tx
        .query_row(
            "SELECT short_id FROM epics WHERE id = ?1",
            [&input.epic_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .context("failed to get epic short_id (check that epic_id is valid)")?
        .context("epic has no short_id assigned")?;

    let max_num: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(CAST(SUBSTR(short_id, INSTR(short_id, '-T') + 2) AS INTEGER)), 0) \
             FROM tasks \
             WHERE epic_id = ?1 AND short_id IS NOT NULL",
            [&input.epic_id],
            |row| row.get(0),
        )
        .context("failed to query next task short_id")?;
    let short_id = format!("{epic_short_id}-T{}", max_num + 1);

    tx.execute(
        "INSERT INTO tasks (id, epic_id, title, description, short_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        [&id, &input.epic_id, &input.title, &input.description, &short_id],
    )
    .context("failed to insert task (check that epic_id is valid)")?;

    tx.commit().context("failed to commit task creation")?;

    super::epic::sync_epic_status(db, &input.epic_id)?;

    get_task(db, &id)?.context("task not found after insert")
}

pub fn get_task(db: &Database, id: &str) -> Result<Option<BlueTask>> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM tasks WHERE id = ?1");
    db.conn()
        .prepare(&sql)?
        .query_row([id], row_to_task)
        .optional()
        .context("failed to query task")
}

pub fn list_tasks(
    db: &Database,
    epic_id: Option<&str>,
    status: Option<ItemStatus>,
) -> Result<Vec<BlueTask>> {
    let base = format!("SELECT {SELECT_COLUMNS} FROM tasks");
    let tail = "ORDER BY created_at DESC";

    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(eid) = epic_id {
        params.push(Box::new(eid.to_string()));
        conditions.push(format!("epic_id = ?{}", params.len()));
    }
    if let Some(s) = status {
        params.push(Box::new(s.as_str().to_string()));
        conditions.push(format!("status = ?{}", params.len()));
    }

    let sql = if conditions.is_empty() {
        format!("{base} {tail}")
    } else {
        format!("{base} WHERE {} {tail}", conditions.join(" AND "))
    };

    let mut stmt = db.conn().prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), row_to_task)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list tasks")
}

pub fn update_task(db: &Database, id: &str, input: UpdateTaskInput) -> Result<BlueTask> {
    let status_changed = input.status.is_some();

    let mut set_clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    let mut bind = |column: &str, value: Box<dyn rusqlite::types::ToSql>| {
        params.push(value);
        set_clauses.push(format!("\"{column}\" = ?{}", params.len()));
    };

    if let Some(title) = input.title {
        bind("title", Box::new(title));
    }
    if let Some(description) = input.description {
        bind("description", Box::new(description));
    }
    if let Some(status) = input.status {
        bind("status", Box::new(status.as_str().to_string()));
    }

    set_clauses.push("updated_at = datetime('now')".to_string());
    params.push(Box::new(id.to_string()));

    let sql = format!(
        "UPDATE tasks SET {} WHERE id = ?{}",
        set_clauses.join(", "),
        params.len(),
    );

    let rows_affected = db
        .conn()
        .execute(&sql, params_from_iter(params.iter()))
        .context("failed to update task")?;

    if rows_affected == 0 {
        anyhow::bail!("task not found: {id}");
    }

    let task = get_task(db, id)?.context("task not found after update")?;

    if status_changed {
        super::epic::sync_epic_status(db, &task.epic_id)?;
    }

    Ok(task)
}

pub fn delete_task(db: &Database, id: &str) -> Result<bool> {
    // Fetch epic_id before deletion so we can sync the epic afterwards
    let epic_id: Option<String> = db
        .conn()
        .query_row(
            "SELECT epic_id FROM tasks WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .context("failed to fetch task epic_id before deletion")?;

    let tx = db
        .conn()
        .unchecked_transaction()
        .context("failed to begin transaction for task deletion")?;

    // Clean up polymorphic dependency rows (no FK cascade for these)
    tx.execute(
        "DELETE FROM dependencies WHERE (blocker_type = 'task' AND blocker_id = ?1) OR (blocked_type = 'task' AND blocked_id = ?1)",
        [id],
    )
    .context("failed to clean up dependencies for task")?;

    let rows_affected = tx
        .execute("DELETE FROM tasks WHERE id = ?1", [id])
        .context("failed to delete task")?;

    tx.commit().context("failed to commit task deletion")?;

    let deleted = rows_affected > 0;
    if let (true, Some(eid)) = (deleted, epic_id) {
        super::epic::sync_epic_status(db, &eid)?;
    }

    Ok(deleted)
}

pub fn resolve_task_id(
    db: &Database,
    id_or_short: &str,
    default_project_id: Option<&str>,
) -> Result<String> {
    match classify_id(id_or_short) {
        IdKind::Ulid => Ok(id_or_short.to_string()),
        IdKind::TaskShortId => {
            let upper = id_or_short.to_uppercase();
            let dash_pos = upper.find("-T").expect("classify_id guaranteed -T present");
            let epic_short = &upper[..dash_pos];
            let epic_id =
                super::epic::resolve_epic_id(db, epic_short, default_project_id)?;
            db.conn()
                .query_row(
                    "SELECT id FROM tasks WHERE short_id = ?1 AND epic_id = ?2",
                    [upper.as_str(), epic_id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .context("failed to resolve task short ID")?
                .ok_or_else(|| anyhow::anyhow!("Task not found: {id_or_short}"))
        }
        IdKind::EpicShortId => {
            anyhow::bail!("Expected task ID, got epic short ID: {id_or_short}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::epic::{create_epic, get_epic};
    use crate::db::project::create_project;
    use crate::models::{CreateEpicInput, CreateProjectInput, Epic, Project};
    use tempfile::TempDir;

    fn open_temp_db() -> (Database, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).unwrap();
        db.migrate().unwrap();
        (db, dir)
    }

    fn create_test_project(db: &Database) -> Project {
        create_project(
            db,
            CreateProjectInput {
                name: "Test Project".to_string(),
                description: "For task tests".to_string(),
            },
        )
        .unwrap()
    }

    fn create_test_epic(db: &Database, project_id: &str) -> Epic {
        create_epic(
            db,
            CreateEpicInput {
                project_id: project_id.to_string(),
                title: "Test Epic".to_string(),
                description: "For task tests".to_string(),
            },
        )
        .unwrap()
    }

    #[test]
    fn test_create_with_valid_epic() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        let task = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "My Task".to_string(),
                description: "Task description".to_string(),
            },
        )
        .unwrap();

        assert_eq!(task.id.len(), 26);
        assert_eq!(task.epic_id, epic.id);
        assert_eq!(task.title, "My Task");
        assert_eq!(task.description, "Task description");
        assert_eq!(task.status, ItemStatus::Todo);
        assert_eq!(task.short_id, Some("E1-T1".to_string()));
    }

    #[test]
    fn test_create_with_invalid_epic_fails() {
        let (db, _dir) = open_temp_db();

        let result = create_task(
            &db,
            CreateTaskInput {
                epic_id: "nonexistent".to_string(),
                title: "Orphan".to_string(),
                description: String::new(),
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_get_by_id() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        let created = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id,
                title: "Lookup".to_string(),
                description: "desc".to_string(),
            },
        )
        .unwrap();

        let found = get_task(&db, &created.id).unwrap().unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.title, "Lookup");

        let missing = get_task(&db, "nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_list_by_epic() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let e1 = create_test_epic(&db, &project.id);
        let e2 = create_test_epic(&db, &project.id);

        create_task(
            &db,
            CreateTaskInput {
                epic_id: e1.id.clone(),
                title: "Task A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        create_task(
            &db,
            CreateTaskInput {
                epic_id: e2.id.clone(),
                title: "Task B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let e1_tasks = list_tasks(&db, Some(&e1.id), None).unwrap();
        assert_eq!(e1_tasks.len(), 1);
        assert_eq!(e1_tasks[0].title, "Task A");

        let e2_tasks = list_tasks(&db, Some(&e2.id), None).unwrap();
        assert_eq!(e2_tasks.len(), 1);
        assert_eq!(e2_tasks[0].title, "Task B");
    }

    #[test]
    fn test_list_by_status() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        let task_a = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Task A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id,
                title: "Task B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        update_task(
            &db,
            &task_a.id,
            UpdateTaskInput {
                status: Some(ItemStatus::InProgress),
                ..Default::default()
            },
        )
        .unwrap();

        let in_progress = list_tasks(&db, None, Some(ItemStatus::InProgress)).unwrap();
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].title, "Task A");

        let todo = list_tasks(&db, None, Some(ItemStatus::Todo)).unwrap();
        assert_eq!(todo.len(), 1);
        assert_eq!(todo[0].title, "Task B");
    }

    #[test]
    fn test_list_no_filter() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        for i in 0..3 {
            create_task(
                &db,
                CreateTaskInput {
                    epic_id: epic.id.clone(),
                    title: format!("Task {i}"),
                    description: String::new(),
                },
            )
            .unwrap();
        }

        let all = list_tasks(&db, None, None).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_update_partial_fields() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        let task = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id,
                title: "Original".to_string(),
                description: "original desc".to_string(),
            },
        )
        .unwrap();

        let updated = update_task(
            &db,
            &task.id,
            UpdateTaskInput {
                title: Some("Renamed".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.title, "Renamed");
        assert_eq!(updated.description, "original desc");
        assert_eq!(updated.status, ItemStatus::Todo);
        assert!(updated.updated_at >= task.updated_at);
    }

    #[test]
    fn test_update_nonexistent_errors() {
        let (db, _dir) = open_temp_db();

        let result = update_task(
            &db,
            "nonexistent",
            UpdateTaskInput {
                title: Some("Name".to_string()),
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("task not found"));
    }

    #[test]
    fn test_delete_cleans_up_dependencies() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        let task = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id,
                title: "Blocker Task".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        // Insert dependency rows via raw SQL
        db.conn()
            .execute(
                "INSERT INTO dependencies (blocker_type, blocker_id, blocked_type, blocked_id) VALUES ('task', ?1, 'task', 'other_task')",
                [&task.id],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO dependencies (blocker_type, blocker_id, blocked_type, blocked_id) VALUES ('task', 'other_task', 'task', ?1)",
                [&task.id],
            )
            .unwrap();

        assert!(delete_task(&db, &task.id).unwrap());

        let dep_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM dependencies WHERE blocker_id = ?1 OR blocked_id = ?1",
                [&task.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dep_count, 0, "dependencies should be cleaned up");
    }

    #[test]
    fn test_delete_nonexistent_returns_false() {
        let (db, _dir) = open_temp_db();
        assert!(!delete_task(&db, "nonexistent").unwrap());
    }

    #[test]
    fn test_full_lifecycle() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        // Create
        let task = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id,
                title: "Lifecycle".to_string(),
                description: "testing".to_string(),
            },
        )
        .unwrap();
        assert_eq!(task.status, ItemStatus::Todo);

        // Read
        let fetched = get_task(&db, &task.id).unwrap().unwrap();
        assert_eq!(fetched.title, "Lifecycle");

        // Update
        let updated = update_task(
            &db,
            &task.id,
            UpdateTaskInput {
                title: Some("Updated".to_string()),
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.title, "Updated");
        assert_eq!(updated.status, ItemStatus::Done);

        // Delete
        assert!(delete_task(&db, &task.id).unwrap());
        assert!(get_task(&db, &task.id).unwrap().is_none());
    }

    #[test]
    fn test_create_assigns_sequential_short_ids() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        let make_task = |title: &str| {
            create_task(
                &db,
                CreateTaskInput {
                    epic_id: epic.id.clone(),
                    title: title.to_string(),
                    description: String::new(),
                },
            )
            .unwrap()
        };

        let t1 = make_task("First");
        let t2 = make_task("Second");
        let t3 = make_task("Third");

        assert_eq!(t1.short_id, Some("E1-T1".to_string()));
        assert_eq!(t2.short_id, Some("E1-T2".to_string()));
        assert_eq!(t3.short_id, Some("E1-T3".to_string()));
    }

    // --- resolve_task_id tests ---

    #[test]
    fn test_resolve_task_id_ulid_passthrough() {
        let (db, _dir) = open_temp_db();
        let result = resolve_task_id(&db, "01ARZ3NDEKTSV4RRFFQ69G5FAV", None).unwrap();
        assert_eq!(result, "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    }

    #[test]
    fn test_resolve_task_id_short_two_step() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let task = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Test Task".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let resolved = resolve_task_id(&db, "E1-T1", Some(&project.id)).unwrap();
        assert_eq!(resolved, task.id);
    }

    #[test]
    fn test_resolve_task_id_epic_not_found() {
        let (db, _dir) = open_temp_db();
        let result = resolve_task_id(&db, "E99-T1", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_task_id_task_not_found() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        create_test_epic(&db, &project.id);

        let result = resolve_task_id(&db, "E1-T99", Some(&project.id));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_task_id_epic_short_id_error() {
        let (db, _dir) = open_temp_db();
        let result = resolve_task_id(&db, "E1", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("epic short ID"));
    }

    #[test]
    fn test_resolve_task_id_case_insensitive() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let task = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Test Task".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let resolved = resolve_task_id(&db, "e1-t1", Some(&project.id)).unwrap();
        assert_eq!(resolved, task.id);
    }

    // --- sync_epic_status integration tests ---

    /// Create a project, epic, and two tasks for epic-sync integration tests.
    fn sync_fixture_with_two_tasks() -> (Database, TempDir, Epic, BlueTask, BlueTask) {
        let (db, dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Task 1".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Task 2".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        (db, dir, epic, t1, t2)
    }

    #[test]
    fn test_update_task_syncs_epic_status() {
        let (db, _dir, epic, t1, t2) = sync_fixture_with_two_tasks();

        // Mark first task done -> epic should be in_progress
        update_task(
            &db,
            &t1.id,
            UpdateTaskInput {
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();
        let e = get_epic(&db, &epic.id).unwrap().unwrap();
        assert_eq!(e.status, ItemStatus::InProgress);

        // Mark second task done -> epic should be done
        update_task(
            &db,
            &t2.id,
            UpdateTaskInput {
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();
        let e = get_epic(&db, &epic.id).unwrap().unwrap();
        assert_eq!(e.status, ItemStatus::Done);
    }

    #[test]
    fn test_delete_task_syncs_epic_status() {
        let (db, _dir, epic, t1, t2) = sync_fixture_with_two_tasks();

        // Mark t1 done, t2 stays todo -> epic in_progress
        update_task(
            &db,
            &t1.id,
            UpdateTaskInput {
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();
        let e = get_epic(&db, &epic.id).unwrap().unwrap();
        assert_eq!(e.status, ItemStatus::InProgress);

        // Delete the non-done task -> only done tasks remain -> epic done
        delete_task(&db, &t2.id).unwrap();
        let e = get_epic(&db, &epic.id).unwrap().unwrap();
        assert_eq!(e.status, ItemStatus::Done);
    }

    #[test]
    fn test_create_task_syncs_epic_status() {
        use crate::db::epic::update_epic;
        use crate::models::UpdateEpicInput;

        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        // Manually set epic to done
        update_epic(
            &db,
            &epic.id,
            UpdateEpicInput {
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();

        // Creating a new (todo) task should revert a done epic
        create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "New Task".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let e = get_epic(&db, &epic.id).unwrap().unwrap();
        assert_eq!(e.status, ItemStatus::Todo);
    }

    #[test]
    fn test_short_ids_scoped_to_epic() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let e1 = create_test_epic(&db, &project.id);
        let e2 = create_test_epic(&db, &project.id);

        let make_task = |epic_id: &str, title: &str| {
            create_task(
                &db,
                CreateTaskInput {
                    epic_id: epic_id.to_string(),
                    title: title.to_string(),
                    description: String::new(),
                },
            )
            .unwrap()
        };

        let e1_t1 = make_task(&e1.id, "E1 Task");
        let e2_t1 = make_task(&e2.id, "E2 Task");

        assert_eq!(e1_t1.short_id, Some("E1-T1".to_string()));
        assert_eq!(e2_t1.short_id, Some("E2-T1".to_string()));
    }
}
