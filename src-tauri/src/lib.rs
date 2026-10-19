mod browsers;
mod commands;
mod core;
mod queue;
mod resolver;
mod settings;
mod theme;
mod tools;
mod worker;
mod ytdlp;

use crate::core::{AppPaths, Core, app_core};
use std::sync::Arc;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config = app.path().config_dir()?.join("laverna");
            let data = app.path().app_data_dir()?;
            let paths = AppPaths {
                settings: config.join("settings.json"),
                theme: config.join("theme.css"),
                queue: data.join("queue.json"),
                archive: data.join("archive.txt"),
            };
            let core = Arc::new(Core::new(app.handle().clone(), paths));
            app.manage(core.clone());

            for id in core.read(|q| q.unresolved()) {
                resolver::spawn(core.clone(), id);
            }
            tauri::async_runtime::spawn(theme::watch(core.clone()));
            tauri::async_runtime::spawn(worker::run(core));
            Ok(())
        })
        .on_window_event(|window, event| {
            let WindowEvent::CloseRequested { api, .. } = event else { return };
            let core = app_core(window.app_handle());
            if !core.is_downloading() {
                return;
            }
            api.prevent_close();
            let app = window.app_handle().clone();
            window
                .dialog()
                .message("A download is in progress. It will resume next time you open Laverna.")
                .title("Quit Laverna?")
                .buttons(MessageDialogButtons::OkCancelCustom("Quit".into(), "Keep downloading".into()))
                .show(move |quit| {
                    if quit {
                        app.exit(0);
                    }
                });
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::detect_tools,
            commands::read_theme,
            commands::get_theme_vars,
            commands::cookies_source,
            commands::set_theme_vars,
            commands::get_queue,
            commands::add_urls,
            commands::remove_item,
            commands::move_item,
            commands::set_item_options,
            commands::retry_item,
            commands::cancel_current,
            commands::pause_queue,
            commands::resume_queue,
            commands::clear_finished,
            commands::get_log,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // yt-dlp runs in its own process group, so it would outlive us unless stopped.
            // The saved queue still has it as downloading; next launch resumes it.
            if let RunEvent::Exit = event {
                app_core(app).kill_current();
            }
        });
}
