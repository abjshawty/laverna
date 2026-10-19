use super::args::{FILE_PREFIX, POSTPROCESS_PREFIX, PROGRESS_PREFIX};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Video,
    Audio,
    /// A single stream carrying both (e.g. format `b`).
    Download,
    Processing,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Progress {
    pub phase: Phase,
    /// 0.0..=1.0, `None` when the total size is unknown.
    pub fraction: Option<f64>,
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
    /// Bytes per second.
    pub speed: Option<f64>,
    pub eta: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    Progress(Progress),
    /// A postprocessor started (merging, embedding, …).
    Processing(String),
    /// Final path after all postprocessing.
    File(String),
    /// Anything else: kept for the item's log.
    Other(String),
}

#[derive(Deserialize)]
struct RawProgress {
    status: Option<String>,
    downloaded: Option<f64>,
    total: Option<f64>,
    total_estimate: Option<f64>,
    speed: Option<f64>,
    eta: Option<f64>,
    vcodec: Option<String>,
    acodec: Option<String>,
}

#[derive(Deserialize)]
struct RawPostprocess {
    status: Option<String>,
    postprocessor: Option<String>,
}

fn has_codec(c: &Option<String>) -> bool {
    matches!(c.as_deref(), Some(c) if c != "none")
}

pub fn parse_line(line: &str) -> Line {
    let line = line.trim_end_matches(['\r', '\n']);
    if let Some(json) = line.strip_prefix(PROGRESS_PREFIX) {
        if let Ok(raw) = serde_json::from_str::<RawProgress>(json) {
            let phase = match (has_codec(&raw.vcodec), has_codec(&raw.acodec)) {
                (true, false) => Phase::Video,
                (false, true) => Phase::Audio,
                _ => Phase::Download,
            };
            let total = raw.total.or(raw.total_estimate).filter(|t| *t > 0.0);
            let finished = raw.status.as_deref() == Some("finished");
            let fraction = if finished {
                Some(1.0)
            } else {
                match (raw.downloaded, total) {
                    (Some(d), Some(t)) => Some((d / t).clamp(0.0, 1.0)),
                    _ => None,
                }
            };
            return Line::Progress(Progress {
                phase,
                fraction,
                downloaded: raw.downloaded.map(|d| d as u64),
                total: total.map(|t| t as u64),
                speed: raw.speed,
                eta: raw.eta.map(|e| e as u64),
            });
        }
    } else if let Some(json) = line.strip_prefix(POSTPROCESS_PREFIX) {
        if let Ok(raw) = serde_json::from_str::<RawPostprocess>(json) {
            if raw.status.as_deref() == Some("started") {
                return Line::Processing(raw.postprocessor.unwrap_or_default());
            }
            return Line::Other(line.to_string());
        }
    } else if let Some(path) = line.strip_prefix(FILE_PREFIX) {
        return Line::File(path.to_string());
    }
    Line::Other(line.to_string())
}

/// yt-dlp prints `ERROR: ...` on failure; the last one is the useful one.
pub fn last_error(log: &[String]) -> Option<&str> {
    log.iter()
        .rev()
        .find_map(|l| l.strip_prefix("ERROR: "))
}

/// Heuristic for "this is yt-dlp being out of date / YouTube changed something".
pub fn looks_like_extractor_error(err: &str) -> bool {
    let e = err.to_lowercase();
    ["unable to extract", "nsig", "signature", "unsupported url", "http error 403", "requested format is not available"]
        .iter()
        .any(|k| e.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Lines below are shaped like real yt-dlp 2026.08 output for our templates.
    #[test]
    fn parses_video_stream_progress() {
        let l = r#"[laverna] {"status":"downloading","downloaded":3072,"total":2848208,"total_estimate":null,"speed":761339.03,"eta":3,"vcodec":"vp9","acodec":"none"}"#;
        let Line::Progress(p) = parse_line(l) else { panic!() };
        assert_eq!(p.phase, Phase::Video);
        assert_eq!(p.downloaded, Some(3072));
        assert_eq!(p.total, Some(2848208));
        assert_eq!(p.eta, Some(3));
        assert!((p.fraction.unwrap() - 3072.0 / 2848208.0).abs() < 1e-9);
    }

    #[test]
    fn audio_phase_and_estimate_fallback() {
        let l = r#"[laverna] {"status":"downloading","downloaded":50,"total":null,"total_estimate":200,"speed":null,"eta":null,"vcodec":"none","acodec":"opus"}"#;
        let Line::Progress(p) = parse_line(l) else { panic!() };
        assert_eq!(p.phase, Phase::Audio);
        assert_eq!(p.fraction, Some(0.25));
        assert_eq!(p.speed, None);
    }

    #[test]
    fn unknown_codecs_and_total() {
        let l = r#"[laverna] {"status":"downloading","downloaded":1024,"total":null,"total_estimate":null,"speed":null,"eta":null,"vcodec":null,"acodec":null}"#;
        let Line::Progress(p) = parse_line(l) else { panic!() };
        assert_eq!(p.phase, Phase::Download);
        assert_eq!(p.fraction, None);
    }

    #[test]
    fn finished_is_full() {
        let l = r#"[laverna] {"status":"finished","downloaded":10,"total":null,"total_estimate":null,"speed":null,"eta":null,"vcodec":"h264","acodec":"aac"}"#;
        let Line::Progress(p) = parse_line(l) else { panic!() };
        assert_eq!(p.fraction, Some(1.0));
    }

    #[test]
    fn postprocess_and_file_lines() {
        assert_eq!(
            parse_line(r#"[laverna-pp] {"status":"started","postprocessor":"Merger"}"#),
            Line::Processing("Merger".into())
        );
        assert!(matches!(
            parse_line(r#"[laverna-pp] {"status":"finished","postprocessor":"Merger"}"#),
            Line::Other(_)
        ));
        assert_eq!(
            parse_line("[laverna-file] /out/a [x].mkv\n"),
            Line::File("/out/a [x].mkv".into())
        );
    }

    #[test]
    fn malformed_json_is_kept_as_log() {
        assert!(matches!(parse_line("[laverna] {nope"), Line::Other(_)));
        assert!(matches!(parse_line("WARNING: something"), Line::Other(_)));
    }

    #[test]
    fn finds_last_error_and_classifies() {
        let log = vec![
            "ERROR: first".to_string(),
            "WARNING: x".to_string(),
            "ERROR: [youtube] abc: Unable to extract uploader id".to_string(),
        ];
        let e = last_error(&log).unwrap();
        assert!(e.contains("uploader id"));
        assert!(looks_like_extractor_error(e));
        assert!(!looks_like_extractor_error("Sign in to confirm you're not a bot"));
    }
}
