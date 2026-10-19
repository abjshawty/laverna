use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub path: Option<PathBuf>,
    pub version: Option<String>,
}

/// Resolves `name` from an explicit override or `PATH`, then asks it for its version.
pub async fn detect(name: &str, override_path: Option<&Path>, version_flag: &str) -> ToolInfo {
    let path = resolve(name, override_path);
    let version = match &path {
        Some(p) => tokio::process::Command::new(p)
            .arg(version_flag)
            .output()
            .await
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.lines().next().map(str::to_string)),
        None => None,
    };
    ToolInfo { path, version }
}

/// An explicit override wins (and must exist); otherwise `PATH` is searched.
pub fn resolve(name: &str, override_path: Option<&Path>) -> Option<PathBuf> {
    match override_path {
        Some(p) => Some(p.to_path_buf()).filter(|p| p.is_file()),
        None => find_in_path(name),
    }
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(&exe))
        .find(|p| p.is_file())
}
