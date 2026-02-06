use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Row};

use crate::db::Database;
use crate::models::{CreatePrdInput, Prd};

const SELECT_COLUMNS: &str = "id, project_id, title, content, created_at";

fn row_to_prd(row: &Row) -> rusqlite::Result<Prd> {
    Ok(Prd {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        title: row.get("title")?,
        content: row.get("content")?,
        created_at: row.get("created_at")?,
    })
}

pub fn create_prd(db: &Database, input: CreatePrdInput) -> Result<Prd> {
    let id = ulid::Ulid::new().to_string();
    db.conn()
        .execute(
            "INSERT INTO prds (id, project_id, title, content) VALUES (?1, ?2, ?3, ?4)",
            [&id, &input.project_id, &input.title, &input.content],
        )
        .context("failed to insert prd")?;

    get_prd(db, &id)?.context("prd not found after insert")
}

pub fn get_prd(db: &Database, id: &str) -> Result<Option<Prd>> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM prds WHERE id = ?1");
    let prd = db
        .conn()
        .prepare(&sql)?
        .query_row([id], row_to_prd)
        .optional()
        .context("failed to query prd")?;

    Ok(prd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::project::create_project;
    use crate::models::CreateProjectInput;
    use tempfile::TempDir;

    fn open_temp_db() -> (Database, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).unwrap();
        db.migrate().unwrap();
        (db, dir)
    }

    #[test]
    fn test_create_prd_valid() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Test Project".to_string(),
                description: "desc".to_string(),
            },
        )
        .unwrap();

        let prd = create_prd(
            &db,
            CreatePrdInput {
                project_id: project.id.clone(),
                title: "My PRD".to_string(),
                content: "# Requirements\n\nSome content".to_string(),
            },
        )
        .unwrap();

        assert_eq!(prd.id.len(), 26); // ULID
        assert_eq!(prd.project_id, project.id);
        assert_eq!(prd.title, "My PRD");
        assert_eq!(prd.content, "# Requirements\n\nSome content");
    }

    #[test]
    fn test_create_prd_invalid_project_fk() {
        let (db, _dir) = open_temp_db();
        let result = create_prd(
            &db,
            CreatePrdInput {
                project_id: "nonexistent".to_string(),
                title: "Bad PRD".to_string(),
                content: "content".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_prd_by_id() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Proj".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let created = create_prd(
            &db,
            CreatePrdInput {
                project_id: project.id.clone(),
                title: "Lookup PRD".to_string(),
                content: "content".to_string(),
            },
        )
        .unwrap();

        let found = get_prd(&db, &created.id).unwrap().unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.title, "Lookup PRD");

        let missing = get_prd(&db, "nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_prd_cascade_deletes_with_project() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "Parent".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let prd = create_prd(
            &db,
            CreatePrdInput {
                project_id: project.id.clone(),
                title: "PRD".to_string(),
                content: "content".to_string(),
            },
        )
        .unwrap();

        crate::db::project::delete_project(&db, &project.id).unwrap();
        assert!(get_prd(&db, &prd.id).unwrap().is_none());
    }
}
