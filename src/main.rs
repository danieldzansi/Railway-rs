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

    // --- Determine image source ---
    // Pass a source directory as the first arg to build with Nixpacks,
    // otherwise fall back to pulling a pre-built image.
    let args: Vec<String> = env::args().collect();

    let image = if let Some(source) = args.get(1) {
        // Nixpacks build path
        builder::nixpacks::check_installed().await?;

        let plan = builder::nixpacks::plan(source).await?;
        println!("--- Build plan ---\n{plan}------------------\n");

        let build_cfg = BuildConfig {
            source: source.clone(),
            image_name: "railway-app:latest".to_string(),
            env: vec![],
            pkgs: vec![],
            build_cmd: None,
            start_cmd: None,
        };

        builder::nixpacks::build(&build_cfg).await?
    } else {
        // Fallback: pull a pre-built image
        let img = "nginx:latest";
        container::image::pull(&docker, img).await?;
        img.to_string()
    };

    // --- Run the container ---
    let cfg = RunConfig {
        image: image.clone(),
        name: "railway-demo".to_string(),
        container_port: "80/tcp".to_string(),
        host_port: 8080,
        env: vec![],
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
