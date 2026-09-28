//! End-to-end feature smoke tests for ApiTest core (DB + HTTP + mock + workspace).
//! Run: `cargo test --manifest-path src-tauri/Cargo.toml`

use crate::db;
use crate::http::{send_http, substitute};
use crate::models::{KeyValue, SendRequestInput};
use rusqlite::params;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_db_path(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("apitest_{tag}_{nanos}.db"))
}

fn open_temp_db(tag: &str) -> (rusqlite::Connection, PathBuf) {
    let path = temp_db_path(tag);
    let _ = fs::remove_file(&path);
    let conn = db::open_and_migrate(&path).expect("open_and_migrate");
    (conn, path)
}

#[test]
fn migrate_and_seed_ok() {
    let (conn, path) = open_temp_db("seed");
    let version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(version, "5");

    let col_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM collections", [], |r| r.get(0))
        .unwrap();
    assert!(col_count >= 1, "seed collection missing");

    let has_base: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('collections') WHERE name = 'base_url'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(has_base, 1, "collections.base_url column missing");

    let req_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM requests", [], |r| r.get(0))
        .unwrap();
    assert!(req_count >= 1, "seed request missing");

    let ws_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM workspaces", [], |r| r.get(0))
        .unwrap();
    assert!(ws_count >= 1, "default workspace missing");

    let _ = fs::remove_file(path);
}

#[test]
fn collection_request_crud() {
    let (conn, path) = open_temp_db("crud");
    let ws_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE is_active = 1 LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();

    conn.execute(
        "INSERT INTO collections (name, sort_order, workspace_id) VALUES ('Auto Col', 1, ?1)",
        params![ws_id],
    )
    .unwrap();
    let col_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO requests (collection_id, name, method, url, headers_json, body_type)
         VALUES (?1, 'Echo', 'GET', 'https://httpbin.org/get', '[]', 'none')",
        params![col_id],
    )
    .unwrap();
    let req_id = conn.last_insert_rowid();

    conn.execute(
        "UPDATE requests SET name = 'Echo Renamed', method = 'POST' WHERE id = ?1",
        params![req_id],
    )
    .unwrap();

    let (name, method): (String, String) = conn
        .query_row(
            "SELECT name, method FROM requests WHERE id = ?1",
            params![req_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "Echo Renamed");
    assert_eq!(method, "POST");

    conn.execute("DELETE FROM requests WHERE id = ?1", params![req_id])
        .unwrap();
    let left: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM requests WHERE id = ?1",
            params![req_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(left, 0);

    conn.execute("DELETE FROM collections WHERE id = ?1", params![col_id])
        .unwrap();

    let _ = fs::remove_file(path);
}

#[test]
fn collection_base_url_persists_and_joins() {
    let (conn, path) = open_temp_db("baseurl");
    let ws_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE is_active = 1 LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();

    conn.execute(
        "INSERT INTO collections (name, workspace_id, base_url) VALUES ('Prefixed', ?1, ?2)",
        params![ws_id, "https://api.example.com/v1"],
    )
    .unwrap();
    let col_id = conn.last_insert_rowid();

    let base: String = conn
        .query_row(
            "SELECT base_url FROM collections WHERE id = ?1",
            params![col_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(base, "https://api.example.com/v1");

    assert_eq!(
        crate::http::join_base_url(&base, "/pets"),
        "https://api.example.com/v1/pets"
    );
    assert_eq!(
        crate::http::join_base_url(&base, "https://other.com/x"),
        "https://other.com/x"
    );

    let _ = fs::remove_file(path);
}

#[test]
fn env_var_substitution() {
    let mut vars = HashMap::new();
    vars.insert("baseUrl".into(), "https://httpbin.org".into());
    vars.insert("token".into(), "abc123".into());

    let url = substitute("{{baseUrl}}/get?t={{token}}", &vars);
    assert_eq!(url, "https://httpbin.org/get?t=abc123");

    let header = substitute("Bearer {{token}}", &vars);
    assert_eq!(header, "Bearer abc123");

    let untouched = substitute("no vars here", &vars);
    assert_eq!(untouched, "no vars here");
}

#[test]
fn environment_vars_persist() {
    let (conn, path) = open_temp_db("env");
    let ws_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE is_active = 1 LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();

    conn.execute(
        "INSERT INTO environments (name, is_active, workspace_id) VALUES ('CI', 1, ?1)",
        params![ws_id],
    )
    .unwrap();
    let env_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO environment_vars (environment_id, key, value, enabled)
         VALUES (?1, 'baseUrl', 'https://example.com', 1)",
        params![env_id],
    )
    .unwrap();

    let value: String = conn
        .query_row(
            "SELECT value FROM environment_vars WHERE environment_id = ?1 AND key = 'baseUrl'",
            params![env_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(value, "https://example.com");

    let _ = fs::remove_file(path);
}

#[test]
fn workspace_isolation() {
    let (conn, path) = open_temp_db("ws");

    conn.execute(
        "INSERT INTO workspaces (name, is_active) VALUES ('Second', 0)",
        [],
    )
    .unwrap();
    let ws2 = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO collections (name, sort_order, workspace_id) VALUES ('WS2 Only', 0, ?1)",
        params![ws2],
    )
    .unwrap();

    let ws1: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE is_active = 1 LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();

    let in_ws1: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM collections WHERE workspace_id = ?1 AND name = 'WS2 Only'",
            params![ws1],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(in_ws1, 0);

    let in_ws2: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM collections WHERE workspace_id = ?1 AND name = 'WS2 Only'",
            params![ws2],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(in_ws2, 1);

    let _ = fs::remove_file(path);
}

#[test]
fn theme_setting() {
    let (conn, path) = open_temp_db("theme");
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('theme', 'dark')",
        [],
    )
    .unwrap();
    let theme: String = conn
        .query_row("SELECT value FROM meta WHERE key = 'theme'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(theme, "dark");
    let _ = fs::remove_file(path);
}

#[test]
fn mock_fields_on_request() {
    let (conn, path) = open_temp_db("mock");
    let col_id: i64 = conn
        .query_row("SELECT id FROM collections LIMIT 1", [], |r| r.get(0))
        .unwrap();

    conn.execute(
        "INSERT INTO requests (
            collection_id, name, method, url, headers_json, body_type,
            mock_enabled, mock_status, mock_body, mock_delay_ms, mock_headers_json
         ) VALUES (?1, 'Mocked', 'GET', 'https://example.com', '[]', 'none', 1, 201, '{\"ok\":true}', 0, '[]')",
        params![col_id],
    )
    .unwrap();
    let req_id = conn.last_insert_rowid();

    let (enabled, status, body): (i64, i64, String) = conn
        .query_row(
            "SELECT mock_enabled, mock_status, mock_body FROM requests WHERE id = ?1",
            params![req_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(enabled, 1);
    assert_eq!(status, 201);
    assert!(body.contains("ok"));

    let _ = fs::remove_file(path);
}

#[test]
fn import_export_json_roundtrip_shape() {
    // mirrors io.rs collection export format
    let payload = serde_json::json!({
        "format": "apitest-collection",
        "version": 1,
        "name": "Imported",
        "requests": [{
            "name": "Ping",
            "method": "GET",
            "url": "https://httpbin.org/get",
            "params": [],
            "headers": [],
            "body_type": "none",
            "body_content": ""
        }]
    });
    assert_eq!(payload["format"], "apitest-collection");
    assert_eq!(payload["requests"].as_array().unwrap().len(), 1);

    let (conn, path) = open_temp_db("io");
    let ws_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE is_active = 1 LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    conn.execute(
        "INSERT INTO collections (name, sort_order, workspace_id) VALUES (?1, 0, ?2)",
        params![payload["name"].as_str().unwrap(), ws_id],
    )
    .unwrap();
    let col_id = conn.last_insert_rowid();
    for req in payload["requests"].as_array().unwrap() {
        conn.execute(
            "INSERT INTO requests (collection_id, name, method, url, headers_json, body_type, body_content)
             VALUES (?1, ?2, ?3, ?4, '[]', ?5, ?6)",
            params![
                col_id,
                req["name"].as_str().unwrap(),
                req["method"].as_str().unwrap(),
                req["url"].as_str().unwrap(),
                req["body_type"].as_str().unwrap(),
                req["body_content"].as_str().unwrap_or("")
            ],
        )
        .unwrap();
    }
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM requests WHERE collection_id = ?1",
            params![col_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
    let _ = fs::remove_file(path);
}

#[test]
fn history_insert_and_clear() {
    let (conn, path) = open_temp_db("hist");
    conn.execute(
        "INSERT INTO history (request_id, method, url, status_code, duration_ms, request_snapshot, response_snapshot)
         VALUES (NULL, 'GET', 'https://example.com', 200, 12, '{}', '{}')",
        [],
    )
    .unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
        .unwrap();
    assert!(n >= 1);
    conn.execute("DELETE FROM history", []).unwrap();
    let left: i64 = conn
        .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 0);
    let _ = fs::remove_file(path);
}

#[test]
fn http_get_httpbin_ip() {
    // network smoke — skip soft-fail if offline
    let input = SendRequestInput {
        method: "GET".into(),
        url: "https://httpbin.org/ip".into(),
        params: vec![],
        headers: vec![],
        body_type: "none".into(),
        body_content: String::new(),
        body_language: String::new(),
        request_id: None,
        environment_id: None,
    };
    let vars = HashMap::new();
    let result = send_http(&input, &vars);
    if let Some(err) = &result.error {
        eprintln!("WARN http_get_httpbin_ip skipped (network): {err}");
        return;
    }
    assert_eq!(result.status, 200);
    assert!(!result.mocked);
    assert!(
        result.body.contains("origin") || result.body.contains("{"),
        "unexpected body: {}",
        result.body
    );
}

#[test]
fn http_get_with_env_substitution() {
    let mut vars = HashMap::new();
    vars.insert("baseUrl".into(), "https://httpbin.org".into());
    let input = SendRequestInput {
        method: "GET".into(),
        url: "{{baseUrl}}/get".into(),
        params: vec![KeyValue::text("from", "apitest")],
        headers: vec![KeyValue::text("X-ApiTest", "1")],
        body_type: "none".into(),
        body_content: String::new(),
        body_language: String::new(),
        request_id: None,
        environment_id: None,
    };
    let result = send_http(&input, &vars);
    if let Some(err) = &result.error {
        eprintln!("WARN http_get_with_env_substitution skipped (network): {err}");
        return;
    }
    assert_eq!(result.status, 200);
    assert!(result.url.contains("httpbin.org"));
    assert!(result.url.contains("from=apitest") || result.body.contains("apitest"));
}

#[test]
fn http_post_json() {
    let input = SendRequestInput {
        method: "POST".into(),
        url: "https://httpbin.org/post".into(),
        params: vec![],
        headers: vec![],
        body_type: "json".into(),
        body_content: r#"{"hello":"apitest"}"#.into(),
        body_language: String::new(),
        request_id: None,
        environment_id: None,
    };
    let result = send_http(&input, &HashMap::new());
    if let Some(err) = &result.error {
        eprintln!("WARN http_post_json skipped (network): {err}");
        return;
    }
    assert_eq!(result.status, 200);
    assert!(result.body.contains("apitest") || result.body.contains("hello"));
}

#[test]
fn form_data_file_field_reads_path() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "apitest_upload_{}.txt",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    {
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"hello-apitest-upload").unwrap();
    }
    let path_str = path.to_string_lossy().to_string();
    let body = serde_json::json!([
        {
            "key": "note",
            "value": "hi",
            "enabled": true,
            "type": "text"
        },
        {
            "key": "file",
            "value": "upload.txt",
            "enabled": true,
            "type": "file",
            "file_path": path_str,
            "file_name": "upload.txt"
        }
    ])
    .to_string();

    let input = SendRequestInput {
        method: "POST".into(),
        url: "https://httpbin.org/post".into(),
        params: vec![],
        headers: vec![],
        body_type: "form-data".into(),
        body_content: body,
        body_language: String::new(),
        request_id: None,
        environment_id: None,
    };
    let result = send_http(&input, &HashMap::new());
    let _ = fs::remove_file(&path);
    if let Some(err) = &result.error {
        eprintln!("WARN form_data_file_field skipped (network): {err}");
        return;
    }
    assert!(
        (200..300).contains(&result.status),
        "unexpected status {}",
        result.status
    );
    assert!(
        result.body.contains("hello-apitest-upload") || result.body.contains("upload.txt"),
        "response should include uploaded content or filename"
    );
}

#[test]
fn pre_and_test_script_columns() {
    let (conn, path) = open_temp_db("scripts");
    let col_id: i64 = conn
        .query_row("SELECT id FROM collections LIMIT 1", [], |r| r.get(0))
        .unwrap();
    conn.execute(
        "INSERT INTO requests (collection_id, name, method, url, headers_json, body_type, pre_script, test_script)
         VALUES (?1, 'Scripted', 'GET', 'https://httpbin.org/get', '[]', 'none',
                 'at.env.set(\"x\",\"1\")',
                 'at.test(\"ok\", () => at.expect(at.response.status).toBe(200))')",
        params![col_id],
    )
    .unwrap();
    let (pre, test): (String, String) = conn
        .query_row(
            "SELECT pre_script, test_script FROM requests WHERE name = 'Scripted'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(pre.contains("at.env.set"));
    assert!(test.contains("at.test"));
    let _ = fs::remove_file(path);
}
