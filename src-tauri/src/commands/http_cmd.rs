use crate::db::DbState;
use crate::http::{cancelled_result, send_http_async};
use crate::models::{KeyValue, SendRequestInput, SendRequestResult};
use rusqlite::params;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::State;
use tokio::sync::oneshot;

/// Tracks the in-flight Send so the UI can cancel it.
pub struct SendCancelState {
    tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl SendCancelState {
    pub fn new() -> Self {
        Self {
            tx: Mutex::new(None),
        }
    }

    fn register(&self) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        if let Ok(mut guard) = self.tx.lock() {
            if let Some(prev) = guard.take() {
                let _ = prev.send(());
            }
            *guard = Some(tx);
        }
        rx
    }

    fn clear(&self) {
        if let Ok(mut guard) = self.tx.lock() {
            guard.take();
        }
    }

    pub fn cancel(&self) {
        if let Ok(mut guard) = self.tx.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        }
    }
}

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

struct MockConfig {
    status: i64,
    headers_json: String,
    body: String,
    delay_ms: i64,
}

fn load_mock_config(
    conn: &rusqlite::Connection,
    request_id: Option<i64>,
) -> Result<Option<MockConfig>, String> {
    let Some(request_id) = request_id else {
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

    Ok(Some(MockConfig {
        status,
        headers_json,
        body,
        delay_ms,
    }))
}

fn write_history(
    conn: &rusqlite::Connection,
    input: &SendRequestInput,
    result: &SendRequestResult,
) {
    let request_snapshot = serde_json::to_string(input).unwrap_or_default();
    let response_snapshot = serde_json::to_string(result).unwrap_or_default();
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
}

#[tauri::command]
pub async fn send_request(
    state: State<'_, DbState>,
    cancel_state: State<'_, SendCancelState>,
    mut input: SendRequestInput,
) -> Result<SendRequestResult, String> {
    let (vars, mock) = {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        let vars = load_env_vars(&conn, input.environment_id)?;

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

        let mock = load_mock_config(&conn, input.request_id)?;
        (vars, mock)
    };

    let mut cancel_rx = cancel_state.register();
    let started = Instant::now();
    let url_for_cancel = input.url.clone();

    let result = if let Some(mock) = mock {
        if mock.delay_ms > 0 {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(mock.delay_ms as u64)) => {}
                _ = &mut cancel_rx => {
                    return Ok(cancelled_result(&url_for_cancel, started.elapsed().as_millis()));
                }
            }
        }

        let headers: Vec<KeyValue> = serde_json::from_str(&mock.headers_json).unwrap_or_default();
        let status_u16 = mock.status.clamp(100, 599) as u16;
        SendRequestResult {
            status: status_u16,
            status_text: "MOCK".into(),
            duration_ms: mock.delay_ms.max(0) as u128,
            headers,
            body: mock.body,
            url: input.url.clone(),
            error: None,
            mocked: true,
        }
    } else {
        tokio::select! {
            res = send_http_async(&input, &vars) => res,
            _ = &mut cancel_rx => {
                return Ok(cancelled_result(&url_for_cancel, started.elapsed().as_millis()));
            }
        }
    };

    cancel_state.clear();

    // Cancelled results should not pollute history
    if result.status_text != "CANCELLED" {
        if let Ok(conn) = state.conn.lock() {
            write_history(&conn, &input, &result);
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn cancel_request(cancel_state: State<'_, SendCancelState>) -> Result<(), String> {
    cancel_state.cancel();
    Ok(())
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
