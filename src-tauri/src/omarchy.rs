//! Optional Omarchy integration: a theme-set hook that copies the current Omarchy theme's colors
//! into theme.css whenever the theme changes.

use std::path::PathBuf;

const HOOK_URL: &str = "https://drive.usercontent.google.com/download?id=1gGX48ldM7QgEai3w_GtaO0sTM0-7iy2s&export=download&authuser=0&confirm=t&uuid=3b48280e-1c63-46a7-bc49-a9943063364a&at=AMrWOn2P_QceH-Srbkpf-eNr7j60:1792475527570";

/// Whether an os-release file describes Omarchy.
fn is_omarchy_release(os_release: &str) -> bool {
    os_release.lines().any(|l| {
        l.strip_prefix("ID=")
            .is_some_and(|v| v.trim_matches(|c| c == '"' || c == '\'') == "omarchy")
    })
}

fn is_omarchy() -> bool {
    std::fs::read_to_string("/etc/os-release").is_ok_and(|s| is_omarchy_release(&s))
}

fn hook_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/omarchy/hooks/theme-set.d/sync-laverna-theme"))
}

/// True when the install button should show: on Omarchy and the hook isn't installed yet.
pub fn can_install_sync() -> bool {
    is_omarchy() && hook_path().is_some_and(|p| !p.exists())
}

/// Downloads the hook and runs it once. Omarchy runs hooks with `bash`, so it needn't be executable.
pub async fn install_sync() -> Result<(), String> {
    let path = hook_path().ok_or("HOME is not set")?;
    install_to(&path).await?;
    run_hook(&path).await
}

async fn install_to(path: &std::path::Path) -> Result<(), String> {
    let part = path.with_extension("part");
    let out = tokio::process::Command::new("curl")
        .args(["-fsSL", "--create-dirs", "-o"])
        .arg(&part)
        .arg(HOOK_URL)
        .output()
        .await
        .map_err(|e| format!("couldn't run curl: {e}"))?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&part);
        return Err(format!("download failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    // Drive answers some failures with a 200 HTML page; don't install that as a hook.
    let body = std::fs::read(&part).map_err(|e| e.to_string())?;
    if !body.starts_with(b"#!") {
        let _ = std::fs::remove_file(&part);
        return Err("download didn't return the hook script".into());
    }
    std::fs::rename(&part, path).map_err(|e| e.to_string())
}

/// Runs the hook once so the current Omarchy theme applies now, not on the next theme change.
/// It writes theme.css, which the theme watcher then reloads.
async fn run_hook(path: &std::path::Path) -> Result<(), String> {
    let out = tokio::process::Command::new("bash")
        .arg(path)
        .output()
        .await
        .map_err(|e| format!("couldn't run the hook: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("installed, but the first sync failed: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_omarchy() {
        assert!(is_omarchy_release("NAME=\"Omarchy\"\nID=omarchy\nID_LIKE=arch\n"));
        assert!(is_omarchy_release("ID=\"omarchy\"\n"));
        assert!(!is_omarchy_release("NAME=\"Arch Linux\"\nID=arch\n"));
        assert!(!is_omarchy_release("ID_LIKE=omarchy\n"));
    }

    #[tokio::test]
    #[ignore = "downloads the hook (needs network)"]
    async fn installs_hook_script() {
        let dir = std::env::temp_dir().join(format!("laverna-omarchy-{}", std::process::id()));
        let path = dir.join("theme-set.d/sync-laverna-theme");
        install_to(&path).await.unwrap();
        let script = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(script.starts_with("#!/bin/bash"));
        assert!(script.contains("laverna/theme.css"));
    }
}
