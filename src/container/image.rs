use anyhow::Result;
use bollard::query_parameters::CreateImageOptions;
use bollard::Docker;
use futures_util::StreamExt;

pub async fn pull(docker: &Docker, image: &str) -> Result<()> {
    let (repo, tag) = match image.split_once(':') {
        Some((r, t)) => (r, t),
        None => (image, "latest"),
    };

    println!("Pulling {repo}:{tag} …");

    let opts = CreateImageOptions {
        from_image: Some(repo.to_string()),
        tag: Some(tag.to_string()),
        ..Default::default()
    };

    let mut stream = docker.create_image(Some(opts), None, None);

    while let Some(info) = stream.next().await {
        let info = info?;
        if let Some(status) = info.status {
            println!("  {status}");
        }
    }

    println!("Pull complete.");
    Ok(())
}
