//! Convert Swagger 2.0 / OpenAPI 3.x documents into ApiTest collections.

use crate::models::{ExportCollection, ExportRequest, KeyValue};
use serde_json::Value;

fn kv(key: &str, value: &str) -> KeyValue {
    KeyValue::text(key, value)
}

fn str_field<'a>(obj: &'a Value, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(|v| v.as_str())
}

fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    if base.is_empty() {
        format!("{{{{baseUrl}}}}{path}")
    } else if base.contains("{{") {
        format!("{base}{path}")
    } else {
        format!("{base}{path}")
    }
}

fn resolve_oas3_server(doc: &Value) -> String {
    let servers = doc.get("servers").and_then(|s| s.as_array());
    let Some(list) = servers else {
        return "{{baseUrl}}".into();
    };
    let Some(first) = list.first() else {
        return "{{baseUrl}}".into();
    };
    let mut url = str_field(first, "url").unwrap_or("{{baseUrl}}").to_string();
    if let Some(vars) = first.get("variables").and_then(|v| v.as_object()) {
        for (name, spec) in vars {
            let default = spec
                .get("default")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            url = url.replace(&format!("{{{name}}}"), default);
        }
    }
    if url.is_empty() {
        "{{baseUrl}}".into()
    } else {
        url
    }
}

fn resolve_swagger2_base(doc: &Value) -> String {
    let host = str_field(doc, "host").unwrap_or("");
    if host.is_empty() {
        return "{{baseUrl}}".into();
    }
    let scheme = doc
        .get("schemes")
        .and_then(|s| s.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("https");
    let base_path = str_field(doc, "basePath").unwrap_or("");
    let base_path = if base_path == "/" { "" } else { base_path };
    format!("{scheme}://{host}{base_path}")
}

fn example_from_schema(schema: &Value) -> String {
    if let Some(ex) = schema.get("example") {
        return serde_json::to_string_pretty(ex).unwrap_or_else(|_| "{}".into());
    }
    if let Some(ex) = schema
        .get("examples")
        .and_then(|e| e.as_object())
        .and_then(|m| m.values().next())
    {
        let val = ex.get("value").unwrap_or(ex);
        return serde_json::to_string_pretty(val).unwrap_or_else(|_| "{}".into());
    }
    match schema.get("type").and_then(|t| t.as_str()).unwrap_or("object") {
        "object" => {
            let mut map = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                for (k, v) in props.iter().take(12) {
                    map.insert(k.clone(), placeholder_value(v));
                }
            }
            serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_else(|_| "{}".into())
        }
        "array" => {
            let item = schema.get("items").unwrap_or(&Value::Null);
            serde_json::to_string_pretty(&Value::Array(vec![placeholder_value(item)]))
                .unwrap_or_else(|_| "[]".into())
        }
        "string" => "\"string\"".into(),
        "integer" | "number" => "0".into(),
        "boolean" => "false".into(),
        _ => "{}".into(),
    }
}

fn placeholder_value(schema: &Value) -> Value {
    if let Some(ex) = schema.get("example") {
        return ex.clone();
    }
    match schema.get("type").and_then(|t| t.as_str()).unwrap_or("string") {
        "object" => {
            let mut map = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                for (k, v) in props.iter().take(8) {
                    map.insert(k.clone(), placeholder_value(v));
                }
            }
            Value::Object(map)
        }
        "array" => Value::Array(vec![placeholder_value(
            schema.get("items").unwrap_or(&Value::Null),
        )]),
        "integer" | "number" => Value::from(0),
        "boolean" => Value::Bool(false),
        _ => Value::String(
            schema
                .get("default")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string(),
        ),
    }
}

fn apply_parameters(
    params_node: Option<&Value>,
    query: &mut Vec<KeyValue>,
    headers: &mut Vec<KeyValue>,
) {
    let Some(arr) = params_node.and_then(|p| p.as_array()) else {
        return;
    };
    for p in arr {
        let name = str_field(p, "name").unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let location = str_field(p, "in").unwrap_or("query");
        let schema = p.get("schema").unwrap_or(p);
        let value = p
            .get("example")
            .or_else(|| schema.get("example"))
            .or_else(|| schema.get("default"))
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string().trim_matches('"').to_string(),
            })
            .unwrap_or_default();

        match location {
            "query" => query.push(kv(&name, &value)),
            "header" => headers.push(kv(&name, &value)),
            // path params stay in URL template {id}
            _ => {}
        }
    }
}

fn extract_body_oas3(op: &Value) -> (String, String, Vec<KeyValue>) {
    let mut headers = Vec::new();
    let Some(body) = op.get("requestBody") else {
        return ("none".into(), String::new(), headers);
    };
    let content = body.get("content").and_then(|c| c.as_object());
    let Some(content) = content else {
        return ("none".into(), String::new(), headers);
    };

    // prefer json
    let preferred = [
        "application/json",
        "application/json; charset=utf-8",
        "text/json",
        "application/x-www-form-urlencoded",
        "multipart/form-data",
        "text/plain",
    ];
    let mut selected_ct: Option<String> = None;
    let mut selected_media: Option<&Value> = None;
    for key in preferred {
        if let Some(v) = content.get(key) {
            selected_ct = Some(key.to_string());
            selected_media = Some(v);
            break;
        }
    }
    if selected_media.is_none() {
        if let Some((k, v)) = content.iter().next() {
            selected_ct = Some(k.clone());
            selected_media = Some(v);
        }
    }
    let Some(ct) = selected_ct else {
        return ("none".into(), String::new(), headers);
    };
    let media = selected_media.unwrap();
    headers.push(kv("Content-Type", &ct));

    if ct.contains("json") {
        let schema = media.get("schema").unwrap_or(&Value::Null);
        let body_content = if let Some(ex) = media.get("example") {
            serde_json::to_string_pretty(ex).unwrap_or_else(|_| "{}".into())
        } else if let Some(ex) = media
            .get("examples")
            .and_then(|e| e.as_object())
            .and_then(|m| m.values().next())
            .map(|e| e.get("value").unwrap_or(e))
        {
            serde_json::to_string_pretty(ex).unwrap_or_else(|_| "{}".into())
        } else {
            example_from_schema(schema)
        };
        return ("json".into(), body_content, headers);
    }
    if ct.contains("urlencoded") {
        let schema = media.get("schema").unwrap_or(&Value::Null);
        let mut rows = Vec::new();
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (k, v) in props {
                let val = v
                    .get("example")
                    .or_else(|| v.get("default"))
                    .map(|x| match x {
                        Value::String(s) => s.clone(),
                        other => other.to_string().trim_matches('"').to_string(),
                    })
                    .unwrap_or_default();
                rows.push(kv(k, &val));
            }
        }
        let body_content = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into());
        return ("x-www-form-urlencoded".into(), body_content, headers);
    }
    if ct.contains("multipart") {
        return ("form-data".into(), "[]".into(), headers);
    }
    ("raw".into(), String::new(), headers)
}

fn extract_body_swagger2(op: &Value) -> (String, String, Vec<KeyValue>) {
    let mut headers = Vec::new();
    let Some(params) = op.get("parameters").and_then(|p| p.as_array()) else {
        return ("none".into(), String::new(), headers);
    };

    let mut form_rows = Vec::new();
    for p in params {
        let location = str_field(p, "in").unwrap_or("");
        let name = str_field(p, "name").unwrap_or("");
        match location {
            "body" => {
                let schema = p.get("schema").unwrap_or(&Value::Null);
                headers.push(kv("Content-Type", "application/json"));
                return ("json".into(), example_from_schema(schema), headers);
            }
            "formData" => {
                let val = p
                    .get("default")
                    .or_else(|| p.get("example"))
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string().trim_matches('"').to_string(),
                    })
                    .unwrap_or_default();
                if !name.is_empty() {
                    form_rows.push(kv(name, &val));
                }
            }
            _ => {}
        }
    }
    if !form_rows.is_empty() {
        let consumes = op
            .get("consumes")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("application/x-www-form-urlencoded");
        headers.push(kv("Content-Type", consumes));
        let body_type = if consumes.contains("multipart") {
            "form-data"
        } else {
            "x-www-form-urlencoded"
        };
        let body_content = serde_json::to_string(&form_rows).unwrap_or_else(|_| "[]".into());
        return (body_type.into(), body_content, headers);
    }
    ("none".into(), String::new(), headers)
}

fn operation_name(method: &str, path: &str, op: &Value) -> String {
    if let Some(id) = str_field(op, "operationId") {
        if !id.is_empty() {
            return id.to_string();
        }
    }
    if let Some(summary) = str_field(op, "summary") {
        if !summary.is_empty() {
            return summary.to_string();
        }
    }
    format!("{} {}", method.to_uppercase(), path)
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
        pre_script: String::new(),
        test_script: String::new(),
        mock_enabled: false,
        mock_status: 200,
        mock_headers: vec![],
        mock_body: String::new(),
        mock_delay_ms: 0,
    }
}

const HTTP_METHODS: &[&str] = &[
    "get", "post", "put", "patch", "delete", "head", "options", "trace",
];

/// Detect Swagger 2 / OpenAPI 3 document from JSON (or YAML text).
pub fn looks_like_openapi(root: &Value) -> bool {
    root.get("openapi").and_then(|v| v.as_str()).is_some()
        || root.get("swagger").and_then(|v| v.as_str()).is_some()
}

pub fn parse_import_json(raw: &str) -> Result<Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("内容为空".into());
    }
    // JSON first
    if trimmed.starts_with('{') {
        return serde_json::from_str(trimmed).map_err(|e| format!("JSON 解析失败: {e}"));
    }
    // YAML OpenAPI often starts with openapi: / swagger:
    if trimmed.starts_with("openapi:")
        || trimmed.starts_with("swagger:")
        || trimmed.contains('\n')
    {
        return serde_yaml::from_str(trimmed).map_err(|e| format!("YAML 解析失败: {e}"));
    }
    serde_json::from_str(trimmed).map_err(|e| format!("JSON 解析失败: {e}"))
}

pub fn openapi_to_collection(doc: &Value) -> Result<ExportCollection, String> {
    if !looks_like_openapi(doc) {
        return Err("不是有效的 Swagger / OpenAPI 文档".into());
    }

    let is_oas3 = doc.get("openapi").is_some();
    let name = doc
        .get("info")
        .and_then(|i| str_field(i, "title"))
        .unwrap_or(if is_oas3 { "OpenAPI Import" } else { "Swagger Import" })
        .to_string();

    let base = if is_oas3 {
        resolve_oas3_server(doc)
    } else {
        resolve_swagger2_base(doc)
    };

    let paths = doc
        .get("paths")
        .and_then(|p| p.as_object())
        .ok_or_else(|| "文档缺少 paths".to_string())?;

    let mut requests = Vec::new();
    for (path, item) in paths {
        let Some(item_obj) = item.as_object() else {
            continue;
        };
        // path-level parameters
        let path_params = item.get("parameters");

        for method in HTTP_METHODS {
            let Some(op) = item_obj.get(*method) else {
                continue;
            };
            if !op.is_object() {
                continue;
            }

            let mut query = Vec::new();
            let mut headers = Vec::new();
            apply_parameters(path_params, &mut query, &mut headers);
            apply_parameters(op.get("parameters"), &mut query, &mut headers);

            let (body_type, body_content, mut body_headers) = if is_oas3 {
                extract_body_oas3(op)
            } else {
                extract_body_swagger2(op)
            };
            headers.append(&mut body_headers);

            // dedupe header keys (keep first)
            let mut seen = std::collections::HashSet::new();
            headers.retain(|h| seen.insert(h.key.to_ascii_lowercase()));

            let mut req = empty_request(
                operation_name(method, path, op),
                method.to_uppercase(),
                join_url(&base, path),
            );
            req.params = query;
            req.headers = headers;
            req.body_type = body_type;
            req.body_content = body_content;
            requests.push(req);
        }
    }

    if requests.is_empty() {
        return Err("未从文档中解析到任何接口".into());
    }

    Ok(ExportCollection {
        format: "apitest-collection".into(),
        version: 1,
        name,
        base_url: String::new(),
        requests,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_openapi3_json() {
        let raw = r#"{
          "openapi": "3.0.0",
          "info": { "title": "Demo API", "version": "1.0.0" },
          "servers": [{ "url": "https://api.example.com/v1" }],
          "paths": {
            "/users": {
              "get": {
                "operationId": "listUsers",
                "parameters": [
                  { "name": "page", "in": "query", "schema": { "type": "integer", "example": 1 } }
                ]
              },
              "post": {
                "summary": "Create user",
                "requestBody": {
                  "content": {
                    "application/json": {
                      "schema": {
                        "type": "object",
                        "properties": {
                          "name": { "type": "string", "example": "Ada" }
                        }
                      }
                    }
                  }
                }
              }
            },
            "/users/{id}": {
              "get": { "operationId": "getUser" }
            }
          }
        }"#;
        let doc = parse_import_json(raw).unwrap();
        let col = openapi_to_collection(&doc).unwrap();
        assert_eq!(col.name, "Demo API");
        assert_eq!(col.requests.len(), 3);
        let list = col.requests.iter().find(|r| r.name == "listUsers").unwrap();
        assert_eq!(list.method, "GET");
        assert_eq!(list.url, "https://api.example.com/v1/users");
        assert!(list.params.iter().any(|p| p.key == "page"));
        let create = col
            .requests
            .iter()
            .find(|r| r.name == "Create user")
            .unwrap();
        assert_eq!(create.method, "POST");
        assert_eq!(create.body_type, "json");
        assert!(create.body_content.contains("Ada") || create.body_content.contains("name"));
    }

    #[test]
    fn import_swagger2_json() {
        let raw = r#"{
          "swagger": "2.0",
          "info": { "title": "Pet Store", "version": "1.0.0" },
          "host": "petstore.swagger.io",
          "basePath": "/v2",
          "schemes": ["https"],
          "paths": {
            "/pet/findByStatus": {
              "get": {
                "operationId": "findPetsByStatus",
                "parameters": [
                  { "name": "status", "in": "query", "type": "string", "default": "available" }
                ]
              }
            }
          }
        }"#;
        let doc = parse_import_json(raw).unwrap();
        let col = openapi_to_collection(&doc).unwrap();
        assert_eq!(col.name, "Pet Store");
        assert_eq!(col.requests.len(), 1);
        assert_eq!(
            col.requests[0].url,
            "https://petstore.swagger.io/v2/pet/findByStatus"
        );
    }
}
