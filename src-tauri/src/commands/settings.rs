use crate::db::DbState;
use crate::models::SetThemeInput;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn get_theme(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let theme: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'theme'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "light".into());
    Ok(theme)
}

#[tauri::command]
pub fn set_theme(state: State<'_, DbState>, input: SetThemeInput) -> Result<(), String> {
    let theme = if input.theme == "dark" { "dark" } else { "light" };
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('theme', ?1)",
        params![theme],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
