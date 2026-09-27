use crate::models::{KeyValue, SendRequestInput, SendRequestResult};
use std::collections::HashMap;
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

pub fn send_http(
    input: &SendRequestInput,
    vars: &HashMap<String, String>,
) -> SendRequestResult {
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

    let body_type = substitute(&input.body_type, vars);
    let body_content = substitute(&input.body_content, vars);

    let client = match reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(false)
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return SendRequestResult {
                status: 0,
                status_text: "CLIENT_ERROR".into(),
                duration_ms: 0,
                headers: vec![],
                body: String::new(),
                url,
                error: Some(e.to_string()),
                mocked: false,
            };
        }
    };

    let mut builder = match method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "PATCH" => client.patch(&url),
        "DELETE" => client.delete(&url),
        "HEAD" => client.head(&url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url),
        other => client.request(
            reqwest::Method::from_bytes(other.as_bytes()).unwrap_or(reqwest::Method::GET),
            &url,
        ),
    };

    for (k, v) in &headers {
        builder = builder.header(k.as_str(), v.as_str());
    }

    if body_type != "none" && !matches!(method.as_str(), "GET" | "HEAD") {
        match body_type.as_str() {
            "json" => {
                if !headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                {
                    builder = builder.header("Content-Type", "application/json");
                }
                builder = builder.body(body_content);
            }
            "raw" => {
                builder = builder.body(body_content);
            }
            "x-www-form-urlencoded" => {
                let rows = parse_form_rows(&body_content);
                let encoded = if rows.is_empty() && !body_content.trim().is_empty() {
                    body_content
                } else {
                    encode_urlencoded(&rows, vars)
                };
                if !headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                {
                    builder =
                        builder.header("Content-Type", "application/x-www-form-urlencoded");
                }
                builder = builder.body(encoded);
            }
            "form-data" => {
                let rows = parse_form_rows(&input.body_content);
                let mut form = reqwest::blocking::multipart::Form::new();
                for row in rows {
                    if !row.enabled || row.key.is_empty() {
                        continue;
                    }
                    let k = substitute(&row.key, vars);
                    if row.field_type == "file" {
                        let path = substitute(&row.file_path, vars);
                        if path.trim().is_empty() {
                            return SendRequestResult {
                                status: 0,
                                status_text: "CLIENT_ERROR".into(),
                                duration_ms: 0,
                                headers: vec![],
                                body: String::new(),
                                url: url.clone(),
                                error: Some(format!("form-data 字段「{k}」未选择文件")),
                                mocked: false,
                            };
                        }
                        let path_buf = std::path::PathBuf::from(&path);
                        if !path_buf.is_file() {
                            return SendRequestResult {
                                status: 0,
                                status_text: "CLIENT_ERROR".into(),
                                duration_ms: 0,
                                headers: vec![],
                                body: String::new(),
                                url: url.clone(),
                                error: Some(format!("文件不存在: {path}")),
                                mocked: false,
                            };
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
                        match reqwest::blocking::multipart::Part::file(&path_buf) {
                            Ok(part) => {
                                form = form.part(k, part.file_name(file_name));
                            }
                            Err(e) => {
                                return SendRequestResult {
                                    status: 0,
                                    status_text: "CLIENT_ERROR".into(),
                                    duration_ms: 0,
                                    headers: vec![],
                                    body: String::new(),
                                    url: url.clone(),
                                    error: Some(format!("读取文件失败 ({path}): {e}")),
                                    mocked: false,
                                };
                            }
                        }
                    } else {
                        let v = substitute(&row.value, vars);
                        form = form.text(k, v);
                    }
                }
                builder = builder.multipart(form);
            }
            _ => {
                builder = builder.body(body_content);
            }
        }
    }

    let started = Instant::now();
    match builder.send() {
        Ok(resp) => {
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
        Err(e) => SendRequestResult {
            status: 0,
            status_text: "NETWORK_ERROR".into(),
            duration_ms: started.elapsed().as_millis(),
            headers: vec![],
            body: String::new(),
            url,
            error: Some(e.to_string()),
            mocked: false,
        },
    }
}
