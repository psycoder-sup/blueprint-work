use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

const MIGRATIONS: &[(i32, &str)] = &[
    (1, include_str!("../../migrations/001_init.sql")),
    (2, include_str!("../../migrations/002_short_ids.sql")),
];

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

    /// Run all pending migrations. Uses a `_schema_version` table to track
    /// which migrations have been applied, and only runs new ones.
    pub fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS _schema_version (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
                )",
            )
            .context("failed to create _schema_version table")?;

        let current_version: i32 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _schema_version",
                [],
                |row| row.get(0),
            )
            .context("failed to query schema version")?;

        for &(version, sql) in MIGRATIONS {
            if version <= current_version {
                continue;
            }

            let tx = self
                .conn
                .unchecked_transaction()
                .with_context(|| format!("failed to begin transaction for migration {version}"))?;

            tx.execute_batch(sql)
                .with_context(|| format!("failed to run migration {version}"))?;

            tx.execute(
                "INSERT INTO _schema_version (version) VALUES (?1)",
                [version],
            )
            .with_context(|| format!("failed to record migration {version}"))?;

            tx.commit()
                .with_context(|| format!("failed to commit migration {version}"))?;
        }

        Ok(())
    }

    /// Access the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

pub mod dependency;
pub mod epic;
pub mod prd;
pub mod project;
pub(crate) mod resolve;
pub mod status;
pub mod task;

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

    /// Apply a single raw migration SQL and record its version.
    /// Mirrors the per-migration transaction logic in `Database::migrate`.
    fn apply_raw_migration(db: &Database, version: i32, sql: &str) {
        db.conn()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS _schema_version (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
                )",
            )
            .unwrap();
        let tx = db.conn().unchecked_transaction().unwrap();
        tx.execute_batch(sql).unwrap();
        tx.execute(
            "INSERT INTO _schema_version (version) VALUES (?1)",
            [version],
        )
        .unwrap();
        tx.commit().unwrap();
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
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_schema_version' ORDER BY name")
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
                "idx_epics_short_id",
                "idx_epics_status",
                "idx_prds_project_id",
                "idx_tasks_epic_id",
                "idx_tasks_short_id",
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
    fn test_migrate_tracks_schema_version() {
        let (db, _dir) = open_temp_db();
        let version: i32 = db
            .conn()
            .query_row("SELECT MAX(version) FROM _schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn test_migrate_backfills_short_ids() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).unwrap();

        // Run only migration 1 manually to simulate a pre-existing database
        db.conn()
            .execute_batch(include_str!("../../migrations/001_init.sql"))
            .unwrap();

        // Insert test data without short_ids
        db.conn()
            .execute(
                "INSERT INTO projects (id, name) VALUES ('p1', 'Project')",
                [],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO epics (id, project_id, title, created_at) VALUES ('e1', 'p1', 'First Epic', '2025-01-01 00:00:00')",
                [],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO epics (id, project_id, title, created_at) VALUES ('e2', 'p1', 'Second Epic', '2025-01-02 00:00:00')",
                [],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO tasks (id, epic_id, title, created_at) VALUES ('t1', 'e1', 'Task A', '2025-01-01 00:00:00')",
                [],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO tasks (id, epic_id, title, created_at) VALUES ('t2', 'e1', 'Task B', '2025-01-02 00:00:00')",
                [],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO tasks (id, epic_id, title, created_at) VALUES ('t3', 'e2', 'Task C', '2025-01-01 00:00:00')",
                [],
            )
            .unwrap();

        // Now run the full versioned migration system
        db.migrate().unwrap();

        // Verify epic short_ids
        let e1_sid: String = db
            .conn()
            .query_row("SELECT short_id FROM epics WHERE id = 'e1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(e1_sid, "E1");

        let e2_sid: String = db
            .conn()
            .query_row("SELECT short_id FROM epics WHERE id = 'e2'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(e2_sid, "E2");

        // Verify task short_ids
        let t1_sid: String = db
            .conn()
            .query_row("SELECT short_id FROM tasks WHERE id = 't1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(t1_sid, "E1-T1");

        let t2_sid: String = db
            .conn()
            .query_row("SELECT short_id FROM tasks WHERE id = 't2'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(t2_sid, "E1-T2");

        let t3_sid: String = db
            .conn()
            .query_row("SELECT short_id FROM tasks WHERE id = 't3'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(t3_sid, "E2-T1");
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
    fn test_failed_migration_does_not_record_version() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).unwrap();

        // Bring the database to schema version 1
        apply_raw_migration(&db, 1, include_str!("../../migrations/001_init.sql"));

        // Sabotage migration 2 by pre-creating a column it tries to add
        db.conn()
            .execute_batch("ALTER TABLE epics ADD COLUMN short_id TEXT")
            .unwrap();

        // migrate() should fail on the duplicate column in migration 2
        let result = db.migrate();
        assert!(
            result.is_err(),
            "migration 2 should fail due to duplicate column"
        );

        // The failed migration must not advance the recorded version
        let version: i32 = db
            .conn()
            .query_row("SELECT MAX(version) FROM _schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 1, "failed migration should not bump schema version");
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
