use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::db::Database;

pub struct BlockedItemRow {
    pub item_type: String,
    pub item_id: String,
    pub title: String,
    pub blocker_id: String,
}

pub struct DependencyDisplayRow {
    pub blocker_title: String,
    pub blocked_title: String,
    pub is_active: bool,
}

/// Builds the SQL and params for a query that optionally filters by project ID.
/// `base_sql` is the shared prefix, `filter_suffix` is appended when a project ID is provided,
/// and `unfiltered_suffix` is appended otherwise.
fn build_filtered_query(
    base_sql: &str,
    filter_suffix: &str,
    unfiltered_suffix: &str,
    project_id: Option<&str>,
) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
    match project_id {
        Some(pid) => (
            format!("{base_sql}{filter_suffix}"),
            vec![Box::new(pid.to_string())],
        ),
        None => (format!("{base_sql}{unfiltered_suffix}"), vec![]),
    }
}

/// Count rows grouped by status, ensuring all three status keys are present.
fn count_by_status(
    db: &Database,
    base_sql: &str,
    filtered_sql: &str,
    project_id: Option<&str>,
    label: &str,
) -> Result<HashMap<String, i64>> {
    let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match project_id {
        Some(pid) => (filtered_sql, vec![Box::new(pid.to_string())]),
        None => (base_sql, vec![]),
    };

    let mut stmt = db.conn().prepare(sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;

    let mut map = HashMap::new();
    for row in rows {
        let (status, count) = row.with_context(|| format!("failed to read {label} status count"))?;
        map.insert(status, count);
    }

    for key in ["todo", "in_progress", "done"] {
        map.entry(key.to_string()).or_insert(0);
    }

    Ok(map)
}

pub fn count_epics_by_status(
    db: &Database,
    project_id: Option<&str>,
) -> Result<HashMap<String, i64>> {
    count_by_status(
        db,
        "SELECT status, COUNT(*) as count FROM epics GROUP BY status",
        "SELECT status, COUNT(*) as count FROM epics WHERE project_id = ?1 GROUP BY status",
        project_id,
        "epic",
    )
}

pub fn count_tasks_by_status(
    db: &Database,
    project_id: Option<&str>,
) -> Result<HashMap<String, i64>> {
    count_by_status(
        db,
        "SELECT status, COUNT(*) as count FROM tasks GROUP BY status",
        "SELECT t.status, COUNT(*) as count FROM tasks t \
         JOIN epics e ON t.epic_id = e.id \
         WHERE e.project_id = ?1 \
         GROUP BY t.status",
        project_id,
        "task",
    )
}

/// SQL suffix to filter dependencies by project ownership.
const PROJECT_FILTER_SUFFIX: &str = " \
    AND ( \
        (d.blocked_type = 'epic' AND blocked_e.project_id = ?1) \
        OR (d.blocked_type = 'task' AND blocked_t.epic_id IN \
            (SELECT id FROM epics WHERE project_id = ?1)) \
    )";

pub fn get_blocked_items(
    db: &Database,
    project_id: Option<&str>,
) -> Result<Vec<BlockedItemRow>> {
    let base = "\
        SELECT \
            d.blocked_type, d.blocked_id, \
            COALESCE(blocked_e.title, blocked_t.title) as title, \
            d.blocker_id \
        FROM dependencies d \
        LEFT JOIN epics blocker_e ON d.blocker_type = 'epic' AND d.blocker_id = blocker_e.id \
        LEFT JOIN tasks blocker_t ON d.blocker_type = 'task' AND d.blocker_id = blocker_t.id \
        LEFT JOIN epics blocked_e ON d.blocked_type = 'epic' AND d.blocked_id = blocked_e.id \
        LEFT JOIN tasks blocked_t ON d.blocked_type = 'task' AND d.blocked_id = blocked_t.id \
        WHERE ( \
            (blocker_e.id IS NOT NULL AND blocker_e.status != 'done') \
            OR (blocker_t.id IS NOT NULL AND blocker_t.status != 'done') \
        ) \
        AND (blocked_e.id IS NOT NULL OR blocked_t.id IS NOT NULL)";

    let order = " ORDER BY d.blocked_type, d.blocked_id";
    let (sql, params) = build_filtered_query(
        base,
        &format!("{PROJECT_FILTER_SUFFIX}{order}"),
        order,
        project_id,
    );

    let mut stmt = db.conn().prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(BlockedItemRow {
            item_type: row.get(0)?,
            item_id: row.get(1)?,
            title: row.get(2)?,
            blocker_id: row.get(3)?,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to query blocked items")
}

pub fn get_dependency_display_rows(
    db: &Database,
    project_id: Option<&str>,
) -> Result<Vec<DependencyDisplayRow>> {
    let base = "\
        SELECT \
            COALESCE(blocker_e.title, blocker_t.title) as blocker_title, \
            COALESCE(blocked_e.title, blocked_t.title) as blocked_title, \
            CASE WHEN ( \
                (blocker_e.id IS NOT NULL AND blocker_e.status != 'done') \
                OR (blocker_t.id IS NOT NULL AND blocker_t.status != 'done') \
            ) THEN 1 ELSE 0 END as is_active \
        FROM dependencies d \
        LEFT JOIN epics blocker_e ON d.blocker_type = 'epic' AND d.blocker_id = blocker_e.id \
        LEFT JOIN tasks blocker_t ON d.blocker_type = 'task' AND d.blocker_id = blocker_t.id \
        LEFT JOIN epics blocked_e ON d.blocked_type = 'epic' AND d.blocked_id = blocked_e.id \
        LEFT JOIN tasks blocked_t ON d.blocked_type = 'task' AND d.blocked_id = blocked_t.id \
        WHERE (blocked_e.id IS NOT NULL OR blocked_t.id IS NOT NULL) \
        AND (blocker_e.id IS NOT NULL OR blocker_t.id IS NOT NULL)";

    let order = " ORDER BY is_active DESC LIMIT 5";
    let (sql, params) = build_filtered_query(
        base,
        &format!("{PROJECT_FILTER_SUFFIX}{order}"),
        order,
        project_id,
    );

    let mut stmt = db.conn().prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(DependencyDisplayRow {
            blocker_title: row.get(0)?,
            blocked_title: row.get(1)?,
            is_active: row.get::<_, i64>(2)? != 0,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to query dependency display rows")
}

pub fn get_max_updated_at(db: &Database) -> Result<String> {
    let sql = "\
        SELECT COALESCE(MAX(ts), '') || ':' || dep_count FROM ( \
            SELECT MAX(updated_at) as ts FROM projects \
            UNION ALL \
            SELECT MAX(updated_at) as ts FROM epics \
            UNION ALL \
            SELECT MAX(updated_at) as ts FROM tasks \
        ), (SELECT CAST(COUNT(*) AS TEXT) as dep_count FROM dependencies)";
    db.conn()
        .query_row(sql, [], |row| row.get(0))
        .context("failed to query max updated_at")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::dependency::add_dependency;
    use crate::db::epic::create_epic;
    use crate::db::project::create_project;
    use crate::db::task::{create_task, update_task};
    use crate::models::{
        AddDependencyInput, CreateEpicInput, CreateProjectInput, CreateTaskInput, DependencyType,
        ItemStatus, UpdateTaskInput,
    };
    use tempfile::TempDir;

    fn open_temp_db() -> (Database, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path).unwrap();
        db.migrate().unwrap();
        (db, dir)
    }

    #[test]
    fn test_empty_db_all_zeros() {
        let (db, _dir) = open_temp_db();

        let epics = count_epics_by_status(&db, None).unwrap();
        assert_eq!(epics["todo"], 0);
        assert_eq!(epics["in_progress"], 0);
        assert_eq!(epics["done"], 0);

        let tasks = count_tasks_by_status(&db, None).unwrap();
        assert_eq!(tasks["todo"], 0);
        assert_eq!(tasks["in_progress"], 0);
        assert_eq!(tasks["done"], 0);

        let blocked = get_blocked_items(&db, None).unwrap();
        assert!(blocked.is_empty());
    }

    #[test]
    fn test_epic_count_breakdown() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        // Create 3 epics: 2 todo (default), 1 in_progress
        create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E1".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E2".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let e3 = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E3".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        crate::db::epic::update_epic(
            &db,
            &e3.id,
            crate::models::UpdateEpicInput {
                status: Some(ItemStatus::InProgress),
                ..Default::default()
            },
        )
        .unwrap();

        let counts = count_epics_by_status(&db, None).unwrap();
        assert_eq!(counts["todo"], 2);
        assert_eq!(counts["in_progress"], 1);
        assert_eq!(counts["done"], 0);
    }

    #[test]
    fn test_epic_counts_with_project_filter() {
        let (db, _dir) = open_temp_db();
        let p1 = create_project(
            &db,
            CreateProjectInput {
                name: "P1".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let p2 = create_project(
            &db,
            CreateProjectInput {
                name: "P2".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        create_epic(
            &db,
            CreateEpicInput {
                project_id: p1.id.clone(),
                title: "E1".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        create_epic(
            &db,
            CreateEpicInput {
                project_id: p2.id.clone(),
                title: "E2".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        create_epic(
            &db,
            CreateEpicInput {
                project_id: p2.id.clone(),
                title: "E3".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let p1_counts = count_epics_by_status(&db, Some(&p1.id)).unwrap();
        assert_eq!(p1_counts["todo"], 1);

        let p2_counts = count_epics_by_status(&db, Some(&p2.id)).unwrap();
        assert_eq!(p2_counts["todo"], 2);
    }

    #[test]
    fn test_task_counts_with_data() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "T1".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "T2".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t3 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "T3".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        update_task(
            &db,
            &t2.id,
            UpdateTaskInput {
                status: Some(ItemStatus::InProgress),
                ..Default::default()
            },
        )
        .unwrap();
        update_task(
            &db,
            &t3.id,
            UpdateTaskInput {
                status: Some(ItemStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();

        let counts = count_tasks_by_status(&db, None).unwrap();
        assert_eq!(counts["todo"], 1);
        assert_eq!(counts["in_progress"], 1);
        assert_eq!(counts["done"], 1);
    }

    #[test]
    fn test_blocked_items_returns_correct_items() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocker".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocked".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

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

        let blocked = get_blocked_items(&db, None).unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].item_type, "task");
        assert_eq!(blocked[0].item_id, t2.id);
        assert_eq!(blocked[0].title, "Blocked");
        assert_eq!(blocked[0].blocker_id, t1.id);
    }

    #[test]
    fn test_get_dependency_display_rows_empty() {
        let (db, _dir) = open_temp_db();
        let rows = get_dependency_display_rows(&db, None).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_get_dependency_display_rows_with_active_block() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocker Task".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocked Task".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

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

        let rows = get_dependency_display_rows(&db, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].blocker_title, "Blocker Task");
        assert_eq!(rows[0].blocked_title, "Blocked Task");
        assert!(rows[0].is_active);
    }

    #[test]
    fn test_get_dependency_display_rows_resolved_block() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocker Task".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocked Task".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

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

        let rows = get_dependency_display_rows(&db, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].is_active);
    }

    #[test]
    fn test_get_max_updated_at_empty() {
        let (db, _dir) = open_temp_db();
        let result = get_max_updated_at(&db).unwrap();
        // Empty DB returns ":0" (no timestamps, zero dependencies)
        assert_eq!(result, ":0");
    }

    #[test]
    fn test_get_max_updated_at_with_data() {
        let (db, _dir) = open_temp_db();
        create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

        let result = get_max_updated_at(&db).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_blocked_items_excludes_done_blockers() {
        let (db, _dir) = open_temp_db();
        let project = create_project(
            &db,
            CreateProjectInput {
                name: "P".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let epic = create_epic(
            &db,
            CreateEpicInput {
                project_id: project.id.clone(),
                title: "E".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t1 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocker".to_string(),
                description: String::new(),
            },
        )
        .unwrap();
        let t2 = create_task(
            &db,
            CreateTaskInput {
                epic_id: epic.id.clone(),
                title: "Blocked".to_string(),
                description: String::new(),
            },
        )
        .unwrap();

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

        let blocked = get_blocked_items(&db, None).unwrap();
        assert!(blocked.is_empty());
    }
}
