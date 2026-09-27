use crate::db::DbState;
use crate::http::send_http;
use crate::models::{KeyValue, SendRequestInput, SendRequestResult};
use rusqlite::params;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tauri::State;

fn load_env_vars(
    conn: &rusqlite::Connection,
    environment_id: Option<i64>,
) -> Result<HashMap<String, String>, String> {
    let env_id = match environment_id {
        Some(id) => id,
        None => {
            match conn.query_row(
                "SELECT e.id FROM environments e
                 INNER JOIN workspaces w ON e.workspace_id = w.id
                 WHERE w.is_active = 1 AND e.is_active = 1
                 LIMIT 1",
                [],
                |row| row.get::<_, i64>(0),
            ) {
                Ok(id) => id,
                Err(_) => {
                    // fallback: any env in active workspace
                    match conn.query_row(
                        "SELECT e.id FROM environments e
                         INNER JOIN workspaces w ON e.workspace_id = w.id
                         WHERE w.is_active = 1
                         ORDER BY e.id LIMIT 1",
                        [],
                        |row| row.get::<_, i64>(0),
                    ) {
                        Ok(id) => id,
                        Err(_) => return Ok(HashMap::new()),
                    }
                }
            }
        }
    };

    let mut stmt = conn
        .prepare(
            "SELECT key, value FROM environment_vars WHERE environment_id = ?1 AND enabled = 1",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![env_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;

    let mut map = HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        if !k.is_empty() {
            map.insert(k, v);
        }
    }
    Ok(map)
}

fn try_mock_response(
    conn: &rusqlite::Connection,
    input: &SendRequestInput,
) -> Result<Option<SendRequestResult>, String> {
    let Some(request_id) = input.request_id else {
        return Ok(None);
    };

    let row = conn.query_row(
        "SELECT mock_enabled, mock_status, mock_headers_json, mock_body, mock_delay_ms
         FROM requests WHERE id = ?1",
        params![request_id],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        },
    );

    let Ok((enabled, status, headers_json, body, delay_ms)) = row else {
        return Ok(None);
    };

    if enabled != 1 {
        return Ok(None);
    }

    if delay_ms > 0 {
        thread::sleep(Duration::from_millis(delay_ms as u64));
    }

    let headers: Vec<KeyValue> = serde_json::from_str(&headers_json).unwrap_or_default();
    let status_u16 = status.clamp(100, 599) as u16;

    Ok(Some(SendRequestResult {
        status: status_u16,
        status_text: "MOCK".into(),
        duration_ms: delay_ms.max(0) as u128,
        headers,
        body,
        url: input.url.clone(),
        error: None,
        mocked: true,
    }))
}

#[tauri::command]
pub fn send_request(
    state: State<'_, DbState>,
    mut input: SendRequestInput,
) -> Result<SendRequestResult, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let vars = load_env_vars(&conn, input.environment_id)?;

    // Apply collection URL prefix when request belongs to a collection
    if let Some(request_id) = input.request_id {
        let base: String = conn
            .query_row(
                "SELECT COALESCE(c.base_url, '')
                 FROM requests r
                 LEFT JOIN collections c ON r.collection_id = c.id
                 WHERE r.id = ?1",
                params![request_id],
                |row| row.get(0),
            )
            .unwrap_or_default();
        if !base.trim().is_empty() {
            input.url = crate::http::join_base_url(&base, &input.url);
        }
    }

    let result = if let Some(mock) = try_mock_response(&conn, &input)? {
        mock
    } else {
        send_http(&input, &vars)
    };

    let request_snapshot = serde_json::to_string(&input).unwrap_or_default();
    let response_snapshot = serde_json::to_string(&result).unwrap_or_default();

    let _ = conn.execute(
        "INSERT INTO history (request_id, method, url, status_code, duration_ms, request_snapshot, response_snapshot)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            input.request_id,
            input.method,
            result.url,
            result.status as i64,
            result.duration_ms as i64,
            request_snapshot,
            response_snapshot
        ],
    );

    Ok(result)
}

#[tauri::command]
pub fn preview_substituted(
    state: State<'_, DbState>,
    text: String,
    environment_id: Option<i64>,
) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let vars = load_env_vars(&conn, environment_id)?;
    Ok(crate::http::substitute(&text, &vars))
}
