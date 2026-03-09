use anyhow::{Context, Result};
use bollard::Docker;

pub fn connect() -> Result<Docker> {
    Docker::connect_with_local_defaults().context("failed to connect to Docker daemon")
}
