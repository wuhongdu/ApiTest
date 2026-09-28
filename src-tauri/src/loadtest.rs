//! Built-in concurrent load testing (JMeter-like).

use crate::http::send_http;
use crate::models::{SendRequestInput, SendRequestResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestInput {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub params: Vec<crate::models::KeyValue>,
    #[serde(default)]
    pub headers: Vec<crate::models::KeyValue>,
    #[serde(default = "default_body_type")]
    pub body_type: String,
    #[serde(default)]
    pub body_content: String,
    #[serde(default)]
    pub body_language: String,
    /// Concurrent virtual users
    #[serde(default = "default_threads")]
    pub threads: u32,
    /// Iterations per thread
    #[serde(default = "default_loops")]
    pub loops: u32,
    /// Seconds to ramp all threads up
    #[serde(default)]
    pub ramp_up_secs: f64,
    /// Env vars already resolved by caller (or empty)
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}

fn default_body_type() -> String {
    "none".into()
}
fn default_threads() -> u32 {
    10
}
fn default_loops() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCount {
    pub status: u16,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResult {
    pub total: u64,
    pub success: u64,
    pub failed: u64,
    pub duration_ms: u128,
    pub min_ms: u128,
    pub max_ms: u128,
    pub avg_ms: f64,
    pub p95_ms: u128,
    pub throughput_rps: f64,
    pub status_counts: Vec<StatusCount>,
    pub error_samples: Vec<String>,
}

#[derive(Default)]
struct Acc {
    durations: Vec<u128>,
    success: u64,
    failed: u64,
    status: HashMap<u16, u64>,
    errors: Vec<String>,
}

fn is_success(r: &SendRequestResult) -> bool {
    r.error.is_none() && r.status >= 200 && r.status < 400
}

/// Run a synchronous multi-thread load test. Caps total samples for safety.
pub fn run_load_test(input: LoadTestInput) -> Result<LoadTestResult, String> {
    let threads = input.threads.clamp(1, 100);
    let loops = input.loops.clamp(1, 1000);
    let total_planned = threads as u64 * loops as u64;
    if total_planned > 10_000 {
        return Err(format!(
            "请求总量过大（{total_planned}），请将 threads×loops 控制在 10000 以内"
        ));
    }
    if input.url.trim().is_empty() {
        return Err("URL 不能为空".into());
    }

    let sample = SendRequestInput {
        method: input.method.clone(),
        url: input.url.clone(),
        params: input.params.clone(),
        headers: input.headers.clone(),
        body_type: input.body_type.clone(),
        body_content: input.body_content.clone(),
        body_language: input.body_language.clone(),
        request_id: None,
        environment_id: None,
    };
    let vars = Arc::new(input.env_vars.clone());
    let acc = Arc::new(Mutex::new(Acc::default()));
    let ramp = input.ramp_up_secs.max(0.0);

    let wall = Instant::now();
    let mut handles = Vec::with_capacity(threads as usize);

    for t in 0..threads {
        let sample = sample.clone();
        let vars = Arc::clone(&vars);
        let acc = Arc::clone(&acc);
        let delay_ms = if threads <= 1 || ramp <= 0.0 {
            0u64
        } else {
            ((ramp * 1000.0) * (t as f64) / (threads as f64 - 1.0)).round() as u64
        };

        handles.push(thread::spawn(move || {
            if delay_ms > 0 {
                thread::sleep(Duration::from_millis(delay_ms));
            }
            for _ in 0..loops {
                let result = send_http(&sample, &vars);
                let ok = is_success(&result);
                let mut g = acc.lock().unwrap();
                g.durations.push(result.duration_ms);
                if ok {
                    g.success += 1;
                } else {
                    g.failed += 1;
                    if g.errors.len() < 20 {
                        let msg = result
                            .error
                            .clone()
                            .unwrap_or_else(|| format!("HTTP {}", result.status));
                        g.errors.push(msg);
                    }
                }
                *g.status.entry(result.status).or_insert(0) += 1;
            }
        }));
    }

    for h in handles {
        h.join().map_err(|_| "压测线程异常退出".to_string())?;
    }

    let wall_ms = wall.elapsed().as_millis();
    let g = acc.lock().unwrap();
    let total = g.success + g.failed;
    let mut durations = g.durations.clone();
    durations.sort_unstable();

    let (min_ms, max_ms, avg_ms, p95_ms) = if durations.is_empty() {
        (0, 0, 0.0, 0)
    } else {
        let min = *durations.first().unwrap();
        let max = *durations.last().unwrap();
        let sum: u128 = durations.iter().sum();
        let avg = sum as f64 / durations.len() as f64;
        let idx = ((durations.len() as f64) * 0.95).ceil() as usize;
        let p95 = durations[(idx.max(1) - 1).min(durations.len() - 1)];
        (min, max, avg, p95)
    };

    let throughput = if wall_ms == 0 {
        total as f64
    } else {
        (total as f64) / (wall_ms as f64 / 1000.0)
    };

    let mut status_counts: Vec<StatusCount> = g
        .status
        .iter()
        .map(|(&status, &count)| StatusCount { status, count })
        .collect();
    status_counts.sort_by_key(|s| s.status);

    Ok(LoadTestResult {
        total,
        success: g.success,
        failed: g.failed,
        duration_ms: wall_ms,
        min_ms,
        max_ms,
        avg_ms,
        p95_ms,
        throughput_rps: throughput,
        status_counts,
        error_samples: g.errors.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_plan() {
        let err = run_load_test(LoadTestInput {
            method: "GET".into(),
            url: "https://example.com".into(),
            params: vec![],
            headers: vec![],
            body_type: "none".into(),
            body_content: String::new(),
            body_language: String::new(),
            threads: 100,
            loops: 200,
            ramp_up_secs: 0.0,
            env_vars: HashMap::new(),
        })
        .unwrap_err();
        assert!(err.contains("10000"));
    }
}
