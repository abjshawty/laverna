use crate::queue::{ItemId, Queue, Status};
use crate::settings::Settings;
use crate::ytdlp::progress::Progress;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::{Notify, Semaphore};

/// `~/.config/laverna` holds user-editable files; the app data dir holds the queue and archive.
pub struct AppPaths {
    pub settings: PathBuf,
    pub theme: PathBuf,
    pub queue: PathBuf,
    pub archive: PathBuf,
}

/// The yt-dlp process currently downloading.
#[derive(Clone, Copy)]
pub struct Running {
    pub id: ItemId,
    pub pid: u32,
}

#[derive(Clone, Serialize)]
struct ProgressEvent {
    id: ItemId,
    progress: Progress,
}

pub struct Core {
    pub app: AppHandle,
    pub paths: AppPaths,
    pub settings: Mutex<Settings>,
    queue: Mutex<Queue>,
    pub current: Mutex<Option<Running>>,
    /// Wakes the download worker after anything that could let it start an item.
    pub wake: Notify,
    /// Limits concurrent metadata lookups.
    pub resolve_slots: Semaphore,
}

impl Core {
    pub fn new(app: AppHandle, paths: AppPaths) -> Self {
        Self {
            settings: Mutex::new(Settings::load(&paths.settings)),
            queue: Mutex::new(Queue::load(&paths.queue)),
            app,
            paths,
            current: Mutex::new(None),
            wake: Notify::new(),
            resolve_slots: Semaphore::new(2),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// Read-only access; nothing is saved or emitted.
    pub fn read<R>(&self, f: impl FnOnce(&Queue) -> R) -> R {
        f(&self.queue.lock().unwrap())
    }

    /// Mutates the queue, then persists it, tells the UI, and wakes the worker.
    pub fn update<R>(&self, f: impl FnOnce(&mut Queue) -> R) -> R {
        let (result, snapshot) = {
            let mut q = self.queue.lock().unwrap();
            let r = f(&mut q);
            (r, q.clone())
        };
        if let Err(e) = snapshot.save(&self.paths.queue) {
            eprintln!("laverna: failed to save queue: {e}");
        }
        let _ = self.app.emit("queue", &snapshot);
        self.wake.notify_one();
        result
    }

    /// Progress is high-frequency: it is emitted but not persisted.
    pub fn progress(&self, id: ItemId, progress: Progress) {
        let ok = self.queue.lock().unwrap().report_progress(id, progress.clone()).is_ok();
        if ok {
            let _ = self.app.emit("progress", ProgressEvent { id, progress });
        }
    }

    pub fn log(&self, id: ItemId, line: String) {
        let _ = self.queue.lock().unwrap().push_log(id, line);
    }

    pub fn is_downloading(&self) -> bool {
        self.current.lock().unwrap().is_some()
    }

    /// Asks the running yt-dlp (and its ffmpeg children) to stop; force-kills if it lingers.
    /// The caller has already moved the item out of `Downloading`, so the worker won't overwrite it.
    pub fn kill_current(&self) {
        let Some(running) = *self.current.lock().unwrap() else { return };
        signal(running.pid, false);
        let app = self.app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let core = app_core(&app);
            let still = matches!(*core.current.lock().unwrap(), Some(r) if r.pid == running.pid);
            if still {
                signal(running.pid, true);
            }
        });
    }

    pub fn notify(&self, title: &str, body: &str) {
        let _ = self.app.notification().builder().title(title).body(body).show();
    }

    pub fn notify_if_failed(&self, id: ItemId) {
        let failed = self.read(|q| {
            q.get(id).and_then(|i| match &i.status {
                Status::Failed { error, .. } => {
                    Some((i.meta.title.clone().unwrap_or_else(|| i.url.clone()), error.clone()))
                }
                _ => None,
            })
        });
        if let Some((title, error)) = failed {
            self.notify(&format!("Download failed: {title}"), &error);
        }
    }
}

pub fn app_core(app: &AppHandle) -> std::sync::Arc<Core> {
    use tauri::Manager;
    app.state::<std::sync::Arc<Core>>().inner().clone()
}

/// yt-dlp runs in its own process group so ffmpeg children are stopped with it.
/// SIGINT lets yt-dlp exit cleanly, keeping the `.part` file for `--continue`.
#[cfg(unix)]
fn signal(pid: u32, force: bool) {
    let sig = if force { libc::SIGKILL } else { libc::SIGINT };
    unsafe {
        libc::kill(-(pid as i32), sig);
    }
}

#[cfg(windows)]
fn signal(pid: u32, _force: bool) {
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status();
}
