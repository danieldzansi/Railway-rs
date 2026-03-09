use anyhow::Result;
use bollard::query_parameters::LogsOptions;
use bollard::Docker;
use futures_util::StreamExt;

pub async fn stream(docker: &Docker, container_id: &str) -> Result<()> {
    let opts = LogsOptions {
        follow: true,
        stdout: true,
        stderr: true,
        ..Default::default()
    };

    let mut stream = docker.logs(container_id, Some(opts));

    while let Some(msg) = stream.next().await {
        let msg = msg?;
        print!("{msg}");
    }

    Ok(())
}
