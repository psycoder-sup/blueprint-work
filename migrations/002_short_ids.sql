ALTER TABLE epics ADD COLUMN short_id TEXT;
ALTER TABLE tasks ADD COLUMN short_id TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_epics_short_id ON epics(project_id, short_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_short_id ON tasks(epic_id, short_id);

-- Backfill epics: E1, E2, ... per project (ordered by created_at)
WITH numbered AS (
    SELECT id, 'E' || ROW_NUMBER() OVER (PARTITION BY project_id ORDER BY created_at) AS sid
    FROM epics
)
UPDATE epics SET short_id = (SELECT sid FROM numbered WHERE numbered.id = epics.id)
WHERE short_id IS NULL;

-- Backfill tasks: E{n}-T1, E{n}-T2, ... per epic (ordered by created_at)
WITH epic_nums AS (
    SELECT id, ROW_NUMBER() OVER (PARTITION BY project_id ORDER BY created_at) AS enum_num
    FROM epics
),
task_nums AS (
    SELECT t.id,
           'E' || en.enum_num || '-T' || ROW_NUMBER() OVER (PARTITION BY t.epic_id ORDER BY t.created_at) AS sid
    FROM tasks t
    JOIN epic_nums en ON en.id = t.epic_id
)
UPDATE tasks SET short_id = (SELECT sid FROM task_nums WHERE task_nums.id = tasks.id)
WHERE short_id IS NULL;
