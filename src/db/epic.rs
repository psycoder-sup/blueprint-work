use anyhow::{Context, Result};
use rusqlite::{params_from_iter, OptionalExtension, Row};

use crate::db::Database;
use crate::db::resolve::{classify_id, IdKind};
use crate::models::{CreateEpicInput, Epic, ItemStatus, UpdateEpicInput};

const SELECT_COLUMNS: &str = "e.id, e.project_id, e.title, e.description, e.status, e.short_id, e.created_at, e.updated_at";
const TASK_AGGREGATES: &str =
    "COUNT(t.id) AS task_count, SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END) AS done_count";

fn row_to_epic(row: &Row) -> rusqlite::Result<Epic> {
    let status_str: String = row.get("status")?;
    let status: ItemStatus = status_str.parse().map_err(|e: anyhow::Error| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        )
    })?;

    Ok(Epic {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        title: row.get("title")?,
        description: row.get("description")?,
        status,
        short_id: row.get("short_id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        task_count: row.get("task_count")?,
        done_count: row.get("done_count")?,
    })
}

pub fn create_epic(db: &Database, input: CreateEpicInput) -> Result<Epic> {
    let id = ulid::Ulid::new().to_string();

    let tx = db
        .conn()
        .unchecked_transaction()
        .context("failed to begin transaction for epic creation")?;

    let max_num: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(CAST(SUBSTR(short_id, 2) AS INTEGER)), 0) \
             FROM epics \
             WHERE project_id = ?1 AND short_id IS NOT NULL",
            [&input.project_id],
            |row| row.get(0),
        )
        .context("failed to query next epic short_id")?;
    let short_id = format!("E{}", max_num + 1);

    tx.execute(
        "INSERT INTO epics (id, project_id, title, description, short_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        [&id, &input.project_id, &input.title, &input.description, &short_id],
    )
    .context("failed to insert epic (check that project_id is valid)")?;

    tx.commit().context("failed to commit epic creation")?;

    get_epic(db, &id)?.context("epic not found after insert")
}

pub fn get_epic(db: &Database, id: &str) -> Result<Option<Epic>> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS}, {TASK_AGGREGATES} \
         FROM epics e LEFT JOIN tasks t ON t.epic_id = e.id \
         WHERE e.id = ?1 \
         GROUP BY e.id"
    );
    db.conn()
        .prepare(&sql)?
        .query_row([id], row_to_epic)
        .optional()
        .context("failed to query epic")
}

pub fn list_epics(
    db: &Database,
    project_id: Option<&str>,
    status: Option<ItemStatus>,
) -> Result<Vec<Epic>> {
    let base = format!(
        "SELECT {SELECT_COLUMNS}, {TASK_AGGREGATES} \
         FROM epics e LEFT JOIN tasks t ON t.epic_id = e.id"
    );
    let tail = "GROUP BY e.id ORDER BY e.created_at DESC";

    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(pid) = project_id {
        params.push(Box::new(pid.to_string()));
        conditions.push(format!("e.project_id = ?{}", params.len()));
    }
    if let Some(s) = status {
        params.push(Box::new(s.as_str().to_string()));
        conditions.push(format!("e.status = ?{}", params.len()));
    }

    let sql = if conditions.is_empty() {
        format!("{base} {tail}")
    } else {
        format!("{base} WHERE {} {tail}", conditions.join(" AND "))
    };

    let mut stmt = db.conn().prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), row_to_epic)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list epics")
}

pub fn update_epic(db: &Database, id: &str, input: UpdateEpicInput) -> Result<Epic> {
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
        "UPDATE epics SET {} WHERE id = ?{}",
        set_clauses.join(", "),
        params.len(),
    );

    let rows_affected = db
        .conn()
        .execute(&sql, params_from_iter(params.iter()))
        .context("failed to update epic")?;

    if rows_affected == 0 {
        anyhow::bail!("epic not found: {id}");
    }

    get_epic(db, id)?.context("epic not found after update")
}

pub fn delete_epic(db: &Database, id: &str) -> Result<bool> {
    let tx = db
        .conn()
        .unchecked_transaction()
        .context("failed to begin transaction for epic deletion")?;

    // Clean up dependencies referencing the epic itself
    tx.execute(
        "DELETE FROM dependencies WHERE (blocker_type = 'epic' AND blocker_id = ?1) OR (blocked_type = 'epic' AND blocked_id = ?1)",
        [id],
    )
    .context("failed to clean up epic dependencies")?;

    // Clean up dependencies referencing child tasks (which will be cascade-deleted)
    tx.execute(
        "DELETE FROM dependencies WHERE (blocker_type = 'task' AND blocker_id IN (SELECT id FROM tasks WHERE epic_id = ?1)) OR (blocked_type = 'task' AND blocked_id IN (SELECT id FROM tasks WHERE epic_id = ?1))",
        [id],
    )
    .context("failed to clean up child task dependencies")?;

    let rows_affected = tx
        .execute("DELETE FROM epics WHERE id = ?1", [id])
        .context("failed to delete epic")?;

    tx.commit().context("failed to commit epic deletion")?;
    Ok(rows_affected > 0)
}

pub fn resolve_epic_id(
    db: &Database,
    id_or_short: &str,
    default_project_id: Option<&str>,
) -> Result<String> {
    match classify_id(id_or_short) {
        IdKind::Ulid => Ok(id_or_short.to_string()),
        IdKind::EpicShortId => {
            let short = id_or_short.to_uppercase();
            match default_project_id {
                Some(pid) => db
                    .conn()
                    .query_row(
                        "SELECT id FROM epics WHERE short_id = ?1 AND project_id = ?2",
                        [short.as_str(), pid],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .context("failed to resolve epic short ID")?
                    .ok_or_else(|| anyhow::anyhow!("Epic not found: {id_or_short}")),
                None => {
                    let mut stmt = db
                        .conn()
                        .prepare("SELECT id FROM epics WHERE short_id = ?1")?;
                    let ids: Vec<String> = stmt
                        .query_map([short.as_str()], |row| row.get(0))?
                        .collect::<rusqlite::Result<Vec<_>>>()
                        .context("failed to resolve epic short ID")?;
                    match ids.len() {
                        0 => anyhow::bail!("Epic not found: {id_or_short}"),
                        1 => Ok(ids.into_iter().next().unwrap()),
                        _ => anyhow::bail!(
                            "Ambiguous short ID '{}': matches {} epics across projects. \
                             Provide a default project or use the full ULID.",
                            id_or_short,
                            ids.len()
                        ),
                    }
                }
            }
        }
        IdKind::TaskShortId => {
            anyhow::bail!("Expected epic ID, got task short ID: {id_or_short}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::project::create_project;
    use crate::models::{CreateProjectInput, Project};
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
                description: "For epic tests".to_string(),
            },
        )
        .unwrap()
    }

    #[test]
    fn test_create_with_valid_project() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);

        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "My Epic".to_string(),
                description: "Epic description".to_string(),
            },
        )
        .unwrap();

        assert_eq!(epic.id.len(), 26);
        assert_eq!(epic.project_id, project.id);
        assert_eq!(epic.title, "My Epic");
        assert_eq!(epic.description, "Epic description");
        assert_eq!(epic.status, ItemStatus::Todo);
        assert_eq!(epic.short_id, Some("E1".to_string()));
        assert_eq!(epic.task_count, 0);
    }

    #[test]
    fn test_create_with_invalid_project_fails() {
        let (db, _dir) = open_temp_db();

        let result = create_epic(
            &db,
            CreateEpicInput {
                project_id: "nonexistent".to_string(),
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

        let created = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Lookup".to_string(),
                description: "desc".to_string(),
            },
        )
        .unwrap();

        let found = get_epic(&db, &created.id).unwrap().unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.title, "Lookup");

        let missing = get_epic(&db, "nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_includes_task_count() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);

        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "With Tasks".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        // Insert tasks via raw SQL
        for i in 0..3 {
            db.conn()
                .execute(
                    "INSERT INTO tasks (id, epic_id, title) VALUES (?1, ?2, ?3)",
                    [&format!("t{i}"), &epic.id, &format!("Task {i}")],
                )
                .unwrap();
        }

        let fetched = get_epic(&db, &epic.id).unwrap().unwrap();
        assert_eq!(fetched.task_count, 3);
    }

    #[test]
    fn test_list_by_project() {
        let (db, _dir) = open_temp_db();
        let p1 = create_test_project(&db);
        let p2 = create_test_project(&db);

        create_epic(
            &db,
            CreateEpicInput {
                project_id: p1.id.clone(),
                title: "Epic A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        create_epic(
            &db,
            CreateEpicInput {
                project_id: p2.id.clone(),
                title: "Epic B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let p1_epics = list_epics(&db, Some(&p1.id), None).unwrap();
        assert_eq!(p1_epics.len(), 1);
        assert_eq!(p1_epics[0].title, "Epic A");

        let p2_epics = list_epics(&db, Some(&p2.id), None).unwrap();
        assert_eq!(p2_epics.len(), 1);
        assert_eq!(p2_epics[0].title, "Epic B");
    }

    #[test]
    fn test_list_by_status() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);

        let epic_a = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "Epic A".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Epic B".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        update_epic(
            &db,
            &epic_a.id,
            UpdateEpicInput {
                status: Some(ItemStatus::InProgress),
                ..Default::default()
            },
        )
        .unwrap();

        let in_progress = list_epics(&db, None, Some(ItemStatus::InProgress)).unwrap();
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].title, "Epic A");

        let todo = list_epics(&db, None, Some(ItemStatus::Todo)).unwrap();
        assert_eq!(todo.len(), 1);
        assert_eq!(todo[0].title, "Epic B");
    }

    #[test]
    fn test_list_no_filter() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);

        for i in 0..3 {
            create_epic(
                &db,
                CreateEpicInput {
                    project_id: project.id.clone(),
                    title: format!("Epic {i}"),
                    description: String::new(),
                },
            )
            .unwrap();
        }

        let all = list_epics(&db, None, None).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_update_partial_fields() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);

        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Original".to_string(),
                description: "original desc".to_string(),
            },
        )
        .unwrap();

        let updated = update_epic(
            &db,
            &epic.id,
            UpdateEpicInput {
                title: Some("Renamed".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.title, "Renamed");
        assert_eq!(updated.description, "original desc");
        assert_eq!(updated.status, ItemStatus::Todo);
        assert!(updated.updated_at >= epic.updated_at);
    }

    #[test]
    fn test_update_nonexistent_errors() {
        let (db, _dir) = open_temp_db();

        let result = update_epic(
            &db,
            "nonexistent",
            UpdateEpicInput {
                title: Some("Name".to_string()),
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("epic not found"));
    }

    #[test]
    fn test_delete_cascades_to_tasks() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);

        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Parent Epic".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        db.conn()
            .execute(
                "INSERT INTO tasks (id, epic_id, title) VALUES ('t1', ?1, 'Child Task')",
                [&epic.id],
            )
            .unwrap();

        delete_epic(&db, &epic.id).unwrap();

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM tasks WHERE id = 't1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "task should be cascade-deleted");
    }

    #[test]
    fn test_delete_nonexistent_returns_false() {
        let (db, _dir) = open_temp_db();
        assert!(!delete_epic(&db, "nonexistent").unwrap());
    }

    #[test]
    fn test_full_lifecycle() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);

        // Create
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id,
                title: "Lifecycle".to_string(),
                description: "testing".to_string(),
            },
        )
        .unwrap();
        assert_eq!(epic.status, ItemStatus::Todo);

        // Read
        let fetched = get_epic(&db, &epic.id).unwrap().unwrap();
        assert_eq!(fetched.title, "Lifecycle");

        // Update
        let updated = update_epic(
            &db,
            &epic.id,
            UpdateEpicInput {
                title: Some("Updated".to_string()),
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.title, "Updated");
        assert_eq!(updated.status, ItemStatus::Done);

        // Delete
        assert!(delete_epic(&db, &epic.id).unwrap());
        assert!(get_epic(&db, &epic.id).unwrap().is_none());
    }

    #[test]
    fn test_create_assigns_sequential_short_ids() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);

        let make_epic = |title: &str| {
            create_epic(
                &db,
                CreateEpicInput {
                    project_id: project.id.clone(),
                    title: title.to_string(),
                    description: String::new(),
                },
            )
            .unwrap()
        };

        let e1 = make_epic("First");
        let e2 = make_epic("Second");
        let e3 = make_epic("Third");

        assert_eq!(e1.short_id, Some("E1".to_string()));
        assert_eq!(e2.short_id, Some("E2".to_string()));
        assert_eq!(e3.short_id, Some("E3".to_string()));
    }

    // --- resolve_epic_id tests ---

    #[test]
    fn test_resolve_epic_id_ulid_passthrough() {
        let (db, _dir) = open_temp_db();
        let result = resolve_epic_id(&db, "01ARZ3NDEKTSV4RRFFQ69G5FAV", None).unwrap();
        assert_eq!(result, "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    }

    #[test]
    fn test_resolve_epic_id_short_with_project() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "Test".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let resolved = resolve_epic_id(&db, "E1", Some(&project.id)).unwrap();
        assert_eq!(resolved, epic.id);
    }

    #[test]
    fn test_resolve_epic_id_short_without_project_unique() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "Test".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let resolved = resolve_epic_id(&db, "E1", None).unwrap();
        assert_eq!(resolved, epic.id);
    }

    #[test]
    fn test_resolve_epic_id_short_without_project_ambiguous() {
        let (db, _dir) = open_temp_db();
        let p1 = create_test_project(&db);
        let p2 = create_test_project(&db);
        create_epic(
            &db,
            CreateEpicInput {
                project_id: p1.id.clone(),
                title: "P1 Epic".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        create_epic(
            &db,
            CreateEpicInput {
                project_id: p2.id.clone(),
                title: "P2 Epic".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let result = resolve_epic_id(&db, "E1", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Ambiguous"));
    }

    #[test]
    fn test_resolve_epic_id_not_found() {
        let (db, _dir) = open_temp_db();
        let result = resolve_epic_id(&db, "E99", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_epic_id_task_short_id_error() {
        let (db, _dir) = open_temp_db();
        let result = resolve_epic_id(&db, "E1-T3", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("task short ID"));
    }

    #[test]
    fn test_resolve_epic_id_case_insensitive() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "Test".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let resolved = resolve_epic_id(&db, "e1", Some(&project.id)).unwrap();
        assert_eq!(resolved, epic.id);
    }

    #[test]
    fn test_short_ids_scoped_to_project() {
        let (db, _dir) = open_temp_db();
        let p1 = create_test_project(&db);
        let p2 = create_test_project(&db);

        let make_epic = |project_id: &str, title: &str| {
            create_epic(
                &db,
                CreateEpicInput {
                    project_id: project_id.to_string(),
                    title: title.to_string(),
                    description: String::new(),
                },
            )
            .unwrap()
        };

        let p1_e1 = make_epic(&p1.id, "P1 Epic");
        let p2_e1 = make_epic(&p2.id, "P2 Epic");

        assert_eq!(p1_e1.short_id, Some("E1".to_string()));
        assert_eq!(p2_e1.short_id, Some("E1".to_string()));
    }
}
