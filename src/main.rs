mod api;
mod builder;
mod container;
mod engine;

use std::net::SocketAddr;

use anyhow::Result;
use builder::nixpacks::BuildConfig;
use clap::{Parser, Subcommand};
use container::runner::RunConfig;
use tokio::net::TcpListener;

// ── CLI definition ──

#[derive(Parser)]
#[command(name = "railway-rs", version, about = "Build, deploy & manage containers from source")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build source with Nixpacks and run the container
    Deploy {
        source: String,
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
        #[arg(short = 'n', long, default_value = "railway-app:latest")]
        name: String,
        #[arg(short, long)]
        env: Vec<String>,
        #[arg(long)]
        pkgs: Vec<String>,
        #[arg(long)]
        build_cmd: Option<String>,
        #[arg(long)]
        start_cmd: Option<String>,
    },
    Serve {
        #[arg(short, long, default_value_t = 3001)]
        port: u16,
    },
    Ps,
    Inspect {
        id: String,
    },
    Logs {
        id: String,
        #[arg(short = 't', long, default_value_t = 100)]
        tail: u32,
    },
    Stop {
        id: String,
    },
    Remove {
        id : String,
    },
    Start{
        id : String ,
    }
    
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy {
            source,
            port,
            name,
            env,
            pkgs,
            build_cmd,
            start_cmd,
        } => cmd_deploy(source, port, name, env, pkgs, build_cmd, start_cmd).await,
        Commands::Serve { port } => cmd_serve(port).await,
        Commands::Ps => cmd_ps().await,
        Commands::Inspect { id } => cmd_inspect(id).await,
        Commands::Logs { id, tail } => cmd_logs(id, tail).await,
        Commands::Stop { id } => cmd_stop(id).await,
        Commands::Remove{ id } =>cmd_remove(id).await,
        Commands::Start{ id } =>cmd_start(id).await,
    }
}

async fn cmd_deploy(
    source: String,
    port: u16,
    name: String,
    mut env: Vec<String>,
    pkgs: Vec<String>,
    build_cmd: Option<String>,
    start_cmd: Option<String>,
) -> Result<()> {
    let docker = engine::docker::connect()?;

    builder::nixpacks::check_installed().await?;

    let plan = builder::nixpacks::plan(&source).await?;
    println!("--- Build plan ---\n{plan}------------------\n");

    if !env.iter().any(|e| e.starts_with("PORT=")) {
        env.push(format!("PORT={port}"));
    }

    let build_cfg = BuildConfig {
        source,
        image_name: name.clone(),
        env: env.clone(),
        pkgs,
        build_cmd,
        start_cmd,
    };

    let image = builder::nixpacks::build(&build_cfg).await?;

    let container_port = format!("{port}/tcp");
    let run_cfg = RunConfig {
        image: image.clone(),
        name: format!("railway-{}", name.replace(':', "-")),
        container_port,
        host_port: port,
        env,
    };

    let id = container::runner::start(&docker, &run_cfg).await?;

    println!("\nStreaming logs (press Ctrl+C to stop) …");
    tokio::select! {
        res = container::logs::stream(&docker, &id) => { res?; }
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutting down …");
        }
    }

    container::runner::stop(&docker, &id).await?;
    Ok(())
}

async fn cmd_serve(port: u16) -> Result<()> {
    let docker = engine::docker::connect()?;
    let app = api::router(docker);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Railway API server listening on http://{addr}");

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            println!("\nShutting down API server …");
        })
        .await?;

    Ok(())
}

async fn cmd_ps() -> Result<()> {
    use bollard::query_parameters::ListContainersOptionsBuilder;
    use container::runner::{LABEL_MANAGED_BY, LABEL_MANAGED_BY_VALUE};

    let docker = engine::docker::connect()?;

    let mut filters = std::collections::HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![format!("{LABEL_MANAGED_BY}={LABEL_MANAGED_BY_VALUE}")],
    );

    let options = ListContainersOptionsBuilder::default()
        .all(true)
        .filters(&filters)
        .build();
    let containers = docker.list_containers(Some(options)).await?;

    if containers.is_empty() {
        println!("No containers found.");
        return Ok(());
    }

    println!(
        "{:<14} {:<30} {:<25} {:<12} {}",
        "CONTAINER ID", "NAME", "IMAGE", "STATE", "STATUS"
    );

    for c in containers {
        let id = c.id.unwrap_or_default();
        let short_id = &id[..id.len().min(12)];
        let name = c
            .names
            .and_then(|n| n.into_iter().next())
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();
        let image = c.image.unwrap_or_default();
        let state = c.state.map(|s| s.to_string()).unwrap_or_default();
        let status = c.status.unwrap_or_default();

        println!("{short_id:<14} {name:<30} {image:<25} {state:<12} {status}");
    }

    Ok(())
}

async fn cmd_inspect(id: String) -> Result<()> {
    let docker = engine::docker::connect()?;
    let info = docker.inspect_container(&id, None).await?;

    let container_id = info.id.unwrap_or_default();
    let name = info
        .name
        .unwrap_or_default()
        .trim_start_matches('/')
        .to_string();
    let image = info
        .config
        .as_ref()
        .and_then(|c| c.image.clone())
        .unwrap_or_default();
    let state_obj = info.state.as_ref();
    let state = state_obj
        .and_then(|s| s.status.as_ref())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let started = state_obj
        .and_then(|s| s.started_at.clone())
        .unwrap_or_default();

    let ports = info
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref())
        .map(|ports| {
            ports
                .iter()
                .map(|(k, v)| {
                    let bindings = v
                        .as_ref()
                        .map(|bs| {
                            bs.iter()
                                .map(|b| {
                                    format!(
                                        "{}:{}",
                                        b.host_ip.as_deref().unwrap_or("0.0.0.0"),
                                        b.host_port.as_deref().unwrap_or("?")
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{k} → {bindings}")
                })
                .collect::<Vec<_>>()
                .join("\n         ")
        })
        .unwrap_or_else(|| "none".to_string());

    println!("ID:      {container_id}");
    println!("Name:    {name}");
    println!("Image:   {image}");
    println!("State:   {state}");
    println!("Started: {started}");
    println!("Ports:   {ports}");

    Ok(())
}

async fn cmd_logs(id: String, tail: u32) -> Result<()> {
    use bollard::query_parameters::LogsOptions;
    use futures_util::StreamExt;

    let docker = engine::docker::connect()?;

    let opts = LogsOptions {
        follow: true,
        stdout: true,
        stderr: true,
        tail: tail.to_string(),
        ..Default::default()
    };

    println!("Streaming logs for {id} (Ctrl+C to stop) …\n");

    let mut stream = docker.logs(&id, Some(opts));

    tokio::select! {
        _ = async {
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(m) => print!("{m}"),
                    Err(e) => {
                        eprintln!("Error reading logs: {e}");
                        break;
                    }
                }
            }
        } => {}
        _ = tokio::signal::ctrl_c() => {
            println!("\nStopped streaming.");
        }
    }

    Ok(())
}

async fn cmd_stop(id: String) -> Result<()> {
    let docker = engine::docker::connect()?;
    container::runner::stop(&docker, &id).await?;
    Ok(())
}


async fn cmd_remove(id: String) -> Result <()>{
    let docker = engine::docker::connect()?;
    container::runner::remove(&docker, &id).await?;
    Ok(())
}

async fn cmd_start(id: String) ->Result <()>{
    let docker = engine::docker::connect()?;
    container::runner::start_existing(&docker, &id).await?;
    Ok(())
}