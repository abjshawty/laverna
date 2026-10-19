//! Background metadata lookups: titles/thumbnails for the queue, and playlist expansion.

use crate::browsers;
use crate::core::Core;
use crate::queue::{ItemId, Metadata};
use crate::tools;
use crate::ytdlp::{args, metadata};
use std::process::Stdio;
use std::sync::Arc;

pub fn spawn(core: Arc<Core>, id: ItemId) {
    tauri::async_runtime::spawn(async move {
        let Ok(_slot) = core.resolve_slots.acquire().await else { return };
        let Some(url) = core.read(|q| q.get(id).map(|i| i.url.clone())) else { return };
        // A missing Zen profile just means no cookies here; the download reports the error.
        let settings = core.settings();
        let settings = browsers::resolve_cookies(&settings)
            .unwrap_or_else(|_| crate::settings::Settings { cookies_from_browser: None, ..settings });

        let resolved = match tools::resolve("yt-dlp", settings.ytdlp_path.as_deref()) {
            Some(ytdlp) => lookup(&ytdlp, &args::metadata_args(&url, &settings)).await,
            None => None,
        };

        core.update(|q| match resolved {
            Some(metadata::Resolved::Playlist(entries)) => q.expand(id, entries).map(|_| ()),
            Some(metadata::Resolved::Single(meta)) => q.set_meta(id, meta),
            // Let the download itself run and surface the real error.
            None => q.set_meta(id, Metadata { resolved: true, ..Default::default() }),
        })
        .ok();
    });
}

async fn lookup(ytdlp: &std::path::Path, args: &[String]) -> Option<metadata::Resolved> {
    let out = tokio::process::Command::new(ytdlp)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())?;
    let json = serde_json::from_slice(&out.stdout).ok()?;
    Some(metadata::parse(&json))
}
