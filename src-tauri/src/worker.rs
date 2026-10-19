//! The single download worker: takes one pending item at a time and runs yt-dlp on it.

use crate::browsers;
use crate::core::{Core, Running};
use crate::queue::ItemId;
use crate::tools;
use crate::ytdlp::args::{self, ItemOptions, Paths};
use crate::ytdlp::progress::{self, Line, Phase, Progress};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

struct Job {
    id: ItemId,
    ytdlp: PathBuf,
    args: Vec<String>,
}

pub async fn run(core: Arc<Core>) {
    let mut did_work = false;
    loop {
        match next_job(&core) {
            Some(job) => {
                did_work = true;
                run_job(&core, job).await;
            }
            None => {
                let idle = core.read(|q| !q.paused && q.remaining() == 0);
                if did_work && idle {
                    core.notify("Downloads finished", "The queue is empty.");
                    did_work = false;
                }
                core.wake.notified().await;
            }
        }
    }
}

fn next_job(core: &Core) -> Option<Job> {
    let settings = core.settings();
    // Without yt-dlp nothing can run; the UI shows a banner. Saving settings wakes us again.
    let ytdlp = tools::resolve("yt-dlp", settings.ytdlp_path.as_deref())?;
    // Check read-only first: `update` wakes the worker, which would spin if nothing can start.
    if !core.read(|q| q.can_start()) {
        return None;
    }

    let (id, url, opts): (ItemId, String, ItemOptions) =
        core.update(|q| q.start_next().map(|i| (i.id, i.url.clone(), i.options.clone())))?;

    let output_dir = match &settings.output_dir {
        Some(d) => d.clone(),
        None => core.app.path().video_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    let built = std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("cannot create {}: {e}", output_dir.display()))
        .and_then(|_| browsers::resolve_cookies(&settings))
        .and_then(|settings| {
            args::download_args(
                &url,
                &opts,
                &settings,
                Paths {
                    output_dir: &output_dir,
                    archive: &core.paths.archive,
                    ffmpeg: settings.ffmpeg_path.as_deref(),
                },
            )
            .map_err(|e| e.to_string())
        });

    match built {
        Ok(args) => Some(Job { id, ytdlp, args }),
        Err(error) => {
            core.update(|q| q.fail(id, error, false)).ok();
            core.notify_if_failed(id);
            // Let the loop move on to the next item.
            core.wake.notify_one();
            None
        }
    }
}

async fn run_job(core: &Core, job: Job) {
    let Job { id, ytdlp, args } = job;
    let mut cmd = tokio::process::Command::new(&ytdlp);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            core.update(|q| q.fail(id, format!("could not start yt-dlp: {e}"), false)).ok();
            core.notify_if_failed(id);
            return;
        }
    };
    *core.current.lock().unwrap() = child.id().map(|pid| Running { id, pid });

    let (tx, mut rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(forward_lines(child.stdout.take().unwrap(), tx.clone()));
    tauri::async_runtime::spawn(forward_lines(child.stderr.take().unwrap(), tx));

    let mut file = None;
    let mut last_emit: Option<(Instant, Phase)> = None;
    while let Some(line) = rx.recv().await {
        match progress::parse_line(&line) {
            Line::Progress(p) => {
                let due = match last_emit {
                    None => true,
                    Some((t, phase)) => phase != p.phase || t.elapsed() >= PROGRESS_INTERVAL || p.fraction == Some(1.0),
                };
                if due {
                    last_emit = Some((Instant::now(), p.phase));
                    core.progress(id, p);
                }
            }
            Line::Processing(name) => {
                core.log(id, format!("[postprocess] {name}"));
                last_emit = Some((Instant::now(), Phase::Processing));
                core.progress(
                    id,
                    Progress { phase: Phase::Processing, fraction: None, downloaded: None, total: None, speed: None, eta: None },
                );
            }
            Line::File(path) => file = Some(path),
            Line::Other(line) => core.log(id, line),
        }
    }

    let status = child.wait().await;
    *core.current.lock().unwrap() = None;

    // If the item was cancelled or paused meanwhile, it is no longer Downloading and these are no-ops.
    let outcome = match status {
        Ok(s) if s.success() => core.update(|q| q.finish(id, file)),
        other => {
            let fallback = match other {
                Ok(s) => format!("yt-dlp exited with {s}"),
                Err(e) => format!("yt-dlp failed: {e}"),
            };
            core.update(|q| {
                let error = q
                    .log(id)
                    .and_then(progress::last_error)
                    .map(str::to_string)
                    .unwrap_or(fallback);
                let hint = progress::looks_like_extractor_error(&error);
                q.fail(id, error, hint)
            })
        }
    };
    if outcome.is_ok() {
        core.notify_if_failed(id);
    }
}

/// Lossy so a stray non-UTF-8 byte in a title can't kill the reader.
async fn forward_lines(stream: impl AsyncRead + Unpin, tx: mpsc::UnboundedSender<String>) {
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&buf).trim_end().to_string();
                if !line.is_empty() && tx.send(line).is_err() {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::settings::{Preset, Settings};
    use crate::ytdlp::args::{self, ItemOptions, Paths};
    use crate::ytdlp::progress::{Line, parse_line};

    /// Runs the real yt-dlp with our arguments and checks our parser understands its output.
    /// Needs network and yt-dlp on PATH: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_ytdlp_output_parses() {
        let dir = std::env::temp_dir().join(format!("laverna-e2e-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("archive.txt");
        let opts = ItemOptions { preset: Preset::Best, subtitles: false, force: false };
        let a = args::download_args(
            "https://download.samplelib.com/mp4/sample-5s.mp4",
            &opts,
            &Settings::default(),
            Paths { output_dir: &dir, archive: &archive, ffmpeg: None },
        )
        .unwrap();

        let run = || {
            let out = std::process::Command::new("yt-dlp").args(&a).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
            // Progress and the file path come on stdout, postprocessor events on stderr; the worker reads both.
            let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
            text.lines().map(parse_line).collect::<Vec<Line>>()
        };

        let first = run();
        assert!(first.iter().any(|l| matches!(l, Line::Progress(p) if p.fraction == Some(1.0))));
        assert!(first.iter().any(|l| matches!(l, Line::Processing(_))));
        let file = first.iter().find_map(|l| match l { Line::File(f) => Some(f.clone()), _ => None }).unwrap();
        assert!(std::path::Path::new(&file).is_file(), "{file}");

        // Second run hits the archive: success, no file line → Skipped.
        let second = run();
        assert!(!second.iter().any(|l| matches!(l, Line::File(_))));

        std::fs::remove_dir_all(dir).ok();
    }
}
