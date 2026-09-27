use crate::db::DbState;
use crate::models::{CreateWorkspaceInput, Workspace};
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
pub fn list_workspaces(state: State<'_, DbState>) -> Result<Vec<Workspace>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, is_active FROM workspaces ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let active: i64 = row.get(2)?;
            Ok(Workspace {
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
pub fn create_workspace(
    state: State<'_, DbState>,
    input: CreateWorkspaceInput,
) -> Result<Workspace, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("工作区名称不能为空".into());
    }
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO workspaces (name, is_active) VALUES (?1, 0)",
        params![name],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();

    // seed default env for workspace (unique name globally)
    let _ = conn.execute(
        "INSERT INTO environments (name, is_active, workspace_id) VALUES (?1, 1, ?2)",
        params![format!("Default#ws{id}"), id],
    );

    Ok(Workspace {
        id,
        name,
        is_active: false,
    })
}

#[tauri::command]
pub fn set_active_workspace(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE workspaces SET is_active = 0", [])
        .map_err(|e| e.to_string())?;
    let n = conn
        .execute(
            "UPDATE workspaces SET is_active = 1, updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("工作区不存在".into());
    }

    // ensure an active environment exists in this workspace
    let has_active: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM environments WHERE workspace_id = ?1 AND is_active = 1",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if has_active == 0 {
        let _ = conn.execute(
            "UPDATE environments SET is_active = 1 WHERE id = (
                SELECT id FROM environments WHERE workspace_id = ?1 ORDER BY id LIMIT 1
            )",
            params![id],
        );
    }
    Ok(())
}

#[tauri::command]
pub fn get_active_workspace_id(state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    active_workspace_id(&conn)
}

#[tauri::command]
pub fn delete_workspace(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    if id == 1 {
        return Err("默认工作区不可删除".into());
    }
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let active = active_workspace_id(&conn)?;
    if active == id {
        return Err("不能删除当前工作区，请先切换".into());
    }

    // delete collections (and requests) in workspace
    let mut stmt = conn
        .prepare("SELECT id FROM collections WHERE workspace_id = ?1")
        .map_err(|e| e.to_string())?;
    let col_ids: Vec<i64> = stmt
        .query_map(params![id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    for cid in col_ids {
        conn.execute("DELETE FROM requests WHERE collection_id = ?1", params![cid])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM collections WHERE id = ?1", params![cid])
            .map_err(|e| e.to_string())?;
    }

    // delete environments
    let mut estmt = conn
        .prepare("SELECT id FROM environments WHERE workspace_id = ?1")
        .map_err(|e| e.to_string())?;
    let env_ids: Vec<i64> = estmt
        .query_map(params![id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(estmt);
    for eid in env_ids {
        conn.execute(
            "DELETE FROM environment_vars WHERE environment_id = ?1",
            params![eid],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM environments WHERE id = ?1", params![eid])
            .map_err(|e| e.to_string())?;
    }

    conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
