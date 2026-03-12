use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    pub source: String,
    #[serde(default = "default_image_name")]
    pub image_name: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub pkgs: Vec<String>,
    pub build_cmd: Option<String>,
    pub start_cmd: Option<String>,
}

fn default_image_name() -> String {
    "railway-app:latest".to_string()
}

fn default_port() -> u16 {
    3000
}

#[derive(Debug, Deserialize)]
pub struct StopRequest {
    #[serde(default)]
    pub remove: bool,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct DeployResponse {
    pub container_id: String,
    pub image: String,
    pub host_port: u16,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct StopResponse {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub container_id: String,
    pub logs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
