use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct DbState {
    pub conn: Mutex<Connection>,
    pub db_path: PathBuf,
}

impl DbState {
    pub fn new(conn: Connection, db_path: PathBuf) -> Self {
        Self {
            conn: Mutex::new(conn),
            db_path,
        }
    }
}

pub fn open_and_migrate(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("open db failed: {e}"))?;

    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| format!("pragma failed: {e}"))?;

    let migration = include_str!("../../migrations/001_init.sql");
    conn.execute_batch(migration)
        .map_err(|e| format!("migration 001 failed: {e}"))?;

    run_phase3_migrate(&conn)?;
    seed_if_empty(&conn)?;

    Ok(conn)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(names.iter().any(|n| n == column))
}

fn add_column_if_missing(conn: &Connection, table: &str, column: &str, ddl: &str) -> Result<(), String> {
    if !column_exists(conn, table, column)? {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {ddl}"), [])
            .map_err(|e| format!("add column {table}.{column} failed: {e}"))?;
    }
    Ok(())
}

fn run_phase3_migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(include_str!("../../migrations/002_phase3.sql"))
        .map_err(|e| format!("migration 002 failed: {e}"))?;

    add_column_if_missing(conn, "collections", "workspace_id", "workspace_id INTEGER NOT NULL DEFAULT 1")?;
    add_column_if_missing(conn, "environments", "workspace_id", "workspace_id INTEGER NOT NULL DEFAULT 1")?;

    add_column_if_missing(conn, "requests", "pre_script", "pre_script TEXT NOT NULL DEFAULT ''")?;
    add_column_if_missing(conn, "requests", "test_script", "test_script TEXT NOT NULL DEFAULT ''")?;
    add_column_if_missing(conn, "requests", "mock_enabled", "mock_enabled INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(conn, "requests", "mock_status", "mock_status INTEGER NOT NULL DEFAULT 200")?;
    add_column_if_missing(
        conn,
        "requests",
        "mock_headers_json",
        "mock_headers_json TEXT NOT NULL DEFAULT '[]'",
    )?;
    add_column_if_missing(conn, "requests", "mock_body", "mock_body TEXT NOT NULL DEFAULT ''")?;
    add_column_if_missing(conn, "requests", "mock_delay_ms", "mock_delay_ms INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(
        conn,
        "collections",
        "base_url",
        "base_url TEXT NOT NULL DEFAULT ''",
    )?;

    // ensure all existing rows belong to default workspace
    let _ = conn.execute(
        "UPDATE collections SET workspace_id = 1 WHERE workspace_id IS NULL OR workspace_id = 0",
        [],
    );
    let _ = conn.execute(
        "UPDATE environments SET workspace_id = 1 WHERE workspace_id IS NULL OR workspace_id = 0",
        [],
    );

    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '4')",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn seed_if_empty(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM collections", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    if count > 0 {
        return Ok(());
    }

    let ws_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE is_active = 1 LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO collections (name, sort_order, workspace_id) VALUES ('My Collection', 0, ?1)",
        rusqlite::params![ws_id],
    )
    .map_err(|e| e.to_string())?;
    let collection_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO requests (collection_id, name, method, url, headers_json, body_type)
         VALUES (?1, 'Get IP', 'GET', 'https://httpbin.org/ip', '[]', 'none')",
        rusqlite::params![collection_id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO requests (collection_id, name, method, url, headers_json, body_type, body_content, test_script)
         VALUES (?1, 'Echo JSON', 'POST', 'https://httpbin.org/post',
                 '[{\"key\":\"Content-Type\",\"value\":\"application/json\",\"enabled\":true}]',
                 'json',
                 '{\"hello\":\"ApiTest\"}',
                 'at.test(\"status is 200\", () => at.expect(at.response.status).toBe(200));\nat.test(\"has json\", () => at.expect(at.response.json()).toBeTruthy());')",
        rusqlite::params![collection_id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO requests (collection_id, name, method, url, body_type, mock_enabled, mock_status, mock_body, mock_headers_json)
         VALUES (?1, 'Mock Demo', 'GET', 'https://example.com/mock', 'none', 1, 200,
                 '{\"mocked\":true,\"message\":\"hello from ApiTest mock\"}',
                 '[{\"key\":\"Content-Type\",\"value\":\"application/json\",\"enabled\":true}]')",
        rusqlite::params![collection_id],
    )
    .map_err(|e| e.to_string())?;

    let env_id: i64 = conn
        .query_row(
            "SELECT id FROM environments WHERE name = 'Default' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(1);

    let _ = conn.execute(
        "UPDATE environments SET workspace_id = ?1 WHERE id = ?2",
        rusqlite::params![ws_id, env_id],
    );

    conn.execute(
        "INSERT OR IGNORE INTO environment_vars (environment_id, key, value, enabled)
         VALUES (?1, 'baseUrl', 'https://httpbin.org', 1)",
        rusqlite::params![env_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
