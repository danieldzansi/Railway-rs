use std::collections::HashMap;

use anyhow::{Context, Result};
use bollard::models::{ContainerCreateBody, HostConfig, PortBinding};
use bollard::query_parameters::{CreateContainerOptions, RemoveContainerOptions};
use bollard::Docker;

pub const LABEL_MANAGED_BY: &str = "managed-by";
pub const LABEL_MANAGED_BY_VALUE: &str = "railway-rs";


pub struct RunConfig {
    pub image: String,
    pub name: String,
    pub container_port: String,
    pub host_port: u16,
    pub env: Vec<String>,
}

pub async fn start(docker: &Docker, cfg: &RunConfig) -> Result<String> {
    let port_binding = PortBinding {
        host_ip: Some("0.0.0.0".to_string()),
        host_port: Some(cfg.host_port.to_string()),
    };

    let mut port_bindings = HashMap::new();
    port_bindings.insert(cfg.container_port.clone(), Some(vec![port_binding]));

    let host_config = HostConfig {
        port_bindings: Some(port_bindings),
        ..Default::default()
    };

    let mut labels = HashMap::new();
    labels.insert(LABEL_MANAGED_BY.to_string(), LABEL_MANAGED_BY_VALUE.to_string());

    let container_config = ContainerCreateBody {
        image: Some(cfg.image.clone()),
        env: Some(cfg.env.clone()),
        exposed_ports: Some(vec![cfg.container_port.clone()]),
        host_config: Some(host_config),
        labels: Some(labels),
        ..Default::default()
    };

    let create_opts = CreateContainerOptions {
        name: Some(cfg.name.clone()),
        ..Default::default()
    };

    let container = docker
        .create_container(Some(create_opts), container_config)
        .await
        .context("failed to create container")?;

    docker
        .start_container(&container.id, None)
        .await
        .context("failed to start container")?;

    println!(
        "Container {} started ({}), port {} → host {}",
        cfg.name, &container.id[..12], cfg.container_port, cfg.host_port
    );

    Ok(container.id)
}

pub async fn stop(docker: &Docker, id: &str) -> Result<()> {
    docker
        .stop_container(id, None)
        .await
        .context("failed to stop container")?;

    // docker
    //     // .remove_container(
    //     //     id,
    //     //     Some(RemoveContainerOptions {
    //     //         force: true,
    //     //         ..Default::default()
    //     //     }),
    //     // )
    //     .await
    //     .context("failed to remove container")?;

    println!("Container {id} stopped ");
    Ok(())
}

pub async fn remove(docker: &Docker, id : &str) ->Result <()>{
    docker
    .remove_container(id, None)
    .await
    .context("failed to remove container")?;
     
    println!("Container {id} removed");
    Ok(())
}