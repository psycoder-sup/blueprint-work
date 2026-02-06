use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

const MIGRATION: &str = include_str!("../../migrations/001_init.sql");

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open a database at the given path, creating parent directories as needed.
    /// Enables WAL mode and foreign key enforcement.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("failed to open database at {}", path.display()))?;

        let mode: String =
            conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        if mode != "wal" {
            anyhow::bail!("failed to enable WAL mode, got: {mode}");
        }
        conn.pragma_update(None, "foreign_keys", "ON")?;

        Ok(Self { conn })
    }

    /// Open the database at the default location.
    /// Uses `BLUEPRINT_DB` env var if set, otherwise `~/.blueprint/blueprint.db`.
    pub fn open_default() -> Result<Self> {
        let path = match std::env::var("BLUEPRINT_DB") {
            Ok(p) => PathBuf::from(p),
            Err(_) => {
                let home = std::env::var("HOME").context("HOME environment variable not set")?;
                PathBuf::from(home).join(".blueprint").join("blueprint.db")
            }
        };
        Self::open(&path)
    }

    /// Run all migrations. Idempotent thanks to `IF NOT EXISTS` clauses.
    pub fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(MIGRATION)
            .context("failed to run database migration")?;
        Ok(())
    }

    /// Access the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
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
    fn test_open_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub").join("test.db");
        let _db = Database::open(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_migrate_creates_tables() {
        let (db, _dir) = open_temp_db();
        let tables: Vec<String> = db
            .conn()
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(tables, ["dependencies", "epics", "prds", "projects", "tasks"]);
    }

    #[test]
    fn test_migrate_creates_indexes() {
        let (db, _dir) = open_temp_db();
        let indexes: Vec<String> = db
            .conn()
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            indexes,
            [
                "idx_deps_blocked",
                "idx_deps_blocker",
                "idx_epics_project_id",
                "idx_epics_status",
                "idx_prds_project_id",
                "idx_tasks_epic_id",
                "idx_tasks_status",
            ]
        );
    }

    #[test]
    fn test_migrate_idempotent() {
        let (db, _dir) = open_temp_db();
        db.migrate().unwrap(); // second run should succeed
    }

    #[test]
    fn test_foreign_keys_enforced() {
        let (db, _dir) = open_temp_db();
        let result = db.conn().execute(
            "INSERT INTO epics (id, project_id, title) VALUES ('e1', 'nonexistent', 'Test')",
            [],
        );
        assert!(result.is_err(), "should reject invalid foreign key");
    }

    #[test]
    fn test_cascade_delete() {
        let (db, _dir) = open_temp_db();
        db.conn()
            .execute("INSERT INTO projects (id, name) VALUES ('p1', 'Proj')", [])
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO epics (id, project_id, title) VALUES ('e1', 'p1', 'Epic')",
                [],
            )
            .unwrap();
        db.conn()
            .execute("DELETE FROM projects WHERE id = 'p1'", [])
            .unwrap();
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM epics WHERE id = 'e1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "epic should be cascade-deleted");
    }

    #[test]
    fn test_wal_mode_enabled() {
        let (db, _dir) = open_temp_db();
        let mode: String = db
            .conn()
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }
}
