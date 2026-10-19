use crate::browsers;
use crate::core::Core;
use crate::queue::{ItemId, Queue, QueueError};
use crate::resolver;
use crate::settings::Settings;
use crate::theme;
use crate::tools::{self, ToolInfo};
use crate::ytdlp::args::ItemOptions;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tauri::State;

type Res<T = ()> = Result<T, String>;

fn err(e: QueueError) -> String {
    match e {
        QueueError::NotFound(id) => format!("item {id} not found"),
        QueueError::InvalidState(id) => format!("item {id} can't do that right now"),
    }
}

#[derive(Serialize)]
pub struct Tools {
    ytdlp: ToolInfo,
    ffmpeg: ToolInfo,
}

#[tauri::command]
pub fn get_settings(core: State<Arc<Core>>) -> Settings {
    core.settings()
}

#[tauri::command]
pub fn save_settings(core: State<Arc<Core>>, settings: Settings) -> Res {
    shell_words::split(&settings.extra_args).map_err(|e| format!("Extra arguments: {e}"))?;
    settings.save(&core.paths.settings).map_err(|e| e.to_string())?;
    *core.settings.lock().unwrap() = settings;
    // A newly configured yt-dlp path may unblock the worker and pending lookups.
    core.wake.notify_one();
    for id in core.read(|q| q.unresolved()) {
        resolver::spawn(core.inner().clone(), id);
    }
    Ok(())
}

#[tauri::command]
pub async fn detect_tools(core: State<'_, Arc<Core>>) -> Res<Tools> {
    let s = core.settings();
    Ok(Tools {
        ytdlp: tools::detect("yt-dlp", s.ytdlp_path.as_deref(), "--version").await,
        ffmpeg: tools::detect("ffmpeg", s.ffmpeg_path.as_deref(), "-version").await,
    })
}

/// The user's theme overrides; empty if the file doesn't exist yet.
#[tauri::command]
pub fn read_theme(core: State<Arc<Core>>) -> String {
    std::fs::read_to_string(&core.paths.theme).unwrap_or_default()
}

#[tauri::command]
pub fn get_theme_vars(core: State<Arc<Core>>) -> BTreeMap<String, String> {
    theme::editor_vars(&std::fs::read_to_string(&core.paths.theme).unwrap_or_default())
}

/// Replaces the editor's variables (empty map = reset to preset). Hand-written CSS is kept.
#[tauri::command]
pub fn set_theme_vars(core: State<Arc<Core>>, vars: BTreeMap<String, String>) -> Res<String> {
    let current = std::fs::read_to_string(&core.paths.theme).unwrap_or_default();
    let css = theme::with_editor_vars(&current, &vars).map_err(|e| e.to_string())?;
    if let Some(dir) = core.paths.theme.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&core.paths.theme, &css).map_err(|e| e.to_string())?;
    Ok(css)
}

/// What `--cookies-from-browser` will receive for `browser`, so Settings can show it.
#[tauri::command]
pub fn cookies_source(browser: String) -> Res<String> {
    browsers::cookies_spec(&browser)
}

#[tauri::command]
pub fn get_queue(core: State<Arc<Core>>) -> Queue {
    core.read(|q| q.clone())
}

/// Splits on whitespace so several pasted links become several items.
#[tauri::command]
pub fn add_urls(core: State<Arc<Core>>, urls: String, options: ItemOptions) -> Vec<ItemId> {
    let ids: Vec<ItemId> = core.update(|q| {
        urls.split_whitespace()
            .map(|u| q.add(u.to_string(), options.clone()))
            .collect()
    });
    for id in &ids {
        resolver::spawn(core.inner().clone(), *id);
    }
    ids
}

#[tauri::command]
pub fn remove_item(core: State<Arc<Core>>, id: ItemId) -> Res {
    core.update(|q| q.remove(id).map(|_| ())).map_err(err)
}

#[tauri::command]
pub fn move_item(core: State<Arc<Core>>, id: ItemId, index: usize) -> Res {
    core.update(|q| q.move_to(id, index)).map_err(err)
}

#[tauri::command]
pub fn set_item_options(core: State<Arc<Core>>, id: ItemId, options: ItemOptions) -> Res {
    core.update(|q| q.set_options(id, options)).map_err(err)
}

#[tauri::command]
pub fn retry_item(core: State<Arc<Core>>, id: ItemId) -> Res {
    core.update(|q| q.retry(id)).map_err(err)
}

#[tauri::command]
pub fn cancel_current(core: State<Arc<Core>>) -> Res {
    let Some(running) = *core.current.lock().unwrap() else { return Ok(()) };
    core.update(|q| q.cancel(running.id)).map_err(err)?;
    core.kill_current();
    Ok(())
}

#[tauri::command]
pub fn pause_queue(core: State<Arc<Core>>) {
    if core.update(|q| q.pause()).is_some() {
        core.kill_current();
    }
}

#[tauri::command]
pub fn resume_queue(core: State<Arc<Core>>) {
    core.update(|q| q.resume());
}

#[tauri::command]
pub fn clear_finished(core: State<Arc<Core>>) {
    core.update(|q| q.clear_finished());
}

#[tauri::command]
pub fn get_log(core: State<Arc<Core>>, id: ItemId) -> Vec<String> {
    core.read(|q| q.log(id).map(<[String]>::to_vec).unwrap_or_default())
}
