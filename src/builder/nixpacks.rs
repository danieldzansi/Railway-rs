use std::path::Path;

use anyhow::{bail, Context, Result};
use tokio::process::Command;

pub struct BuildConfig {
    pub source: String,
    pub image_name: String,
    pub env: Vec<String>,
    pub pkgs: Vec<String>,
    pub build_cmd: Option<String>,
    pub start_cmd: Option<String>,
}

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

/// Reads package.json and returns the major Node version (e.g. "22")
/// Falls back to "22" if not found or unparseable
fn detect_node_version(source: &str) -> Option<String> {
    let pkg_path = Path::new(source).join("package.json");
    if !pkg_path.exists() {
        return None; // Not a Node project
    }

    let content = std::fs::read_to_string(&pkg_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Try engines.node first (e.g. ">=18.0.0" or "22")
    if let Some(engines_node) = json["engines"]["node"].as_str() {
        let version: String = engines_node.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
        let major = version.split('.').next()?;
        if !major.is_empty() {
            return Some(major.to_string());
        }
    }

    // Fall back to "22" for any Node project without engines specified
    Some("22".to_string())
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

    // Merge user-supplied env with auto-detected Node version
    let mut env = cfg.env.clone();

    // If it's a Node project and NIXPACKS_NODE_VERSION isn't already set, inject it
    let already_set = env.iter().any(|e| e.starts_with("NIXPACKS_NODE_VERSION="));
    if !already_set {
        if let Some(node_version) = detect_node_version(&cfg.source) {
            println!("Detected Node.js project, pinning NIXPACKS_NODE_VERSION={node_version}");
            env.push(format!("NIXPACKS_NODE_VERSION={node_version}"));
        }
    }

    for e in &env {
        args.push("--env".to_string());
        args.push(e.clone());
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

    let output = Command::new("nixpacks")
        .args(&args)
        .output()
        .await
        .context("failed to run `nixpacks build`")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        println!("{stdout}");
    }

    if !output.status.success() {
        if !stderr.is_empty() {
            eprintln!("{stderr}");
        }
        bail!("nixpacks build failed with {}", output.status);
    }

    println!("Image '{}' built successfully.", cfg.image_name);

    Ok(cfg.image_name.clone())
}