use crate::db::DbState;
use crate::models::{CreateRequestInput, KeyValue, RequestItem, SaveRequestInput};
use rusqlite::params;
use tauri::State;

fn parse_kv(json: &str) -> Vec<KeyValue> {
    serde_json::from_str(json).unwrap_or_default()
}

fn map_request(row: &rusqlite::Row<'_>) -> rusqlite::Result<RequestItem> {
    let params_json: String = row.get(5)?;
    let headers_json: String = row.get(6)?;
    let mock_headers_json: String = row.get(14)?;
    let mock_enabled: i64 = row.get(12)?;
    Ok(RequestItem {
        id: row.get(0)?,
        collection_id: row.get(1)?,
        name: row.get(2)?,
        method: row.get(3)?,
        url: row.get(4)?,
        params: parse_kv(&params_json),
        headers: parse_kv(&headers_json),
        body_type: row.get(7)?,
        body_content: row.get(8)?,
        sort_order: row.get(9)?,
        pre_script: row.get(10)?,
        test_script: row.get(11)?,
        mock_enabled: mock_enabled == 1,
        mock_status: row.get(13)?,
        mock_headers: parse_kv(&mock_headers_json),
        mock_body: row.get(15)?,
        mock_delay_ms: row.get(16)?,
    })
}

const REQUEST_SELECT: &str = "SELECT id, collection_id, name, method, url, params_json, headers_json,
        body_type, body_content, sort_order, pre_script, test_script, mock_enabled, mock_status,
        mock_headers_json, mock_body, mock_delay_ms
 FROM requests";

#[tauri::command]
pub fn get_request(state: State<'_, DbState>, id: i64) -> Result<RequestItem, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        &format!("{REQUEST_SELECT} WHERE id = ?1"),
        params![id],
        map_request,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_request(
    state: State<'_, DbState>,
    input: CreateRequestInput,
) -> Result<RequestItem, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("请求名称不能为空".into());
    }

    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO requests (collection_id, name, method, url) VALUES (?1, ?2, ?3, ?4)",
        params![
            input.collection_id,
            name,
            input.method.to_uppercase(),
            input.url
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    conn.query_row(
        &format!("{REQUEST_SELECT} WHERE id = ?1"),
        params![id],
        map_request,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_request(state: State<'_, DbState>, input: SaveRequestInput) -> Result<(), String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("请求名称不能为空".into());
    }

    let params_json = serde_json::to_string(&input.params).map_err(|e| e.to_string())?;
    let headers_json = serde_json::to_string(&input.headers).map_err(|e| e.to_string())?;
    let mock_headers_json =
        serde_json::to_string(&input.mock_headers).map_err(|e| e.to_string())?;
    let mock_enabled = if input.mock_enabled { 1 } else { 0 };

    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let n = conn
        .execute(
            "UPDATE requests SET
                collection_id = ?1,
                name = ?2,
                method = ?3,
                url = ?4,
                params_json = ?5,
                headers_json = ?6,
                body_type = ?7,
                body_content = ?8,
                pre_script = ?9,
                test_script = ?10,
                mock_enabled = ?11,
                mock_status = ?12,
                mock_headers_json = ?13,
                mock_body = ?14,
                mock_delay_ms = ?15,
                updated_at = datetime('now')
             WHERE id = ?16",
            params![
                input.collection_id,
                name,
                input.method.to_uppercase(),
                input.url,
                params_json,
                headers_json,
                input.body_type,
                input.body_content,
                input.pre_script,
                input.test_script,
                mock_enabled,
                input.mock_status,
                mock_headers_json,
                input.mock_body,
                input.mock_delay_ms,
                input.id
            ],
        )
        .map_err(|e| e.to_string())?;

    if n == 0 {
        return Err("请求不存在".into());
    }
    Ok(())
}

#[tauri::command]
pub fn delete_request(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM requests WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
