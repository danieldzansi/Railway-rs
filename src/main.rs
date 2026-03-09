mod container;
mod engine;

use anyhow::Result;
use container::runner::RunConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let docker = engine::docker::connect()?;

    let image = "nginx:latest";
    container::image::pull(&docker, image).await?;

    let cfg = RunConfig {
        image: image.to_string(),
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
