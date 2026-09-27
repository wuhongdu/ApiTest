use crate::db::DbState;
use crate::models::{
    Collection, ExportCollection, ExportRequest, ImportCollectionInput, KeyValue,
};
use crate::openapi::{looks_like_openapi, openapi_to_collection, parse_import_json};
use rusqlite::params;
use tauri::State;

fn parse_kv(json: &str) -> Vec<KeyValue> {
    serde_json::from_str(json).unwrap_or_default()
}

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

fn resolve_import_payload(raw: &str) -> Result<ExportCollection, String> {
    let value = parse_import_json(raw)?;
    if looks_like_openapi(&value) {
        return openapi_to_collection(&value);
    }

    let data: ExportCollection = serde_json::from_value(value).map_err(|e| {
        format!("无法识别导入内容（支持 ApiTest JSON / Swagger 2 / OpenAPI 3）: {e}")
    })?;

    if !data.format.is_empty() && data.format != "apitest-collection" {
        return Err(format!("不支持的导入格式: {}", data.format));
    }
    if data.requests.is_empty() && data.name.trim().is_empty() {
        return Err("导入内容为空".into());
    }
    Ok(data)
}

fn insert_collection(
    conn: &rusqlite::Connection,
    data: &ExportCollection,
) -> Result<Collection, String> {
    let name = if data.name.trim().is_empty() {
        "Imported Collection".into()
    } else {
        data.name.trim().to_string()
    };

    let ws = active_workspace_id(conn)?;
    conn.execute(
        "INSERT INTO collections (name, workspace_id, base_url) VALUES (?1, ?2, ?3)",
        params![name, ws, data.base_url.trim()],
    )
    .map_err(|e| e.to_string())?;
    let collection_id = conn.last_insert_rowid();

    for (idx, req) in data.requests.iter().enumerate() {
        let params_json = serde_json::to_string(&req.params).unwrap_or_else(|_| "[]".into());
        let headers_json = serde_json::to_string(&req.headers).unwrap_or_else(|_| "[]".into());
        let mock_headers_json =
            serde_json::to_string(&req.mock_headers).unwrap_or_else(|_| "[]".into());
        let method = if req.method.trim().is_empty() {
            "GET".into()
        } else {
            req.method.to_uppercase()
        };
        let req_name = if req.name.trim().is_empty() {
            format!("Request {}", idx + 1)
        } else {
            req.name.clone()
        };
        let body_type = if req.body_type.trim().is_empty() {
            "none".into()
        } else {
            req.body_type.clone()
        };
        let mock_enabled = if req.mock_enabled { 1 } else { 0 };

        conn.execute(
            "INSERT INTO requests
             (collection_id, name, method, url, params_json, headers_json, body_type, body_content,
              sort_order, pre_script, test_script, mock_enabled, mock_status, mock_headers_json,
              mock_body, mock_delay_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                collection_id,
                req_name,
                method,
                req.url,
                params_json,
                headers_json,
                body_type,
                req.body_content,
                idx as i64,
                req.pre_script,
                req.test_script,
                mock_enabled,
                req.mock_status,
                mock_headers_json,
                req.mock_body,
                req.mock_delay_ms
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    conn.query_row(
        "SELECT id, parent_id, name, sort_order, workspace_id, base_url FROM collections WHERE id = ?1",
        params![collection_id],
        |row| {
            Ok(Collection {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                sort_order: row.get(3)?,
                workspace_id: row.get(4)?,
                base_url: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_collection(
    state: State<'_, DbState>,
    collection_id: i64,
) -> Result<ExportCollection, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let (name, base_url): (String, String) = conn
        .query_row(
            "SELECT name, COALESCE(base_url, '') FROM collections WHERE id = ?1",
            params![collection_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "集合不存在".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT name, method, url, params_json, headers_json, body_type, body_content,
                    pre_script, test_script, mock_enabled, mock_status, mock_headers_json,
                    mock_body, mock_delay_ms
             FROM requests WHERE collection_id = ?1 ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;

    let requests = stmt
        .query_map(params![collection_id], |row| {
            let params_json: String = row.get(3)?;
            let headers_json: String = row.get(4)?;
            let mock_headers_json: String = row.get(11)?;
            let mock_enabled: i64 = row.get(9)?;
            Ok(ExportRequest {
                name: row.get(0)?,
                method: row.get(1)?,
                url: row.get(2)?,
                params: parse_kv(&params_json),
                headers: parse_kv(&headers_json),
                body_type: row.get(5)?,
                body_content: row.get(6)?,
                pre_script: row.get(7)?,
                test_script: row.get(8)?,
                mock_enabled: mock_enabled == 1,
                mock_status: row.get(10)?,
                mock_headers: parse_kv(&mock_headers_json),
                mock_body: row.get(12)?,
                mock_delay_ms: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ExportCollection {
        format: "apitest-collection".into(),
        version: 1,
        name,
        base_url,
        requests,
    })
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExportJmeterInput {
    pub collection_id: i64,
    #[serde(default = "default_jmeter_threads")]
    pub threads: u32,
    #[serde(default = "default_jmeter_loops")]
    pub loops: i32,
    #[serde(default = "default_jmeter_ramp")]
    pub ramp_up: u32,
}

fn default_jmeter_threads() -> u32 {
    10
}
fn default_jmeter_loops() -> i32 {
    10
}
fn default_jmeter_ramp() -> u32 {
    1
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportJmeterResult {
    pub name: String,
    pub jmx: String,
}

#[tauri::command]
pub fn export_jmeter(
    state: State<'_, DbState>,
    input: ExportJmeterInput,
) -> Result<ExportJmeterResult, String> {
    let data = export_collection(state, input.collection_id)?;
    let jmx = crate::jmeter::collection_to_jmx(&data, input.threads, input.loops, input.ramp_up);
    Ok(ExportJmeterResult {
        name: data.name,
        jmx,
    })
}

#[tauri::command]
pub fn import_collection(
    state: State<'_, DbState>,
    input: ImportCollectionInput,
) -> Result<Collection, String> {
    let data = resolve_import_payload(&input.json)?;
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    insert_collection(&conn, &data)
}
