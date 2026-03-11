mod builder;
mod container;
mod engine;

use std::env;

use anyhow::Result;
use builder::nixpacks::BuildConfig;
use container::runner::RunConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = engine::docker::connect()?;

    let args: Vec<String> = env::args().collect();

    let (image, container_port, host_port, env) = if let Some(source) = args.get(1) {

        builder::nixpacks::check_installed().await?;

        let plan = builder::nixpacks::plan(source).await?;
        println!("--- Build plan ---\n{plan}------------------\n");

        let app_port: u16 = args
            .get(2)
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        let build_cfg = BuildConfig {
            source: source.clone(),
            image_name: "railway-app:latest".to_string(),
            env: vec![format!("PORT={app_port}")],
            pkgs: vec![],
            build_cmd: None,
            start_cmd: None,
        };

        let img = builder::nixpacks::build(&build_cfg).await?;
        (img, format!("{app_port}/tcp"), app_port, vec![format!("PORT={app_port}")])
    } else {
        // Fallback: pull a pre-built image
        let img = "nginx:latest";
        container::image::pull(&docker, img).await?;
        (img.to_string(), "80/tcp".to_string(), 8080u16, vec![])
    };

    // --- Run the container ---
    let cfg = RunConfig {
        image: image.clone(),
        name: "railway-demo".to_string(),
        container_port,
        host_port,
        env,
    };

    let id = container::runner::start(&docker, &cfg).await?;

    println!("Streaming logs (press Ctrl+C to stop) …");
    tokio::select! {
        res = container::logs::stream(&docker, &id) => { res?; }
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutting down …");
        }
    }

    container::runner::stop(&docker, &id).await?;
    Ok(())
}
