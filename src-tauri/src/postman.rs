//! Convert Postman Collection v2.0 / v2.1 JSON into ApiTest collections.

use crate::models::{ExportCollection, ExportRequest, KeyValue};
use serde_json::Value;

fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, b) in chunk.iter().enumerate() {
            buf[i] = *b;
        }
        let n = (u32::from(buf[0]) << 16) | (u32::from(buf[1]) << 8) | u32::from(buf[2]);
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn kv(key: &str, value: &str) -> KeyValue {
    KeyValue::text(key, value)
}

fn kv_disabled(key: &str, value: &str, enabled: bool) -> KeyValue {
    let mut row = KeyValue::text(key, value);
    row.enabled = enabled;
    row
}

fn str_field<'a>(obj: &'a Value, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(|v| v.as_str())
}

fn is_disabled(obj: &Value) -> bool {
    obj.get("disabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Detect Postman Collection v2.x export JSON.
pub fn looks_like_postman(root: &Value) -> bool {
    if !root.is_object() {
        return false;
    }
    // Explicit schema URL is the strongest signal
    if let Some(schema) = root
        .get("info")
        .and_then(|i| str_field(i, "schema"))
    {
        let lower = schema.to_ascii_lowercase();
        if lower.contains("getpostman.com") || lower.contains("postman") {
            return true;
        }
    }
    // Heuristic: info + item[], without OpenAPI markers
    let has_info = root.get("info").map(|v| v.is_object()).unwrap_or(false);
    let has_item = root.get("item").map(|v| v.is_array()).unwrap_or(false);
    let is_openapi = root.get("openapi").is_some() || root.get("swagger").is_some();
    let is_apitest = root
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s == "apitest-collection")
        .unwrap_or(false);
    has_info && has_item && !is_openapi && !is_apitest
}

fn empty_request(name: String, method: String, url: String) -> ExportRequest {
    ExportRequest {
        name,
        method,
        url,
        params: vec![],
        headers: vec![],
        body_type: "none".into(),
        body_content: String::new(),
        body_language: String::new(),
        pre_script: String::new(),
        test_script: String::new(),
        mock_enabled: false,
        mock_status: 200,
        mock_headers: vec![],
        mock_body: String::new(),
        mock_delay_ms: 0,
    }
}

fn join_script_exec(script: &Value) -> String {
    if let Some(s) = script.as_str() {
        return s.to_string();
    }
    let Some(obj) = script.as_object() else {
        return String::new();
    };
    if let Some(exec) = obj.get("exec") {
        if let Some(arr) = exec.as_array() {
            return arr
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("\n");
        }
        if let Some(s) = exec.as_str() {
            return s.to_string();
        }
    }
    String::new()
}

fn extract_scripts(item: &Value) -> (String, String) {
    let mut pre = String::new();
    let mut test = String::new();
    let Some(events) = item.get("event").and_then(|e| e.as_array()) else {
        return (pre, test);
    };
    for ev in events {
        let listen = str_field(ev, "listen").unwrap_or("");
        let script = match ev.get("script") {
            Some(s) => join_script_exec(s),
            None => continue,
        };
        if script.trim().is_empty() {
            continue;
        }
        match listen {
            "prerequest" => {
                if !pre.is_empty() {
                    pre.push_str("\n");
                }
                pre.push_str(&script);
            }
            "test" => {
                if !test.is_empty() {
                    test.push_str("\n");
                }
                test.push_str(&script);
            }
            _ => {}
        }
    }
    (pre, test)
}

fn parse_query(url_obj: &Value) -> Vec<KeyValue> {
    let mut out = Vec::new();
    let Some(query) = url_obj.get("query").and_then(|q| q.as_array()) else {
        return out;
    };
    for q in query {
        let key = str_field(q, "key").unwrap_or("").to_string();
        if key.is_empty() {
            continue;
        }
        let value = str_field(q, "value").unwrap_or("").to_string();
        out.push(kv_disabled(&key, &value, !is_disabled(q)));
    }
    out
}

fn resolve_url(request: &Value) -> (String, Vec<KeyValue>) {
    let url_val = match request.get("url") {
        Some(u) => u,
        None => {
            // v2.0 sometimes uses request as a plain URL string
            if let Some(s) = request.as_str() {
                return (s.to_string(), vec![]);
            }
            return (String::new(), vec![]);
        }
    };

    if let Some(s) = url_val.as_str() {
        return (s.to_string(), vec![]);
    }

    let raw = str_field(url_val, "raw").unwrap_or("").to_string();
    let params = parse_query(url_val);

    if !raw.is_empty() {
        // Prefer raw URL; query params are also kept in params for editing
        // Strip query string from raw when we have structured query to avoid duplication
        let url = if !params.is_empty() {
            raw.split('?').next().unwrap_or(&raw).to_string()
        } else {
            raw
        };
        return (url, params);
    }

    // Reconstruct from host + path
    let protocol = str_field(url_val, "protocol").unwrap_or("https");
    let host = url_val
        .get("host")
        .and_then(|h| h.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(".")
        })
        .unwrap_or_default();
    let path = url_val
        .get("path")
        .and_then(|p| p.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_default();

    // Apply path variables (:id → value)
    let mut path = path;
    if let Some(vars) = url_val.get("variable").and_then(|v| v.as_array()) {
        for v in vars {
            let key = str_field(v, "key").unwrap_or("");
            let val = str_field(v, "value").unwrap_or("");
            if !key.is_empty() {
                path = path.replace(&format!(":{key}"), val);
                path = path.replace(&format!("{{{{{key}}}}}"), val);
            }
        }
    }

    let url = if host.is_empty() {
        if path.is_empty() {
            String::new()
        } else if path.starts_with('/') || path.starts_with('{') {
            path
        } else {
            format!("/{path}")
        }
    } else {
        let path = if path.is_empty() {
            String::new()
        } else if path.starts_with('/') {
            path
        } else {
            format!("/{path}")
        };
        format!("{protocol}://{host}{path}")
    };

    (url, params)
}

fn parse_headers(request: &Value) -> Vec<KeyValue> {
    let mut out = Vec::new();
    let Some(headers) = request.get("header").and_then(|h| h.as_array()) else {
        return out;
    };
    for h in headers {
        let key = str_field(h, "key").unwrap_or("").to_string();
        if key.is_empty() {
            continue;
        }
        let value = str_field(h, "value").unwrap_or("").to_string();
        out.push(kv_disabled(&key, &value, !is_disabled(h)));
    }
    out
}

fn apply_auth(request: &Value, headers: &mut Vec<KeyValue>) {
    let auth = match request.get("auth") {
        Some(a) if a.is_object() => a,
        _ => return,
    };
    let auth_type = str_field(auth, "type").unwrap_or("");
    let already_has_auth = headers
        .iter()
        .any(|h| h.key.eq_ignore_ascii_case("authorization"));

    match auth_type {
        "bearer" => {
            if already_has_auth {
                return;
            }
            let token = auth
                .get("bearer")
                .and_then(|b| b.as_array())
                .and_then(|arr| {
                    arr.iter().find_map(|row| {
                        if str_field(row, "key") == Some("token") {
                            str_field(row, "value").map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
                .or_else(|| {
                    // Some exports use object form
                    auth.get("bearer")
                        .and_then(|b| str_field(b, "token"))
                        .map(|s| s.to_string())
                })
                .unwrap_or_default();
            if !token.is_empty() {
                headers.push(kv("Authorization", &format!("Bearer {token}")));
            }
        }
        "basic" => {
            if already_has_auth {
                return;
            }
            let get_basic = |key: &str| -> String {
                auth.get("basic")
                    .and_then(|b| b.as_array())
                    .and_then(|arr| {
                        arr.iter().find_map(|row| {
                            if str_field(row, "key") == Some(key) {
                                str_field(row, "value").map(|s| s.to_string())
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or_default()
            };
            let user = get_basic("username");
            let pass = get_basic("password");
            if !user.is_empty() || !pass.is_empty() {
                let encoded = encode_base64(format!("{user}:{pass}").as_bytes());
                headers.push(kv("Authorization", &format!("Basic {encoded}")));
            }
        }
        "apikey" => {
            let (key, value, location) = {
                let arr = auth.get("apikey").and_then(|a| a.as_array());
                let mut key = String::new();
                let mut value = String::new();
                let mut location = "header".to_string();
                if let Some(arr) = arr {
                    for row in arr {
                        match str_field(row, "key") {
                            Some("key") => key = str_field(row, "value").unwrap_or("").to_string(),
                            Some("value") => {
                                value = str_field(row, "value").unwrap_or("").to_string()
                            }
                            Some("in") => {
                                location = str_field(row, "value").unwrap_or("header").to_string()
                            }
                            _ => {}
                        }
                    }
                }
                (key, value, location)
            };
            if !key.is_empty() && location.eq_ignore_ascii_case("header") {
                if !headers.iter().any(|h| h.key.eq_ignore_ascii_case(&key)) {
                    headers.push(kv(&key, &value));
                }
            }
        }
        _ => {}
    }
}

fn looks_like_json(s: &str) -> bool {
    let t = s.trim();
    (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']'))
}

fn parse_body(request: &Value) -> (String, String, String, Vec<KeyValue>) {
    let body = match request.get("body") {
        Some(b) if !b.is_null() => b,
        _ => return ("none".into(), String::new(), String::new(), vec![]),
    };
    let mode = str_field(body, "mode").unwrap_or("raw");

    match mode {
        "raw" => {
            let content = str_field(body, "raw").unwrap_or("").to_string();
            let language = body
                .get("options")
                .and_then(|o| o.get("raw"))
                .and_then(|r| str_field(r, "language"))
                .unwrap_or("")
                .to_ascii_lowercase();
            let language = if language.is_empty() {
                if looks_like_json(&content) {
                    "json".into()
                } else {
                    "text".into()
                }
            } else if language == "js" {
                "javascript".into()
            } else {
                language
            };
            let mut extra_headers = Vec::new();
            if language == "json" {
                extra_headers.push(kv("Content-Type", "application/json"));
            }
            ("raw".into(), content, language, extra_headers)
        }
        "urlencoded" => {
            let mut rows = Vec::new();
            if let Some(arr) = body.get("urlencoded").and_then(|u| u.as_array()) {
                for row in arr {
                    let key = str_field(row, "key").unwrap_or("").to_string();
                    if key.is_empty() {
                        continue;
                    }
                    let value = str_field(row, "value").unwrap_or("").to_string();
                    rows.push(kv_disabled(&key, &value, !is_disabled(row)));
                }
            }
            let content = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());
            (
                "x-www-form-urlencoded".into(),
                content,
                String::new(),
                vec![kv("Content-Type", "application/x-www-form-urlencoded")],
            )
        }
        "formdata" => {
            let mut rows = Vec::new();
            if let Some(arr) = body.get("formdata").and_then(|f| f.as_array()) {
                for row in arr {
                    let key = str_field(row, "key").unwrap_or("").to_string();
                    if key.is_empty() {
                        continue;
                    }
                    let field_type = str_field(row, "type").unwrap_or("text");
                    let enabled = !is_disabled(row);
                    if field_type == "file" {
                        let src = row
                            .get("src")
                            .and_then(|s| {
                                if let Some(st) = s.as_str() {
                                    Some(st.to_string())
                                } else if let Some(arr) = s.as_array() {
                                    arr.first()
                                        .and_then(|v| v.as_str())
                                        .map(|st| st.to_string())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default();
                        let file_name = std::path::Path::new(&src)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        rows.push(KeyValue {
                            key,
                            value: String::new(),
                            enabled,
                            field_type: "file".into(),
                            file_path: src,
                            file_name,
                        });
                    } else {
                        let value = str_field(row, "value").unwrap_or("").to_string();
                        rows.push(kv_disabled(&key, &value, enabled));
                    }
                }
            }
            let content = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());
            ("form-data".into(), content, String::new(), vec![])
        }
        "graphql" => {
            let gql = body.get("graphql");
            let query = gql
                .and_then(|g| str_field(g, "query"))
                .unwrap_or("")
                .to_string();
            let variables = gql
                .and_then(|g| g.get("variables"))
                .map(|v| {
                    if let Some(s) = v.as_str() {
                        s.to_string()
                    } else {
                        serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".into())
                    }
                })
                .unwrap_or_else(|| "{}".into());
            let payload = serde_json::json!({
                "query": query,
                "variables": variables,
            });
            let content = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into());
            (
                "graphql".into(),
                content,
                String::new(),
                vec![kv("Content-Type", "application/json")],
            )
        }
        "file" => {
            let src = body
                .get("file")
                .and_then(|f| str_field(f, "src"))
                .unwrap_or("");
            if src.is_empty() {
                ("binary".into(), String::new(), String::new(), vec![])
            } else {
                let file_name = std::path::Path::new(src)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                let meta = serde_json::json!({
                    "file_path": src,
                    "file_name": file_name,
                });
                (
                    "binary".into(),
                    serde_json::to_string(&meta).unwrap_or_default(),
                    String::new(),
                    vec![],
                )
            }
        }
        _ => ("none".into(), String::new(), String::new(), vec![]),
    }
}

fn convert_request_item(item: &Value, folder_prefix: &str) -> Option<ExportRequest> {
    let request = item.get("request")?;
    // Folder items have `item` array and usually no usable request object as HTTP
    // (some exports nest both — prefer real HTTP request)
    if request.is_null() {
        return None;
    }

    let name_raw = str_field(item, "name").unwrap_or("Untitled");
    let name = if folder_prefix.is_empty() {
        name_raw.to_string()
    } else {
        format!("{folder_prefix} / {name_raw}")
    };

    let method = if let Some(s) = request.as_str() {
        // request is just a URL string
        let mut req = empty_request(name, "GET".into(), s.to_string());
        let (pre, test) = extract_scripts(item);
        req.pre_script = pre;
        req.test_script = test;
        return Some(req);
    } else {
        str_field(request, "method")
            .unwrap_or("GET")
            .to_uppercase()
    };

    let (url, params) = resolve_url(request);
    let mut headers = parse_headers(request);
    apply_auth(request, &mut headers);

    let (body_type, body_content, body_language, mut body_headers) = parse_body(request);
    // Merge body content-type only if not already present
    for bh in body_headers.drain(..) {
        if !headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case(&bh.key))
        {
            headers.push(bh);
        }
    }

    let (pre, test) = extract_scripts(item);

    let mut req = empty_request(name, method, url);
    req.params = params;
    req.headers = headers;
    req.body_type = body_type;
    req.body_content = body_content;
    req.body_language = body_language;
    req.pre_script = pre;
    req.test_script = test;
    Some(req)
}

fn walk_items(items: &[Value], folder_prefix: &str, out: &mut Vec<ExportRequest>) {
    for item in items {
        // Folder: has nested `item` array
        if let Some(children) = item.get("item").and_then(|i| i.as_array()) {
            let folder_name = str_field(item, "name").unwrap_or("Folder");
            let next_prefix = if folder_prefix.is_empty() {
                folder_name.to_string()
            } else {
                format!("{folder_prefix} / {folder_name}")
            };
            // Folder may also have its own request (rare) — skip; walk children
            walk_items(children, &next_prefix, out);
            continue;
        }
        if let Some(req) = convert_request_item(item, folder_prefix) {
            out.push(req);
        }
    }
}

fn collection_base_url(doc: &Value) -> String {
    // Prefer collection variable named baseUrl / host / url
    let vars = match doc.get("variable").and_then(|v| v.as_array()) {
        Some(v) => v,
        None => return String::new(),
    };
    for preferred in ["baseUrl", "base_url", "host", "url"] {
        for v in vars {
            if str_field(v, "key") == Some(preferred) {
                let val = str_field(v, "value").unwrap_or("").trim().to_string();
                if !val.is_empty() {
                    return val;
                }
            }
        }
    }
    String::new()
}

pub fn postman_to_collection(doc: &Value) -> Result<ExportCollection, String> {
    if !looks_like_postman(doc) {
        return Err("不是有效的 Postman Collection".into());
    }

    let name = doc
        .get("info")
        .and_then(|i| str_field(i, "name"))
        .unwrap_or("Postman Import")
        .to_string();

    let items = doc
        .get("item")
        .and_then(|i| i.as_array())
        .ok_or_else(|| "Postman 集合缺少 item".to_string())?;

    let mut requests = Vec::new();
    walk_items(items, "", &mut requests);

    if requests.is_empty() {
        return Err("未从 Postman 集合中解析到任何请求".into());
    }

    Ok(ExportCollection {
        format: "apitest-collection".into(),
        version: 1,
        name,
        base_url: collection_base_url(doc),
        requests,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_postman_v21() {
        let doc = serde_json::json!({
            "info": {
                "name": "Demo",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": []
        });
        assert!(looks_like_postman(&doc));
        assert!(!crate::openapi::looks_like_openapi(&doc));
    }

    #[test]
    fn import_postman_with_folder_body_and_auth() {
        let raw = r#"{
          "info": {
            "name": "Shop API",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
          },
          "variable": [
            { "key": "baseUrl", "value": "https://api.shop.test" }
          ],
          "item": [
            {
              "name": "Users",
              "item": [
                {
                  "name": "List users",
                  "request": {
                    "method": "GET",
                    "header": [
                      { "key": "Accept", "value": "application/json" },
                      { "key": "X-Skip", "value": "1", "disabled": true }
                    ],
                    "url": {
                      "raw": "https://api.shop.test/users?page=1",
                      "protocol": "https",
                      "host": ["api", "shop", "test"],
                      "path": ["users"],
                      "query": [
                        { "key": "page", "value": "1" },
                        { "key": "limit", "value": "10", "disabled": true }
                      ]
                    }
                  }
                },
                {
                  "name": "Create user",
                  "event": [
                    {
                      "listen": "prerequest",
                      "script": { "exec": ["console.log('pre')"] }
                    },
                    {
                      "listen": "test",
                      "script": { "exec": ["pm.test('ok', function(){})"] }
                    }
                  ],
                  "request": {
                    "auth": {
                      "type": "bearer",
                      "bearer": [{ "key": "token", "value": "{{token}}", "type": "string" }]
                    },
                    "method": "POST",
                    "header": [],
                    "body": {
                      "mode": "raw",
                      "raw": "{\"name\":\"Ada\"}",
                      "options": { "raw": { "language": "json" } }
                    },
                    "url": "{{baseUrl}}/users"
                  }
                }
              ]
            },
            {
              "name": "Upload",
              "request": {
                "method": "POST",
                "body": {
                  "mode": "formdata",
                  "formdata": [
                    { "key": "title", "value": "pic", "type": "text" },
                    { "key": "file", "type": "file", "src": "/tmp/a.png" }
                  ]
                },
                "url": "https://api.shop.test/upload"
              }
            }
          ]
        }"#;
        let doc: Value = serde_json::from_str(raw).unwrap();
        let col = postman_to_collection(&doc).unwrap();
        assert_eq!(col.name, "Shop API");
        assert_eq!(col.base_url, "https://api.shop.test");
        assert_eq!(col.requests.len(), 3);

        let list = &col.requests[0];
        assert_eq!(list.name, "Users / List users");
        assert_eq!(list.method, "GET");
        assert_eq!(list.url, "https://api.shop.test/users");
        assert_eq!(list.params.len(), 2);
        assert!(list.params[0].enabled);
        assert!(!list.params[1].enabled);
        assert_eq!(list.headers.len(), 2);
        assert!(!list.headers[1].enabled);

        let create = &col.requests[1];
        assert_eq!(create.name, "Users / Create user");
        assert_eq!(create.body_type, "raw");
        assert_eq!(create.body_language, "json");
        assert!(create.body_content.contains("Ada"));
        assert!(create
            .headers
            .iter()
            .any(|h| h.key == "Authorization" && h.value == "Bearer {{token}}"));
        assert_eq!(create.pre_script, "console.log('pre')");
        assert!(create.test_script.contains("pm.test"));

        let upload = &col.requests[2];
        assert_eq!(upload.body_type, "form-data");
        let rows: Vec<KeyValue> = serde_json::from_str(&upload.body_content).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].field_type, "file");
        assert_eq!(rows[1].file_path, "/tmp/a.png");
    }

    #[test]
    fn import_urlencoded_and_string_url() {
        let doc = serde_json::json!({
            "info": { "name": "Simple", "schema": "https://schema.getpostman.com/json/collection/v2.0.0/collection.json" },
            "item": [
                {
                    "name": "Form",
                    "request": {
                        "method": "POST",
                        "body": {
                            "mode": "urlencoded",
                            "urlencoded": [
                                { "key": "a", "value": "1" },
                                { "key": "b", "value": "2", "disabled": true }
                            ]
                        },
                        "url": "https://example.com/form"
                    }
                },
                {
                    "name": "Bare",
                    "request": "https://example.com/ping"
                }
            ]
        });
        let col = postman_to_collection(&doc).unwrap();
        assert_eq!(col.requests.len(), 2);
        assert_eq!(col.requests[0].body_type, "x-www-form-urlencoded");
        assert_eq!(col.requests[1].method, "GET");
        assert_eq!(col.requests[1].url, "https://example.com/ping");
    }
}
