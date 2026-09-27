use crate::db::DbState;
use crate::models::{EnvVar, Environment, UpsertEnvVarInput};
use rusqlite::params;
use tauri::State;

fn active_workspace_id(conn: &rusqlite::Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT id FROM workspaces WHERE is_active = 1 LIMIT 1",
        [],
        |row| row.get(0),
    )
    .or_else(|_| {
        conn.query_row("SELECT id FROM workspaces ORDER BY id LIMIT 1", [], |row| {
            row.get(0)
        })
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_environments(state: State<'_, DbState>) -> Result<Vec<Environment>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let ws = active_workspace_id(&conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, is_active FROM environments WHERE workspace_id = ?1 ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![ws], |row| {
            let active: i64 = row.get(2)?;
            Ok(Environment {
                id: row.get(0)?,
                name: row.get(1)?,
                is_active: active == 1,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_environment(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let ws: i64 = conn
        .query_row(
            "SELECT workspace_id FROM environments WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|_| "环境不存在".to_string())?;

    conn.execute(
        "UPDATE environments SET is_active = 0 WHERE workspace_id = ?1",
        params![ws],
    )
    .map_err(|e| e.to_string())?;
    let n = conn
        .execute(
            "UPDATE environments SET is_active = 1, updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("环境不存在".into());
    }
    Ok(())
}

#[tauri::command]
pub fn list_env_vars(state: State<'_, DbState>, environment_id: i64) -> Result<Vec<EnvVar>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, environment_id, key, value, enabled FROM environment_vars WHERE environment_id = ?1 ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![environment_id], |row| {
            let enabled: i64 = row.get(4)?;
            Ok(EnvVar {
                id: row.get(0)?,
                environment_id: row.get(1)?,
                key: row.get(2)?,
                value: row.get(3)?,
                enabled: enabled == 1,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_env_var(state: State<'_, DbState>, input: UpsertEnvVarInput) -> Result<EnvVar, String> {
    let key = input.key.trim().to_string();
    if key.is_empty() {
        return Err("变量名不能为空".into());
    }
    let enabled = if input.enabled { 1 } else { 0 };
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO environment_vars (environment_id, key, value, enabled)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(environment_id, key) DO UPDATE SET value = excluded.value, enabled = excluded.enabled",
        params![input.environment_id, key, input.value, enabled],
    )
    .map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT id, environment_id, key, value, enabled FROM environment_vars WHERE environment_id = ?1 AND key = ?2",
        params![input.environment_id, input.key.trim()],
        |row| {
            let en: i64 = row.get(4)?;
            Ok(EnvVar {
                id: row.get(0)?,
                environment_id: row.get(1)?,
                key: row.get(2)?,
                value: row.get(3)?,
                enabled: en == 1,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_env_var(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM environment_vars WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn create_environment(state: State<'_, DbState>, name: String) -> Result<Environment, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("环境名称不能为空".into());
    }
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let ws = active_workspace_id(&conn)?;
    // keep unique globally: suffix workspace id when needed
    let stored_name = format!("{name}#ws{ws}");
    conn.execute(
        "INSERT INTO environments (name, is_active, workspace_id) VALUES (?1, 0, ?2)",
        params![stored_name, ws],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    Ok(Environment {
        id,
        name: stored_name,
        is_active: false,
    })
}
