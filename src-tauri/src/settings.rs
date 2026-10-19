use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    #[default]
    Best,
    #[serde(rename = "2160p")]
    P2160,
    #[serde(rename = "1440p")]
    P1440,
    #[serde(rename = "1080p")]
    P1080,
    #[serde(rename = "720p")]
    P720,
    #[serde(rename = "480p")]
    P480,
    Audio,
}

impl Preset {
    pub fn max_height(self) -> Option<u32> {
        match self {
            Preset::P2160 => Some(2160),
            Preset::P1440 => Some(1440),
            Preset::P1080 => Some(1080),
            Preset::P720 => Some(720),
            Preset::P480 => Some(480),
            Preset::Best | Preset::Audio => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    #[default]
    Mkv,
    Mp4,
    Webm,
}

impl Container {
    pub fn as_str(self) -> &'static str {
        match self {
            Container::Mkv => "mkv",
            Container::Mp4 => "mp4",
            Container::Webm => "webm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    /// Keep the original stream, no re-encode.
    #[default]
    Original,
    Mp3,
    M4a,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Follow `prefers-color-scheme`.
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SubtitleSettings {
    pub include_by_default: bool,
    /// Comma-separated, passed to `--sub-langs`.
    pub languages: String,
    pub auto_generated: bool,
}

impl Default for SubtitleSettings {
    fn default() -> Self {
        Self {
            include_by_default: false,
            languages: "en".into(),
            auto_generated: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub output_dir: Option<PathBuf>,
    pub filename_template: String,
    pub default_preset: Preset,
    pub container: Container,
    pub audio_format: AudioFormat,
    pub subtitles: SubtitleSettings,
    pub ytdlp_path: Option<PathBuf>,
    pub ffmpeg_path: Option<PathBuf>,
    /// Browser name for `--cookies-from-browser`; `None` = off.
    pub cookies_from_browser: Option<String>,
    /// Value for `-r`, e.g. `2M`; empty = unlimited.
    pub rate_limit: String,
    /// Free text, shell-split and appended last.
    pub extra_args: String,
    pub theme_mode: ThemeMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: None,
            filename_template: "%(title)s [%(id)s].%(ext)s".into(),
            default_preset: Preset::Best,
            container: Container::Mkv,
            audio_format: AudioFormat::Original,
            subtitles: SubtitleSettings::default(),
            ytdlp_path: None,
            ffmpeg_path: None,
            cookies_from_browser: None,
            rate_limit: String::new(),
            extra_args: String::new(),
            theme_mode: ThemeMode::System,
        }
    }
}

impl Settings {
    /// Missing or unreadable file → defaults. Unknown/missing fields → defaults per field.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_json_fills_defaults() {
        let s: Settings = serde_json::from_str(r#"{"container":"mp4"}"#).unwrap();
        assert_eq!(s.container, Container::Mp4);
        assert_eq!(s.default_preset, Preset::Best);
        assert_eq!(s.subtitles.languages, "en");
    }

    #[test]
    fn preset_serializes_as_label() {
        assert_eq!(serde_json::to_string(&Preset::P1080).unwrap(), r#""1080p""#);
        assert_eq!(serde_json::to_string(&Preset::Audio).unwrap(), r#""audio""#);
    }
}
