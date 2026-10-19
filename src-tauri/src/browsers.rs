//! Browsers yt-dlp doesn't know by name. Zen is Firefox-based, so its cookies are read with
//! `--cookies-from-browser firefox:<zen profile dir>`.

use crate::settings::Settings;
use std::path::{Path, PathBuf};

struct Profile {
    path: String,
    relative: bool,
    default: bool,
}

/// Picks the profile the browser actually uses from a profiles.ini.
///
/// `[Install…] Default=` wins: it's the profile the installed browser opens. `Default=1` on a
/// `[Profile…]` section is only a fallback; it can point at a different, unused profile.
fn default_profile(ini: &str) -> Option<(String, bool)> {
    let mut install_default: Option<String> = None;
    let mut profiles: Vec<Profile> = Vec::new();
    let mut section = String::new();

    for line in ini.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_string();
            if section.starts_with("Profile") {
                profiles.push(Profile { path: String::new(), relative: true, default: false });
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim());
        if section.starts_with("Install") {
            if key == "Default" && install_default.is_none() {
                install_default = Some(value.to_string());
            }
        } else if section.starts_with("Profile") {
            let p = profiles.last_mut().unwrap();
            match key {
                "Path" => p.path = value.to_string(),
                "IsRelative" => p.relative = value == "1",
                "Default" => p.default = value == "1",
                _ => {}
            }
        }
    }
    profiles.retain(|p| !p.path.is_empty());

    if let Some(path) = install_default {
        let relative = profiles
            .iter()
            .find(|p| p.path == path)
            .map(|p| p.relative)
            .unwrap_or(!Path::new(&path).is_absolute());
        return Some((path, relative));
    }
    profiles
        .iter()
        .find(|p| p.default)
        .or(profiles.first())
        .map(|p| (p.path.clone(), p.relative))
}

/// Where Zen keeps profiles.ini, newest layout first.
fn zen_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        roots.push(config.join("zen"));
    }
    if let Some(home) = &home {
        roots.push(home.join(".config/zen"));
        roots.push(home.join(".zen"));
        roots.push(home.join(".var/app/app.zen_browser.zen/.zen")); // Flatpak
        roots.push(home.join("Library/Application Support/zen")); // macOS
    }
    if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
        roots.push(appdata.join("zen")); // Windows
    }
    roots
}

pub fn zen_profile() -> Result<PathBuf, String> {
    for root in zen_roots() {
        let Ok(ini) = std::fs::read_to_string(root.join("profiles.ini")) else { continue };
        if let Some((path, relative)) = default_profile(&ini) {
            let dir = if relative { root.join(path) } else { PathBuf::from(path) };
            if dir.is_dir() {
                return Ok(dir);
            }
        }
    }
    Err("Couldn't find a Zen profile. Is Zen installed and has it been opened once?".into())
}

/// The value to pass to `--cookies-from-browser`.
pub fn cookies_spec(browser: &str) -> Result<String, String> {
    match browser {
        "zen" => Ok(format!("firefox:{}", zen_profile()?.display())),
        other => Ok(other.to_string()),
    }
}

/// Settings with the cookie browser translated into something yt-dlp understands.
pub fn resolve_cookies(settings: &Settings) -> Result<Settings, String> {
    let mut s = settings.clone();
    if let Some(browser) = &settings.cookies_from_browser {
        s.cookies_from_browser = Some(cookies_spec(browser)?);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shaped like a real Zen profiles.ini where `Default=1` marks an unused profile.
    const ZEN_INI: &str = "\
[General]
StartWithLastProfile=1
Version=2

[Profile0]
Name=Default (release)
IsRelative=1
Path=abcd1234.Default (release)

[Install15B76BAA26BA15E7]
Default=abcd1234.Default (release)
Locked=1

[Profile1]
Name=Default Profile
IsRelative=1
Path=wxyz9876.Default Profile
Default=1
";

    #[test]
    fn install_default_beats_profile_default_flag() {
        assert_eq!(default_profile(ZEN_INI), Some(("abcd1234.Default (release)".into(), true)));
    }

    #[test]
    fn falls_back_to_default_flag_then_first() {
        let ini = "[Profile0]\nPath=a\n\n[Profile1]\nPath=b\nDefault=1\n";
        assert_eq!(default_profile(ini), Some(("b".into(), true)));
        assert_eq!(default_profile("[Profile0]\nPath=a\n"), Some(("a".into(), true)));
        assert_eq!(default_profile("[General]\nVersion=2\n"), None);
    }

    #[test]
    fn absolute_profile_paths() {
        let ini = "[Profile0]\nIsRelative=0\nPath=/data/zen/profile\nDefault=1\n";
        assert_eq!(default_profile(ini), Some(("/data/zen/profile".into(), false)));
    }

    #[test]
    fn zen_resolves_to_firefox_profile_dir() {
        let root = std::env::temp_dir().join(format!("laverna-zen-{}", std::process::id()));
        let profile = root.join("zen/abcd1234.Default (release)");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(root.join("zen/profiles.ini"), ZEN_INI).unwrap();

        // SAFETY: tests touching XDG_CONFIG_HOME run only here.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &root) };
        let spec = cookies_spec("zen").unwrap();
        assert_eq!(spec, format!("firefox:{}", profile.display()));
        assert_eq!(cookies_spec("chrome").unwrap(), "chrome");

        std::fs::remove_dir_all(root).ok();
    }
}
