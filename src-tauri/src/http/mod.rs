use crate::models::{KeyValue, SendRequestInput, SendRequestResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

pub fn substitute(input: &str, vars: &HashMap<String, String>) -> String {
    let mut out = input.to_string();
    for (k, v) in vars {
        let pattern = format!("{{{{{}}}}}", k);
        out = out.replace(&pattern, v);
    }
    out
}

/// Join collection base URL with request URL.
/// Absolute http(s) URLs and URLs starting with `{{var}}` ignore the prefix
/// (env placeholders usually expand to a full host).
pub fn join_base_url(base: &str, url: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    let url = url.trim();
    if url.is_empty() {
        return base.to_string();
    }
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || url.starts_with("{{") {
        return url.to_string();
    }
    if base.is_empty() {
        return url.to_string();
    }
    if url.starts_with('/') {
        format!("{base}{url}")
    } else {
        format!("{base}/{url}")
    }
}

#[cfg(test)]
mod join_tests {
    use super::join_base_url;

    #[test]
    fn joins_relative_paths() {
        assert_eq!(
            join_base_url("https://api.example.com/v1", "/users"),
            "https://api.example.com/v1/users"
        );
        assert_eq!(
            join_base_url("https://api.example.com/v1", "users"),
            "https://api.example.com/v1/users"
        );
    }

    #[test]
    fn keeps_absolute_url() {
        assert_eq!(
            join_base_url("https://api.example.com", "https://other.com/x"),
            "https://other.com/x"
        );
    }

    #[test]
    fn supports_env_placeholder_base() {
        assert_eq!(
            join_base_url("{{baseUrl}}/api", "/pets"),
            "{{baseUrl}}/api/pets"
        );
    }

    #[test]
    fn ignores_prefix_when_url_starts_with_env_var() {
        assert_eq!(
            join_base_url("http://localhost:8080", "{{baseUrl}}/"),
            "{{baseUrl}}/"
        );
        assert_eq!(
            join_base_url("http://localhost:8080", "{{baseUrl}}/login"),
            "{{baseUrl}}/login"
        );
    }
}

fn parse_form_rows(content: &str) -> Vec<KeyValue> {
    serde_json::from_str(content).unwrap_or_default()
}

fn encode_urlencoded(rows: &[KeyValue], vars: &HashMap<String, String>) -> String {
    let mut pairs = Vec::new();
    for row in rows {
        if !row.enabled || row.key.is_empty() {
            continue;
        }
        let k = substitute(&row.key, vars);
        let v = substitute(&row.value, vars);
        pairs.push(format!(
            "{}={}",
            urlencoding_encode(&k),
            urlencoding_encode(&v)
        ));
    }
    pairs.join("&")
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn client_error(url: String, msg: impl Into<String>) -> SendRequestResult {
    SendRequestResult {
        status: 0,
        status_text: "CLIENT_ERROR".into(),
        duration_ms: 0,
        headers: vec![],
        body: String::new(),
        url,
        error: Some(msg.into()),
        mocked: false,
    }
}

pub fn cancelled_result(url: &str, duration_ms: u128) -> SendRequestResult {
    SendRequestResult {
        status: 0,
        status_text: "CANCELLED".into(),
        duration_ms,
        headers: vec![],
        body: String::new(),
        url: url.to_string(),
        error: Some("请求已取消".into()),
        mocked: false,
    }
}

enum PreparedBody {
    None,
    Text {
        content: String,
        content_type: Option<&'static str>,
    },
    Bytes {
        data: Vec<u8>,
        content_type: Option<&'static str>,
    },
    Multipart(Vec<PreparedPart>),
}

enum PreparedPart {
    Text {
        key: String,
        value: String,
    },
    File {
        key: String,
        path: PathBuf,
        file_name: String,
    },
}

struct PreparedRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: PreparedBody,
}

fn raw_content_type(language: &str) -> Option<&'static str> {
    match language.trim().to_ascii_lowercase().as_str() {
        "json" => Some("application/json"),
        "javascript" | "js" => Some("application/javascript"),
        "html" => Some("text/html"),
        "xml" => Some("application/xml"),
        "text" | "plain" => Some("text/plain"),
        _ => None,
    }
}

fn parse_binary_meta(content: &str) -> Result<(PathBuf, String), String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("未选择 binary 文件".into());
    }
    if trimmed.starts_with('{') {
        let v: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| format!("binary 元数据无效: {e}"))?;
        let path = v
            .get("file_path")
            .or_else(|| v.get("path"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if path.trim().is_empty() {
            return Err("未选择 binary 文件".into());
        }
        let path_buf = PathBuf::from(&path);
        let file_name = v
            .get("file_name")
            .or_else(|| v.get("name"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                path_buf
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file")
                    .to_string()
            });
        return Ok((path_buf, file_name));
    }
    let path_buf = PathBuf::from(trimmed);
    let file_name = path_buf
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string();
    Ok((path_buf, file_name))
}

fn build_graphql_payload(content: &str) -> Result<String, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(r#"{"query":"","variables":{}}"#.into());
    }
    let v: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| format!("GraphQL body 不是合法 JSON: {e}"))?;
    let query = v
        .get("query")
        .and_then(|q| q.as_str())
        .unwrap_or("")
        .to_string();
    let variables = match v.get("variables") {
        Some(serde_json::Value::String(s)) => {
            serde_json::from_str::<serde_json::Value>(s).unwrap_or(serde_json::json!({}))
        }
        Some(other) => other.clone(),
        None => serde_json::json!({}),
    };
    serde_json::to_string(&serde_json::json!({
        "query": query,
        "variables": variables,
    }))
    .map_err(|e| e.to_string())
}

fn prepare_request(
    input: &SendRequestInput,
    vars: &HashMap<String, String>,
) -> Result<PreparedRequest, SendRequestResult> {
    let method = substitute(&input.method, vars).to_uppercase();
    let mut url = substitute(&input.url, vars);

    let params: Vec<(String, String)> = input
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.is_empty())
        .map(|p| (substitute(&p.key, vars), substitute(&p.value, vars)))
        .collect();

    if !params.is_empty() {
        if let Ok(mut parsed) = url::Url::parse(&url) {
            {
                let mut qp = parsed.query_pairs_mut();
                for (k, v) in &params {
                    qp.append_pair(k, v);
                }
            }
            url = parsed.to_string();
        }
    }

    let headers: Vec<(String, String)> = input
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.is_empty())
        .map(|h| (substitute(&h.key, vars), substitute(&h.value, vars)))
        .collect();

    let mut body_type = substitute(&input.body_type, vars);
    let body_content = substitute(&input.body_content, vars);
    let mut body_language = substitute(&input.body_language, vars);
    if body_type == "json" {
        body_type = "raw".into();
        if body_language.trim().is_empty() {
            body_language = "json".into();
        }
    }

    let body = if body_type == "none" || matches!(method.as_str(), "GET" | "HEAD") {
        PreparedBody::None
    } else {
        match body_type.as_str() {
            "raw" => PreparedBody::Text {
                content: body_content,
                content_type: raw_content_type(&body_language),
            },
            "x-www-form-urlencoded" => {
                let rows = parse_form_rows(&body_content);
                let encoded = if rows.is_empty() && !body_content.trim().is_empty() {
                    body_content
                } else {
                    encode_urlencoded(&rows, vars)
                };
                PreparedBody::Text {
                    content: encoded,
                    content_type: Some("application/x-www-form-urlencoded"),
                }
            }
            "form-data" => {
                let rows = parse_form_rows(&input.body_content);
                let mut parts = Vec::new();
                for row in rows {
                    if !row.enabled || row.key.is_empty() {
                        continue;
                    }
                    let k = substitute(&row.key, vars);
                    if row.field_type == "file" {
                        let path = substitute(&row.file_path, vars);
                        if path.trim().is_empty() {
                            return Err(client_error(
                                url,
                                format!("form-data 字段「{k}」未选择文件"),
                            ));
                        }
                        let path_buf = PathBuf::from(&path);
                        if !path_buf.is_file() {
                            return Err(client_error(url, format!("文件不存在: {path}")));
                        }
                        let file_name = if row.file_name.trim().is_empty() {
                            path_buf
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("file")
                                .to_string()
                        } else {
                            substitute(&row.file_name, vars)
                        };
                        parts.push(PreparedPart::File {
                            key: k,
                            path: path_buf,
                            file_name,
                        });
                    } else {
                        parts.push(PreparedPart::Text {
                            key: k,
                            value: substitute(&row.value, vars),
                        });
                    }
                }
                PreparedBody::Multipart(parts)
            }
            "binary" => {
                let (path_buf, _name) = match parse_binary_meta(&body_content) {
                    Ok(v) => v,
                    Err(e) => return Err(client_error(url, e)),
                };
                if !path_buf.is_file() {
                    return Err(client_error(
                        url,
                        format!("文件不存在: {}", path_buf.display()),
                    ));
                }
                match std::fs::read(&path_buf) {
                    Ok(data) => PreparedBody::Bytes {
                        data,
                        content_type: Some("application/octet-stream"),
                    },
                    Err(e) => {
                        return Err(client_error(
                            url,
                            format!("读取文件失败 ({}): {e}", path_buf.display()),
                        ));
                    }
                }
            }
            "graphql" => match build_graphql_payload(&body_content) {
                Ok(payload) => PreparedBody::Text {
                    content: payload,
                    content_type: Some("application/json"),
                },
                Err(e) => return Err(client_error(url, e)),
            },
            _ => PreparedBody::Text {
                content: body_content,
                content_type: None,
            },
        }
    };

    Ok(PreparedRequest {
        method,
        url,
        headers,
        body,
    })
}

fn has_content_type(headers: &[(String, String)]) -> bool {
    headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
}

fn map_response(started: Instant, resp: reqwest::blocking::Response) -> SendRequestResult {
    let duration_ms = started.elapsed().as_millis();
    let status = resp.status().as_u16();
    let status_text = resp
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();
    let final_url = resp.url().to_string();
    let resp_headers: Vec<KeyValue> = resp
        .headers()
        .iter()
        .map(|(k, v)| KeyValue::text(k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body = resp.text().unwrap_or_default();
    SendRequestResult {
        status,
        status_text,
        duration_ms,
        headers: resp_headers,
        body,
        url: final_url,
        error: None,
        mocked: false,
    }
}

fn map_async_response(
    started: Instant,
    resp: reqwest::Response,
) -> impl std::future::Future<Output = SendRequestResult> {
    async move {
        let duration_ms = started.elapsed().as_millis();
        let status = resp.status().as_u16();
        let status_text = resp
            .status()
            .canonical_reason()
            .unwrap_or("")
            .to_string();
        let final_url = resp.url().to_string();
        let resp_headers: Vec<KeyValue> = resp
            .headers()
            .iter()
            .map(|(k, v)| KeyValue::text(k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.text().await.unwrap_or_default();
        SendRequestResult {
            status,
            status_text,
            duration_ms,
            headers: resp_headers,
            body,
            url: final_url,
            error: None,
            mocked: false,
        }
    }
}

fn network_error(url: String, started: Instant, e: impl ToString) -> SendRequestResult {
    SendRequestResult {
        status: 0,
        status_text: "NETWORK_ERROR".into(),
        duration_ms: started.elapsed().as_millis(),
        headers: vec![],
        body: String::new(),
        url,
        error: Some(e.to_string()),
        mocked: false,
    }
}

/// Blocking HTTP send (used by load test worker threads).
pub fn send_http(input: &SendRequestInput, vars: &HashMap<String, String>) -> SendRequestResult {
    let prepared = match prepare_request(input, vars) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let client = match reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(false)
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => return client_error(prepared.url, e.to_string()),
    };

    let mut builder = match prepared.method.as_str() {
        "GET" => client.get(&prepared.url),
        "POST" => client.post(&prepared.url),
        "PUT" => client.put(&prepared.url),
        "PATCH" => client.patch(&prepared.url),
        "DELETE" => client.delete(&prepared.url),
        "HEAD" => client.head(&prepared.url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &prepared.url),
        other => client.request(
            reqwest::Method::from_bytes(other.as_bytes()).unwrap_or(reqwest::Method::GET),
            &prepared.url,
        ),
    };

    for (k, v) in &prepared.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }

    match &prepared.body {
        PreparedBody::None => {}
        PreparedBody::Text {
            content,
            content_type,
        } => {
            if let Some(ct) = content_type {
                if !has_content_type(&prepared.headers) {
                    builder = builder.header("Content-Type", *ct);
                }
            }
            builder = builder.body(content.clone());
        }
        PreparedBody::Bytes { data, content_type } => {
            if let Some(ct) = content_type {
                if !has_content_type(&prepared.headers) {
                    builder = builder.header("Content-Type", *ct);
                }
            }
            builder = builder.body(data.clone());
        }
        PreparedBody::Multipart(parts) => {
            let mut form = reqwest::blocking::multipart::Form::new();
            for part in parts {
                match part {
                    PreparedPart::Text { key, value } => {
                        form = form.text(key.clone(), value.clone());
                    }
                    PreparedPart::File {
                        key,
                        path,
                        file_name,
                    } => match reqwest::blocking::multipart::Part::file(path) {
                        Ok(p) => {
                            form = form.part(key.clone(), p.file_name(file_name.clone()));
                        }
                        Err(e) => {
                            return client_error(
                                prepared.url,
                                format!("读取文件失败 ({}): {e}", path.display()),
                            );
                        }
                    },
                }
            }
            builder = builder.multipart(form);
        }
    }

    let started = Instant::now();
    let url = prepared.url.clone();
    match builder.send() {
        Ok(resp) => map_response(started, resp),
        Err(e) => network_error(url, started, e),
    }
}

/// Async HTTP send — dropping this future aborts the in-flight request.
pub async fn send_http_async(
    input: &SendRequestInput,
    vars: &HashMap<String, String>,
) -> SendRequestResult {
    let prepared = match prepare_request(input, vars) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let client = match reqwest::Client::builder()
        .danger_accept_invalid_certs(false)
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => return client_error(prepared.url, e.to_string()),
    };

    let mut builder = match prepared.method.as_str() {
        "GET" => client.get(&prepared.url),
        "POST" => client.post(&prepared.url),
        "PUT" => client.put(&prepared.url),
        "PATCH" => client.patch(&prepared.url),
        "DELETE" => client.delete(&prepared.url),
        "HEAD" => client.head(&prepared.url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &prepared.url),
        other => client.request(
            reqwest::Method::from_bytes(other.as_bytes()).unwrap_or(reqwest::Method::GET),
            &prepared.url,
        ),
    };

    for (k, v) in &prepared.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }

    match prepared.body {
        PreparedBody::None => {}
        PreparedBody::Text {
            content,
            content_type,
        } => {
            if let Some(ct) = content_type {
                if !has_content_type(&prepared.headers) {
                    builder = builder.header("Content-Type", ct);
                }
            }
            builder = builder.body(content);
        }
        PreparedBody::Bytes { data, content_type } => {
            if let Some(ct) = content_type {
                if !has_content_type(&prepared.headers) {
                    builder = builder.header("Content-Type", ct);
                }
            }
            builder = builder.body(data);
        }
        PreparedBody::Multipart(parts) => {
            let mut form = reqwest::multipart::Form::new();
            for part in parts {
                match part {
                    PreparedPart::Text { key, value } => {
                        form = form.text(key, value);
                    }
                    PreparedPart::File {
                        key,
                        path,
                        file_name,
                    } => {
                        let bytes = match std::fs::read(&path) {
                            Ok(b) => b,
                            Err(e) => {
                                return client_error(
                                    prepared.url,
                                    format!("读取文件失败 ({}): {e}", path.display()),
                                );
                            }
                        };
                        let p = reqwest::multipart::Part::bytes(bytes).file_name(file_name);
                        form = form.part(key, p);
                    }
                }
            }
            builder = builder.multipart(form);
        }
    }

    let started = Instant::now();
    let url = prepared.url.clone();
    match builder.send().await {
        Ok(resp) => map_async_response(started, resp).await,
        Err(e) => {
            if e.is_timeout() {
                network_error(url, started, e)
            } else {
                // Cancelled futures often surface as request errors
                network_error(url, started, e)
            }
        }
    }
}
