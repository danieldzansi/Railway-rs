use std::path::Path;

use anyhow::{bail, Context, Result};
use tokio::process::Command;

/// Configuration for a Nixpacks build.
pub struct BuildConfig {
    /// Path to the source directory to build.
    pub source: String,
    /// Name (and optional tag) for the resulting Docker image.
    pub image_name: String,
    /// Optional environment variables passed to the build (`--env KEY=VAL`).
    pub env: Vec<String>,
    /// Optional list of Nix packages to install (`--pkgs`).
    pub pkgs: Vec<String>,
    /// Optional build command override.
    pub build_cmd: Option<String>,
    /// Optional start command override.
    pub start_cmd: Option<String>,
}

/// Verify that the `nixpacks` CLI is available on `$PATH`.
pub async fn check_installed() -> Result<()> {
    let output = Command::new("nixpacks")
        .arg("--version")
        .output()
        .await
        .context("could not find `nixpacks` – is it installed? (https://nixpacks.com)")?;

    if !output.status.success() {
        bail!("`nixpacks --version` exited with {}", output.status);
    }

    let version = String::from_utf8_lossy(&output.stdout);
    println!("Using {}", version.trim());

    Ok(())
}

pub async fn plan(source: &str) -> Result<String> {
    let source_path = Path::new(source);
    if !source_path.exists() {
        bail!("source path does not exist: {source}");
    }

    let output = Command::new("nixpacks")
        .args(["plan", source])
        .output()
        .await
        .context("failed to run `nixpacks plan`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("nixpacks plan failed:\n{stderr}");
    }

    let plan = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(plan)
}

pub async fn build(cfg: &BuildConfig) -> Result<String> {
    let source_path = Path::new(&cfg.source);
    if !source_path.exists() {
        bail!("source path does not exist: {}", cfg.source);
    }

    let mut args: Vec<String> = vec!["build".to_string(), cfg.source.clone()];

    args.push("--name".to_string());
    args.push(cfg.image_name.clone());

    for env in &cfg.env {
        args.push("--env".to_string());
        args.push(env.clone());
    }

    if !cfg.pkgs.is_empty() {
        args.push("--pkgs".to_string());
        for pkg in &cfg.pkgs {
            args.push(pkg.clone());
        }
    }

    if let Some(cmd) = &cfg.build_cmd {
        args.push("--build-cmd".to_string());
        args.push(cmd.clone());
    }

    if let Some(cmd) = &cfg.start_cmd {
        args.push("--start-cmd".to_string());
        args.push(cmd.clone());
    }

    println!("Building image '{}' from {} …", cfg.image_name, cfg.source);

    let status = Command::new("nixpacks")
        .args(&args)
        .status()
        .await
        .context("failed to run `nixpacks build`")?;

    if !status.success() {
        bail!("nixpacks build failed with {status}");
    }

    println!("Image '{}' built successfully.", cfg.image_name);

    Ok(cfg.image_name.clone())
}
