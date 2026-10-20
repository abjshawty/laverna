// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK's DMA-BUF renderer fails on some GPU/driver combinations (GBM buffer
    // allocation errors, or a Wayland protocol error 71 at startup). Laverna's UI is
    // light, so default to the safe path; set the variable to 0 to opt back in.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: first statement in main, before any other thread exists.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    laverna_lib::run()
}
