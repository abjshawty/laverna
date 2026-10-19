use crate::settings::{AudioFormat, Container, Preset, Settings};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const PROGRESS_PREFIX: &str = "[laverna] ";
pub const POSTPROCESS_PREFIX: &str = "[laverna-pp] ";
pub const FILE_PREFIX: &str = "[laverna-file] ";

// `|null` makes missing fields render as JSON null instead of a bare `NA`.
const PROGRESS_TEMPLATE: &str = concat!(
    "download:[laverna] {",
    r#""status":%(progress.status|null)j,"#,
    r#""downloaded":%(progress.downloaded_bytes|null)j,"#,
    r#""total":%(progress.total_bytes|null)j,"#,
    r#""total_estimate":%(progress.total_bytes_estimate|null)j,"#,
    r#""speed":%(progress.speed|null)j,"#,
    r#""eta":%(progress.eta|null)j,"#,
    r#""vcodec":%(info.vcodec|null)j,"#,
    r#""acodec":%(info.acodec|null)j"#,
    "}"
);
const POSTPROCESS_TEMPLATE: &str = concat!(
    "postprocess:[laverna-pp] {",
    r#""status":%(progress.status|null)j,"#,
    r#""postprocessor":%(progress.postprocessor|null)j"#,
    "}"
);

/// Per-item choices, prefilled from settings when the item is added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemOptions {
    pub preset: Preset,
    pub subtitles: bool,
    /// Bypass the download archive ("download anyway").
    #[serde(default)]
    pub force: bool,
}

pub struct Paths<'a> {
    pub output_dir: &'a Path,
    pub archive: &'a Path,
    pub ffmpeg: Option<&'a Path>,
}

#[derive(Debug, thiserror::Error)]
pub enum ArgsError {
    #[error("could not parse extra yt-dlp args: {0}")]
    ExtraArgs(#[from] shell_words::ParseError),
}

pub fn download_args(
    url: &str,
    opts: &ItemOptions,
    settings: &Settings,
    paths: Paths,
) -> Result<Vec<String>, ArgsError> {
    let mut a: Vec<String> = Vec::new();
    let mut push = |xs: &[&str]| a.extend(xs.iter().map(|s| s.to_string()));

    push(&["--newline", "--no-colors", "--progress", "--continue", "--no-playlist"]);
    push(&["--progress-template", PROGRESS_TEMPLATE]);
    push(&["--progress-template", POSTPROCESS_TEMPLATE]);
    push(&["--print", "after_move:[laverna-file] %(filepath)s"]);
    push(&["-P", &paths.output_dir.to_string_lossy()]);
    push(&["-o", &settings.filename_template]);
    if !opts.force {
        push(&["--download-archive", &paths.archive.to_string_lossy()]);
    }
    if let Some(ff) = paths.ffmpeg {
        push(&["--ffmpeg-location", &ff.to_string_lossy()]);
    }

    if opts.preset == Preset::Audio {
        push(&["-f", "ba/b", "-x"]);
        match settings.audio_format {
            AudioFormat::Original => {}
            AudioFormat::Mp3 => push(&["--audio-format", "mp3"]),
            AudioFormat::M4a => push(&["--audio-format", "m4a"]),
        }
    } else {
        push(&["-f", "bv*+ba/b"]);
        let mut sort: Vec<String> = match settings.container {
            Container::Mkv => vec![],
            Container::Mp4 => vec!["vcodec:h264".into(), "acodec:aac".into()],
            Container::Webm => vec!["vcodec:vp9".into(), "acodec:opus".into()],
        };
        if let Some(h) = opts.preset.max_height() {
            sort.push(format!("res:{h}"));
        }
        if !sort.is_empty() {
            push(&["-S", &sort.join(",")]);
        }
        push(&["--merge-output-format", settings.container.as_str()]);

        if opts.subtitles {
            push(&["--write-subs", "--embed-subs", "--sub-langs", &settings.subtitles.languages]);
            if settings.subtitles.auto_generated {
                push(&["--write-auto-subs"]);
            }
        }
    }

    push(&["--embed-metadata", "--embed-thumbnail", "--embed-chapters"]);

    if let Some(browser) = &settings.cookies_from_browser {
        push(&["--cookies-from-browser", browser]);
    }
    if !settings.rate_limit.trim().is_empty() {
        push(&["-r", settings.rate_limit.trim()]);
    }

    a.extend(shell_words::split(&settings.extra_args)?);
    a.push("--".into());
    a.push(url.into());
    Ok(a)
}

/// Resolves metadata without downloading; playlists come back flat.
pub fn metadata_args(url: &str, settings: &Settings) -> Vec<String> {
    // `--no-playlist` only affects URLs that are both a video and a playlist (watch?v=…&list=…):
    // those resolve to the video. Pure playlist URLs still expand.
    let mut a: Vec<String> = ["-J", "--flat-playlist", "--no-playlist", "--no-warnings"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    if let Some(browser) = &settings.cookies_from_browser {
        a.extend(["--cookies-from-browser".into(), browser.clone()]);
    }
    a.extend(["--".into(), url.into()]);
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn build(opts: &ItemOptions, s: &Settings) -> Vec<String> {
        let out = PathBuf::from("/out");
        let archive = PathBuf::from("/data/archive.txt");
        download_args(
            "https://youtu.be/x",
            opts,
            s,
            Paths { output_dir: &out, archive: &archive, ffmpeg: None },
        )
        .unwrap()
    }

    fn value_of<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        args.iter().position(|a| a == flag).map(|i| args[i + 1].as_str())
    }

    fn opts(preset: Preset) -> ItemOptions {
        ItemOptions { preset, subtitles: false, force: false }
    }

    #[test]
    fn best_mkv_has_no_sort_and_merges_to_mkv() {
        let a = build(&opts(Preset::Best), &Settings::default());
        assert_eq!(value_of(&a, "-f"), Some("bv*+ba/b"));
        assert_eq!(value_of(&a, "-S"), None);
        assert_eq!(value_of(&a, "--merge-output-format"), Some("mkv"));
        assert_eq!(value_of(&a, "--download-archive"), Some("/data/archive.txt"));
        assert_eq!(a[a.len() - 2..], ["--".to_string(), "https://youtu.be/x".to_string()]);
    }

    #[test]
    fn height_preset_sorts_by_resolution() {
        let a = build(&opts(Preset::P1080), &Settings::default());
        assert_eq!(value_of(&a, "-S"), Some("res:1080"));
    }

    #[test]
    fn mp4_prefers_h264_aac_before_resolution() {
        let s = Settings { container: Container::Mp4, ..Default::default() };
        let a = build(&opts(Preset::P720), &s);
        assert_eq!(value_of(&a, "-S"), Some("vcodec:h264,acodec:aac,res:720"));
        assert_eq!(value_of(&a, "--merge-output-format"), Some("mp4"));
    }

    #[test]
    fn audio_only_extracts_and_ignores_subtitles() {
        let s = Settings { audio_format: AudioFormat::Mp3, ..Default::default() };
        let a = build(&ItemOptions { preset: Preset::Audio, subtitles: true, force: false }, &s);
        assert_eq!(value_of(&a, "-f"), Some("ba/b"));
        assert!(a.contains(&"-x".to_string()));
        assert_eq!(value_of(&a, "--audio-format"), Some("mp3"));
        assert!(!a.contains(&"--embed-subs".to_string()));
        assert!(!a.contains(&"--merge-output-format".to_string()));
    }

    #[test]
    fn audio_original_has_no_conversion() {
        let a = build(&opts(Preset::Audio), &Settings::default());
        assert_eq!(value_of(&a, "--audio-format"), None);
    }

    #[test]
    fn subtitles_with_auto_generated() {
        let mut s = Settings::default();
        s.subtitles.languages = "en,fr".into();
        s.subtitles.auto_generated = true;
        let a = build(&ItemOptions { preset: Preset::Best, subtitles: true, force: false }, &s);
        assert_eq!(value_of(&a, "--sub-langs"), Some("en,fr"));
        assert!(a.contains(&"--embed-subs".to_string()));
        assert!(a.contains(&"--write-auto-subs".to_string()));
    }

    #[test]
    fn force_skips_archive() {
        let a = build(&ItemOptions { preset: Preset::Best, subtitles: false, force: true }, &Settings::default());
        assert_eq!(value_of(&a, "--download-archive"), None);
    }

    #[test]
    fn power_user_settings_are_appended_before_url() {
        let s = Settings {
            cookies_from_browser: Some("firefox".into()),
            rate_limit: " 2M ".into(),
            extra_args: r#"--sponsorblock-remove "sponsor,intro""#.into(),
            ..Default::default()
        };
        let a = build(&opts(Preset::Best), &s);
        assert_eq!(value_of(&a, "--cookies-from-browser"), Some("firefox"));
        assert_eq!(value_of(&a, "-r"), Some("2M"));
        assert_eq!(value_of(&a, "--sponsorblock-remove"), Some("sponsor,intro"));
        assert_eq!(a[a.len() - 2], "--");
    }

    #[test]
    fn unbalanced_extra_args_is_an_error() {
        let s = Settings { extra_args: r#"--foo "bar"#.into(), ..Default::default() };
        let out = PathBuf::from("/out");
        let r = download_args("u", &opts(Preset::Best), &s, Paths { output_dir: &out, archive: &out, ffmpeg: None });
        assert!(r.is_err());
    }
}
