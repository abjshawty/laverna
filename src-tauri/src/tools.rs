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
            .and_then(|s| s.lines().next().map(|l| short_version(name, l))),
        None => None,
    };
    ToolInfo { path, version }
}

/// ffmpeg prints `ffmpeg version n9.0.1 Copyright (c) …`; keep just the version.
fn short_version(name: &str, line: &str) -> String {
    line.strip_prefix(&format!("{name} version "))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or(line)
        .to_string()
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

#[cfg(test)]
mod tests {
    #[test]
    fn short_version() {
        let ff = "ffmpeg version n9.0.1 Copyright (c) 2000-2026 the FFmpeg developers";
        assert_eq!(super::short_version("ffmpeg", ff), "n9.0.1");
        assert_eq!(super::short_version("yt-dlp", "2026.08.19"), "2026.08.19");
    }
}
