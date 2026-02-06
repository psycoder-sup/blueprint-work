use anyhow::{Context, Result};
use rusqlite::Row;

use crate::db::Database;
use crate::models::{AddDependencyInput, Dependency, DependencyType};

const SELECT_COLUMNS: &str = "id, blocker_type, blocker_id, blocked_type, blocked_id";

fn parse_dependency_type(s: &str) -> rusqlite::Result<DependencyType> {
    s.parse().map_err(|e: anyhow::Error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        )
    })
}

fn row_to_dependency(row: &Row) -> rusqlite::Result<Dependency> {
    let blocker_type_str: String = row.get("blocker_type")?;
    let blocked_type_str: String = row.get("blocked_type")?;

    Ok(Dependency {
        id: row.get("id")?,
        blocker_type: parse_dependency_type(&blocker_type_str)?,
        blocker_id: row.get("blocker_id")?,
        blocked_type: parse_dependency_type(&blocked_type_str)?,
        blocked_id: row.get("blocked_id")?,
    })
}

fn validate_item_exists(db: &Database, item_type: &DependencyType, item_id: &str) -> Result<()> {
    let table = match item_type {
        DependencyType::Epic => "epics",
        DependencyType::Task => "tasks",
    };

    let exists: bool = db
        .conn()
        .prepare(&format!("SELECT 1 FROM {table} WHERE id = ?1"))?
        .exists([item_id])
        .with_context(|| format!("failed to check existence of {item_type} {item_id}"))?;

    anyhow::ensure!(exists, "{item_type} not found: {item_id}");
    Ok(())
}

pub fn add_dependency(db: &Database, input: AddDependencyInput) -> Result<Dependency> {
    if input.blocker_type == input.blocked_type && input.blocker_id == input.blocked_id {
        anyhow::bail!("cannot create self-referencing dependency");
    }

    validate_item_exists(db, &input.blocker_type, &input.blocker_id)?;
    validate_item_exists(db, &input.blocked_type, &input.blocked_id)?;

    if let Err(e) = db.conn().execute(
        "INSERT INTO dependencies (blocker_type, blocker_id, blocked_type, blocked_id) VALUES (?1, ?2, ?3, ?4)",
        [
            input.blocker_type.as_str(),
            &input.blocker_id,
            input.blocked_type.as_str(),
            &input.blocked_id,
        ],
    ) {
        return match e {
            rusqlite::Error::SqliteFailure(ref err, _)
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                anyhow::bail!("dependency already exists")
            }
            _ => Err(e).context("failed to insert dependency"),
        };
    }

    let id = db.conn().last_insert_rowid();
    db.conn()
        .prepare(&format!("SELECT {SELECT_COLUMNS} FROM dependencies WHERE id = ?1"))?
        .query_row([id], row_to_dependency)
        .context("dependency not found after insert")
}

pub fn remove_dependency(
    db: &Database,
    blocker_type: &DependencyType,
    blocker_id: &str,
    blocked_type: &DependencyType,
    blocked_id: &str,
) -> Result<bool> {
    let rows_affected = db
        .conn()
        .execute(
            "DELETE FROM dependencies WHERE blocker_type = ?1 AND blocker_id = ?2 AND blocked_type = ?3 AND blocked_id = ?4",
            [blocker_type.as_str(), blocker_id, blocked_type.as_str(), blocked_id],
        )
        .context("failed to delete dependency")?;

    Ok(rows_affected > 0)
}

pub fn get_blockers(
    db: &Database,
    item_type: &DependencyType,
    item_id: &str,
) -> Result<Vec<Dependency>> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM dependencies WHERE blocked_type = ?1 AND blocked_id = ?2");
    let mut stmt = db.conn().prepare(&sql)?;
    let rows = stmt.query_map([item_type.as_str(), item_id], row_to_dependency)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list blockers")
}

pub fn get_blocked_by(
    db: &Database,
    item_type: &DependencyType,
    item_id: &str,
) -> Result<Vec<Dependency>> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM dependencies WHERE blocker_type = ?1 AND blocker_id = ?2");
    let mut stmt = db.conn().prepare(&sql)?;
    let rows = stmt.query_map([item_type.as_str(), item_id], row_to_dependency)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list blocked items")
}

pub fn get_all_dependencies(db: &Database) -> Result<Vec<Dependency>> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM dependencies");
    let mut stmt = db.conn().prepare(&sql)?;
    let rows = stmt.query_map([], row_to_dependency)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list all dependencies")
}

pub fn is_blocked(
    db: &Database,
    item_type: &DependencyType,
    item_id: &str,
) -> Result<bool> {
    let sql = "
        SELECT EXISTS(
            SELECT 1 FROM dependencies d
            JOIN epics e ON d.blocker_type = 'epic' AND d.blocker_id = e.id
            WHERE d.blocked_type = ?1 AND d.blocked_id = ?2 AND e.status != 'done'

            UNION ALL

            SELECT 1 FROM dependencies d
            JOIN tasks t ON d.blocker_type = 'task' AND d.blocker_id = t.id
            WHERE d.blocked_type = ?1 AND d.blocked_id = ?2 AND t.status != 'done'
        )
    ";

    db.conn()
        .query_row(sql, [item_type.as_str(), item_id], |row| row.get(0))
        .context("failed to check if item is blocked")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::epic::create_epic;
    use crate::db::project::create_project;
    use crate::db::task::{create_task, update_task};
    use crate::models::{
        CreateEpicInput, CreateProjectInput, CreateTaskInput, Epic, ItemStatus, Project,
        UpdateTaskInput,
    };
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
                description: "For dependency tests".to_string(),
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
                description: "For dependency tests".to_string(),
            },
        )
        .unwrap()
    }

    fn create_test_task(db: &Database, epic_id: &str) -> crate::models::BlueTask {
        create_task(
            db,
            CreateTaskInput {
                epic_id: epic_id.to_string(),
                title: "Test Task".to_string(),
                description: "For dependency tests".to_string(),
            },
        )
        .unwrap()
    }

    #[test]
    fn test_add_between_epics() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let e1 = create_test_epic(&db, &project.id);
        let e2 = create_test_epic(&db, &project.id);

        let dep = add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: e1.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: e2.id.clone(),
            },
        )
        .unwrap();

        assert!(dep.id > 0);
        assert_eq!(dep.blocker_type, DependencyType::Epic);
        assert_eq!(dep.blocker_id, e1.id);
        assert_eq!(dep.blocked_type, DependencyType::Epic);
        assert_eq!(dep.blocked_id, e2.id);
    }

    #[test]
    fn test_add_between_tasks() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let t1 = create_test_task(&db, &epic.id);
        let t2 = create_test_task(&db, &epic.id);

        let dep = add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t2.id.clone(),
            },
        )
        .unwrap();

        assert_eq!(dep.blocker_type, DependencyType::Task);
        assert_eq!(dep.blocker_id, t1.id);
        assert_eq!(dep.blocked_type, DependencyType::Task);
        assert_eq!(dep.blocked_id, t2.id);
    }

    #[test]
    fn test_add_cross_type() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let task = create_test_task(&db, &epic.id);

        let dep = add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: epic.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: task.id.clone(),
            },
        )
        .unwrap();

        assert_eq!(dep.blocker_type, DependencyType::Epic);
        assert_eq!(dep.blocked_type, DependencyType::Task);
    }

    #[test]
    fn test_add_rejects_self_reference() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        let result = add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: epic.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: epic.id.clone(),
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("self-referencing"));
    }

    #[test]
    fn test_add_rejects_duplicate() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let e1 = create_test_epic(&db, &project.id);
        let e2 = create_test_epic(&db, &project.id);

        let input = || AddDependencyInput {
            blocker_type: DependencyType::Epic,
            blocker_id: e1.id.clone(),
            blocked_type: DependencyType::Epic,
            blocked_id: e2.id.clone(),
        };

        add_dependency(&db, input()).unwrap();
        let result = add_dependency(&db, input());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_add_rejects_nonexistent_blocker() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        let result = add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: "nonexistent".to_string(),
                blocked_type: DependencyType::Epic,
                blocked_id: epic.id.clone(),
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_add_rejects_nonexistent_blocked() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);

        let result = add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: epic.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: "nonexistent".to_string(),
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_remove_existing() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let e1 = create_test_epic(&db, &project.id);
        let e2 = create_test_epic(&db, &project.id);

        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: e1.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: e2.id.clone(),
            },
        )
        .unwrap();

        let removed =
            remove_dependency(&db, &DependencyType::Epic, &e1.id, &DependencyType::Epic, &e2.id)
                .unwrap();
        assert!(removed);
    }

    #[test]
    fn test_remove_nonexistent() {
        let (db, _dir) = open_temp_db();

        let removed = remove_dependency(
            &db,
            &DependencyType::Epic,
            "a",
            &DependencyType::Epic,
            "b",
        )
        .unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_get_blockers() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let e1 = create_test_epic(&db, &project.id);
        let e2 = create_test_epic(&db, &project.id);
        let e3 = create_test_epic(&db, &project.id);

        // e1 blocks e3, e2 blocks e3
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: e1.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: e3.id.clone(),
            },
        )
        .unwrap();
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: e2.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: e3.id.clone(),
            },
        )
        .unwrap();

        let blockers = get_blockers(&db, &DependencyType::Epic, &e3.id).unwrap();
        assert_eq!(blockers.len(), 2);

        let blocker_ids: Vec<&str> = blockers.iter().map(|d| d.blocker_id.as_str()).collect();
        assert!(blocker_ids.contains(&e1.id.as_str()));
        assert!(blocker_ids.contains(&e2.id.as_str()));
    }

    #[test]
    fn test_get_blocked_by() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let e1 = create_test_epic(&db, &project.id);
        let e2 = create_test_epic(&db, &project.id);
        let e3 = create_test_epic(&db, &project.id);

        // e1 blocks e2, e1 blocks e3
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: e1.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: e2.id.clone(),
            },
        )
        .unwrap();
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: e1.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: e3.id.clone(),
            },
        )
        .unwrap();

        let blocked = get_blocked_by(&db, &DependencyType::Epic, &e1.id).unwrap();
        assert_eq!(blocked.len(), 2);

        let blocked_ids: Vec<&str> = blocked.iter().map(|d| d.blocked_id.as_str()).collect();
        assert!(blocked_ids.contains(&e2.id.as_str()));
        assert!(blocked_ids.contains(&e3.id.as_str()));
    }

    #[test]
    fn test_get_all_dependencies() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let e1 = create_test_epic(&db, &project.id);
        let e2 = create_test_epic(&db, &project.id);
        let epic = create_test_epic(&db, &project.id);
        let t1 = create_test_task(&db, &epic.id);

        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: e1.id.clone(),
                blocked_type: DependencyType::Epic,
                blocked_id: e2.id.clone(),
            },
        )
        .unwrap();
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Epic,
                blocker_id: e1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t1.id.clone(),
            },
        )
        .unwrap();

        let all = get_all_dependencies(&db).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_is_blocked_blocker_not_done() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let t1 = create_test_task(&db, &epic.id);
        let t2 = create_test_task(&db, &epic.id);

        // t1 blocks t2, t1 is still "todo"
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t2.id.clone(),
            },
        )
        .unwrap();

        assert!(is_blocked(&db, &DependencyType::Task, &t2.id).unwrap());
    }

    #[test]
    fn test_is_blocked_blocker_done() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let t1 = create_test_task(&db, &epic.id);
        let t2 = create_test_task(&db, &epic.id);

        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t2.id.clone(),
            },
        )
        .unwrap();

        // Mark blocker as done
        update_task(
            &db,
            &t1.id,
            UpdateTaskInput {
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(!is_blocked(&db, &DependencyType::Task, &t2.id).unwrap());
    }

    #[test]
    fn test_is_blocked_mixed_statuses() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let t1 = create_test_task(&db, &epic.id);
        let t2 = create_test_task(&db, &epic.id);
        let t3 = create_test_task(&db, &epic.id);

        // t1 and t2 both block t3
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t3.id.clone(),
            },
        )
        .unwrap();
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t2.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t3.id.clone(),
            },
        )
        .unwrap();

        // Mark t1 as done, t2 still todo
        update_task(
            &db,
            &t1.id,
            UpdateTaskInput {
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();

        // t3 should still be blocked (t2 is not done)
        assert!(is_blocked(&db, &DependencyType::Task, &t3.id).unwrap());
    }

    #[test]
    fn test_is_blocked_no_dependencies() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let task = create_test_task(&db, &epic.id);

        assert!(!is_blocked(&db, &DependencyType::Task, &task.id).unwrap());
    }

    #[test]
    fn test_full_lifecycle() {
        let (db, _dir) = open_temp_db();
        let project = create_test_project(&db);
        let epic = create_test_epic(&db, &project.id);
        let t1 = create_test_task(&db, &epic.id);
        let t2 = create_test_task(&db, &epic.id);

        // Add dependency: t1 blocks t2
        add_dependency(
            &db,
            AddDependencyInput {
                blocker_type: DependencyType::Task,
                blocker_id: t1.id.clone(),
                blocked_type: DependencyType::Task,
                blocked_id: t2.id.clone(),
            },
        )
        .unwrap();

        // t2 should be blocked
        assert!(is_blocked(&db, &DependencyType::Task, &t2.id).unwrap());

        // Mark t1 as done
        update_task(
            &db,
            &t1.id,
            UpdateTaskInput {
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();

        // t2 should no longer be blocked
        assert!(!is_blocked(&db, &DependencyType::Task, &t2.id).unwrap());

        // Remove the dependency
        let removed = remove_dependency(
            &db,
            &DependencyType::Task,
            &t1.id,
            &DependencyType::Task,
            &t2.id,
        )
        .unwrap();
        assert!(removed);

        // No more blockers
        let blockers = get_blockers(&db, &DependencyType::Task, &t2.id).unwrap();
        assert!(blockers.is_empty());
    }
}
