use crate::db::DbState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct PingResponse {
    pub message: String,
    pub app: String,
}

#[derive(Serialize)]
pub struct DbVersionResponse {
    pub schema_version: String,
    pub ok: bool,
}

#[derive(Serialize)]
pub struct DbPathResponse {
    pub path: String,
}

#[tauri::command]
pub fn ping() -> PingResponse {
    PingResponse {
        message: "pong".into(),
        app: "ApiTest".into(),
    }
}

#[tauri::command]
pub fn db_version(state: State<'_, DbState>) -> Result<DbVersionResponse, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "unknown".into());

    Ok(DbVersionResponse {
        schema_version: version,
        ok: true,
    })
}

#[tauri::command]
pub fn db_path(state: State<'_, DbState>) -> Result<DbPathResponse, String> {
    Ok(DbPathResponse {
        path: state.db_path.display().to_string(),
    })
}
