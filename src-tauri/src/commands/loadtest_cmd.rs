use crate::db::DbState;
use crate::http::join_base_url;
use crate::loadtest::{run_load_test, LoadTestInput, LoadTestResult};
use crate::models::KeyValue;
use rusqlite::params;
use serde::Deserialize;
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Clone, Deserialize)]
pub struct RunLoadTestInput {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub params: Vec<KeyValue>,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default = "default_body_none")]
    pub body_type: String,
    #[serde(default)]
    pub body_content: String,
    /// When set, apply that request's collection base_url to relative URLs.
    pub request_id: Option<i64>,
    pub environment_id: Option<i64>,
    #[serde(default = "default_threads")]
    pub threads: u32,
    #[serde(default = "default_loops")]
    pub loops: u32,
    #[serde(default)]
    pub ramp_up_secs: f64,
}

fn default_body_none() -> String {
    "none".into()
}
fn default_threads() -> u32 {
    10
}
fn default_loops() -> u32 {
    10
}

fn load_env_vars(
    conn: &rusqlite::Connection,
    environment_id: Option<i64>,
) -> Result<HashMap<String, String>, String> {
    let env_id = match environment_id {
        Some(id) => id,
        None => match conn.query_row(
            "SELECT e.id FROM environments e
             INNER JOIN workspaces w ON e.workspace_id = w.id
             WHERE w.is_active = 1 AND e.is_active = 1
             LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(id) => id,
            Err(_) => return Ok(HashMap::new()),
        },
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

#[tauri::command]
pub fn run_load_test_cmd(
    state: State<'_, DbState>,
    input: RunLoadTestInput,
) -> Result<LoadTestResult, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let env_vars = load_env_vars(&conn, input.environment_id)?;

    let mut url = input.url;
    if let Some(rid) = input.request_id {
        let base: String = conn
            .query_row(
                "SELECT COALESCE(c.base_url, '')
                 FROM requests r
                 LEFT JOIN collections c ON r.collection_id = c.id
                 WHERE r.id = ?1",
                params![rid],
                |row| row.get(0),
            )
            .unwrap_or_default();
        if !base.trim().is_empty() {
            url = join_base_url(&base, &url);
        }
    }
    drop(conn);

    run_load_test(LoadTestInput {
        method: input.method,
        url,
        params: input.params,
        headers: input.headers,
        body_type: input.body_type,
        body_content: input.body_content,
        threads: input.threads,
        loops: input.loops,
        ramp_up_secs: input.ramp_up_secs,
        env_vars,
    })
}
