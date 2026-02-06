use anyhow::{Context, Result};
use rusqlite::{params_from_iter, OptionalExtension, Row};

use crate::db::Database;
use crate::models::{CreateProjectInput, Project, ProjectStatus, UpdateProjectInput};

const SELECT_COLUMNS: &str = "id, name, description, status, created_at, updated_at";

fn row_to_project(row: &Row) -> rusqlite::Result<Project> {
    let status_str: String = row.get("status")?;
    let status: ProjectStatus = status_str.parse().map_err(|e: anyhow::Error| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        )
    })?;

    Ok(Project {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        status,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn create_project(db: &Database, input: CreateProjectInput) -> Result<Project> {
    let id = ulid::Ulid::new().to_string();
    db.conn()
        .execute(
            "INSERT INTO projects (id, name, description) VALUES (?1, ?2, ?3)",
            [&id, &input.name, &input.description],
        )
        .context("failed to insert project")?;

    get_project(db, &id)?.context("project not found after insert")
}

pub fn get_project(db: &Database, id: &str) -> Result<Option<Project>> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM projects WHERE id = ?1");
    let project = db
        .conn()
        .prepare(&sql)?
        .query_row([id], row_to_project)
        .optional()
        .context("failed to query project")?;

    Ok(project)
}

pub fn list_projects(
    db: &Database,
    status: Option<ProjectStatus>,
) -> Result<Vec<Project>> {
    let base = format!("SELECT {SELECT_COLUMNS} FROM projects");
    let sql = match &status {
        Some(_) => format!("{base} WHERE status = ?1 ORDER BY created_at DESC"),
        None => format!("{base} ORDER BY created_at DESC"),
    };

    let mut stmt = db.conn().prepare(&sql)?;
    let rows = match &status {
        Some(s) => stmt.query_map([s.as_str()], row_to_project)?,
        None => stmt.query_map([], row_to_project)?,
    };

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list projects")
}

pub fn update_project(db: &Database, id: &str, input: UpdateProjectInput) -> Result<Project> {
    let mut set_clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    let mut bind = |column: &str, value: Box<dyn rusqlite::types::ToSql>| {
        params.push(value);
        set_clauses.push(format!("\"{column}\" = ?{}", params.len()));
    };

    if let Some(name) = input.name {
        bind("name", Box::new(name));
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
        "UPDATE projects SET {} WHERE id = ?{}",
        set_clauses.join(", "),
        params.len(),
    );

    let rows_affected = db
        .conn()
        .execute(&sql, params_from_iter(params.iter()))
        .context("failed to update project")?;

    if rows_affected == 0 {
        anyhow::bail!("project not found: {id}");
    }

    get_project(db, id)?.context("project not found after update")
}

pub fn delete_project(db: &Database, id: &str) -> Result<bool> {
    let rows_affected = db
        .conn()
        .execute("DELETE FROM projects WHERE id = ?1", [id])
        .context("failed to delete project")?;

    Ok(rows_affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_temp_db() -> (Database, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).unwrap();
        db.migrate().unwrap();
        (db, dir)
    }

    #[test]
    fn test_create_returns_valid_ulid() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Test Project".to_string(),
                description: "A description".to_string(),
            },
        )
        .unwrap();

        assert_eq!(project.id.len(), 26);
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description, "A description");
        assert_eq!(project.status, ProjectStatus::Active);
    }

    #[test]
    fn test_get_by_id() {
        let (db, _dir) = open_temp_db();
        let created = create_project(
            &db,
            CreateProjectInput {
                name: "Lookup".to_string(),
                description: "desc".to_string(),
            },
        )
        .unwrap();

        let found = get_project(&db, &created.id).unwrap().unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.name, "Lookup");

        let missing = get_project(&db, "nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_list_without_filter() {
        let (db, _dir) = open_temp_db();
        for i in 0..3 {
            create_project(
                &db,
                CreateProjectInput {
                    name: format!("Project {i}"),
                    description: String::new(),
                },
            )
            .unwrap();
        }

        let all = list_projects(&db, None).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_list_with_status_filter() {
        let (db, _dir) = open_temp_db();
        let p1 = create_project(
            &db,
            CreateProjectInput {
                name: "Active".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let p2 = create_project(
            &db,
            CreateProjectInput {
                name: "To Archive".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        update_project(
            &db,
            &p2.id,
            UpdateProjectInput {
                status: Some(ProjectStatus::Archived),
                ..Default::default()
            },
        )
        .unwrap();

        let active = list_projects(&db, Some(ProjectStatus::Active)).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, p1.id);

        let archived = list_projects(&db, Some(ProjectStatus::Archived)).unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].id, p2.id);
    }

    #[test]
    fn test_update_partial_fields() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Original".to_string(),
                description: "original desc".to_string(),
            },
        )
        .unwrap();

        let updated = update_project(
            &db,
            &project.id,
            UpdateProjectInput {
                name: Some("Renamed".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.description, "original desc");
        assert_eq!(updated.status, ProjectStatus::Active);
        assert!(updated.updated_at >= project.updated_at);
    }

    #[test]
    fn test_update_nonexistent_errors() {
        let (db, _dir) = open_temp_db();
        let result = update_project(
            &db,
            "nonexistent",
            UpdateProjectInput {
                name: Some("Name".to_string()),
                ..Default::default()
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("project not found"));
    }

    #[test]
    fn test_delete_returns_true() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "To Delete".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        assert!(delete_project(&db, &project.id).unwrap());
        assert!(get_project(&db, &project.id).unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_returns_false() {
        let (db, _dir) = open_temp_db();
        assert!(!delete_project(&db, "nonexistent").unwrap());
    }

    #[test]
    fn test_delete_cascades() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Parent".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        db.conn()
            .execute(
                "INSERT INTO epics (id, project_id, title) VALUES ('e1', ?1, 'Child Epic')",
                [&project.id],
            )
            .unwrap();

        delete_project(&db, &project.id).unwrap();

        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM epics WHERE id = 'e1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "epic should be cascade-deleted");
    }

    #[test]
    fn test_full_lifecycle() {
        let (db, _dir) = open_temp_db();

        // Create
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Lifecycle".to_string(),
                description: "testing".to_string(),
            },
        )
        .unwrap();
        assert_eq!(project.status, ProjectStatus::Active);

        // Read
        let fetched = get_project(&db, &project.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Lifecycle");

        // Update
        let updated = update_project(
            &db,
            &project.id,
            UpdateProjectInput {
                name: Some("Updated".to_string()),
                status: Some(ProjectStatus::Archived),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.status, ProjectStatus::Archived);

        // Delete
        assert!(delete_project(&db, &project.id).unwrap());
        assert!(get_project(&db, &project.id).unwrap().is_none());
    }
}
