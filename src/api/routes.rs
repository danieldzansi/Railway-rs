use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use bollard::query_parameters::{ListContainersOptionsBuilder, LogsOptions};
use bollard::Docker;
use futures_util::StreamExt;

use crate::builder::nixpacks::{self, BuildConfig};
use crate::container::runner::{self, RunConfig};

use super::errors::ApiError;
use super::models::*;

pub type AppState = Arc<Docker>;

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

pub async fn deploy(
    State(docker): State<AppState>,
    Json(req): Json<DeployRequest>,
) -> Result<(StatusCode, Json<DeployResponse>), ApiError> {
    nixpacks::check_installed().await?;

    let build_cfg = BuildConfig {
        source: req.source,
        image_name: req.image_name.clone(),
        env: {
            let mut env = req.env;
            if !env.iter().any(|e| e.starts_with("PORT=")) {
                env.push(format!("PORT={}", req.port));
            }
            env
        },
        pkgs: req.pkgs,
        build_cmd: req.build_cmd,
        start_cmd: req.start_cmd,
    };

    let image = nixpacks::build(&build_cfg).await?;

    let container_port = format!("{}/tcp", req.port);
    let run_cfg = RunConfig {
        image: image.clone(),
        name: format!("railway-{}", &req.image_name.replace(':', "-")),
        container_port: container_port.clone(),
        host_port: req.port,
        env: build_cfg.env.clone(),
    };

    let id = runner::start(&docker, &run_cfg).await?;

    Ok((
        StatusCode::CREATED,
        Json(DeployResponse {
            container_id: id,
            image,
            host_port: req.port,
            message: format!("Container running on port {}", req.port),
        }),
    ))
}

pub async fn list_containers(
    State(docker): State<AppState>,
) -> Result<Json<Vec<ContainerInfo>>, ApiError> {
    use crate::container::runner::{LABEL_MANAGED_BY, LABEL_MANAGED_BY_VALUE};

    let mut filters = std::collections::HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![format!("{LABEL_MANAGED_BY}={LABEL_MANAGED_BY_VALUE}")],
    );

    let options = ListContainersOptionsBuilder::default()
        .all(true)
        .filters(&filters)
        .build();

    let containers = docker
        .list_containers(Some(options))
        .await
        .map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("failed to list containers: {e}"),
        })?;

    let infos: Vec<ContainerInfo> = containers
        .into_iter()
        .map(|c| ContainerInfo {
            id: c.id.unwrap_or_default(),
            name: c
                .names
                .and_then(|n| n.into_iter().next())
                .unwrap_or_default()
                .trim_start_matches('/')
                .to_string(),
            image: c.image.unwrap_or_default(),
            state: c.state.map(|s| s.to_string()).unwrap_or_default(),
            status: c.status.unwrap_or_default(),
        })
        .collect();

    Ok(Json(infos))
}

pub async fn get_container(
    State(docker): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ContainerInfo>, ApiError> {
    let info = docker
        .inspect_container(&id, None)
        .await
        .map_err(|e| ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("container not found: {e}"),
        })?;

    let state = info.state.as_ref();

    Ok(Json(ContainerInfo {
        id: info.id.unwrap_or_default(),
        name: info
            .name
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string(),
        image: info
            .config
            .and_then(|c| c.image)
            .unwrap_or_default(),
        state: state
            .and_then(|s| s.status.as_ref())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        status: state
            .and_then(|s| s.status.as_ref())
            .map(|s| s.to_string())
            .unwrap_or_default(),
    }))
}

pub async fn stop_container(
    State(docker): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<StopResponse>, ApiError> {
    runner::stop(&docker, &id).await?;

    Ok(Json(StopResponse {
        id,
        message: "Container stopped and removed".to_string(),
    }))
}


pub async fn get_logs(
    State(docker): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<LogsResponse>, ApiError> {
    let opts = LogsOptions {
        stdout: true,
        stderr: true,
        tail: "100".to_string(),
        ..Default::default()
    };

    let mut stream = docker.logs(&id, Some(opts));
    let mut lines = Vec::new();

    while let Some(msg) = stream.next().await {
        let msg = msg.map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("failed to read logs: {e}"),
        })?;
        lines.push(msg.to_string());
    }

    Ok(Json(LogsResponse {
        container_id: id,
        logs: lines,
    }))
}
