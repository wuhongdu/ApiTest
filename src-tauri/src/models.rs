use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// `text` (default) or `file` — used by multipart form-data
    #[serde(default = "default_field_type", rename = "type")]
    pub field_type: String,
    /// Absolute path when field_type == "file"
    #[serde(default)]
    pub file_path: String,
    /// Original file name for Content-Disposition
    #[serde(default)]
    pub file_name: String,
}

fn default_true() -> bool {
    true
}

fn default_field_type() -> String {
    "text".into()
}

impl KeyValue {
    pub fn text(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
            field_type: "text".into(),
            file_path: String::new(),
            file_name: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub sort_order: i64,
    #[serde(default = "default_workspace")]
    pub workspace_id: i64,
    /// Global URL prefix for requests in this collection (supports {{var}})
    #[serde(default)]
    pub base_url: String,
}

fn default_workspace() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestItem {
    pub id: i64,
    pub collection_id: Option<i64>,
    pub name: String,
    pub method: String,
    pub url: String,
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub body_type: String,
    pub body_content: String,
    pub sort_order: i64,
    #[serde(default)]
    pub pre_script: String,
    #[serde(default)]
    pub test_script: String,
    #[serde(default)]
    pub mock_enabled: bool,
    #[serde(default = "default_mock_status")]
    pub mock_status: i64,
    #[serde(default)]
    pub mock_headers: Vec<KeyValue>,
    #[serde(default)]
    pub mock_body: String,
    #[serde(default)]
    pub mock_delay_ms: i64,
}

fn default_mock_status() -> i64 {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: i64,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub id: i64,
    pub environment_id: i64,
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionInput {
    pub name: String,
    pub parent_id: Option<i64>,
    pub workspace_id: Option<i64>,
    #[serde(default)]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCollectionInput {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub base_url: String,
}

/// Kept for compatibility; prefer UpdateCollectionInput.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameCollectionInput {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRequestInput {
    pub collection_id: Option<i64>,
    pub name: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub url: String,
}

fn default_method() -> String {
    "GET".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveRequestInput {
    pub id: i64,
    pub name: String,
    pub method: String,
    pub url: String,
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub body_type: String,
    pub body_content: String,
    pub collection_id: Option<i64>,
    #[serde(default)]
    pub pre_script: String,
    #[serde(default)]
    pub test_script: String,
    #[serde(default)]
    pub mock_enabled: bool,
    #[serde(default = "default_mock_status")]
    pub mock_status: i64,
    #[serde(default)]
    pub mock_headers: Vec<KeyValue>,
    #[serde(default)]
    pub mock_body: String,
    #[serde(default)]
    pub mock_delay_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertEnvVarInput {
    pub environment_id: i64,
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRequestInput {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub params: Vec<KeyValue>,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default = "default_body_type")]
    pub body_type: String,
    #[serde(default)]
    pub body_content: String,
    pub request_id: Option<i64>,
    pub environment_id: Option<i64>,
}

fn default_body_type() -> String {
    "none".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRequestResult {
    pub status: u16,
    pub status_text: String,
    pub duration_ms: u128,
    pub headers: Vec<KeyValue>,
    pub body: String,
    pub url: String,
    pub error: Option<String>,
    #[serde(default)]
    pub mocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub collection_id: Option<i64>,
    pub request_id: Option<i64>,
    pub method: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: i64,
    pub request_id: Option<i64>,
    pub method: String,
    pub url: String,
    pub status_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryDetail {
    pub id: i64,
    pub request_id: Option<i64>,
    pub method: String,
    pub url: String,
    pub status_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub request_snapshot: Option<String>,
    pub response_snapshot: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub body_type: String,
    pub body_content: String,
    #[serde(default)]
    pub pre_script: String,
    #[serde(default)]
    pub test_script: String,
    #[serde(default)]
    pub mock_enabled: bool,
    #[serde(default = "default_mock_status")]
    pub mock_status: i64,
    #[serde(default)]
    pub mock_headers: Vec<KeyValue>,
    #[serde(default)]
    pub mock_body: String,
    #[serde(default)]
    pub mock_delay_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCollection {
    #[serde(default)]
    pub format: String,
    #[serde(default = "default_export_version")]
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub requests: Vec<ExportRequest>,
}

fn default_export_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportCollectionInput {
    pub json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceInput {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetThemeInput {
    pub theme: String,
}
