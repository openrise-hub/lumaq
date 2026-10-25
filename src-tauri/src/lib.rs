pub mod app_state;
pub mod commands;

use app_state::LumaqAppState;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tracing::info;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Starting Lumaq Desktop Application");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(LumaqAppState::new())
        .setup(|app| {
            let handle = app.handle().clone();

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(100));
                loop {
                    interval.tick().await;
                    let state = handle.state::<LumaqAppState>();
                    if state.is_recording.load(std::sync::atomic::Ordering::Relaxed) {
                        let mut stats = state.core.get_stats();
                        let (mic_db, sys_db) = state.audio.get_meter_levels();
                        stats.mic_db = mic_db;
                        stats.system_db = sys_db;

                        if let Some(start) = *state.recording_start_instant.lock() {
                            stats.elapsed_seconds = start.elapsed().as_secs_f64();
                        }

                        let _ = handle.emit("telemetry-tick", &stats);
                    }
                }
            });

            let toggle_item = MenuItem::with_id(app, "toggle", "Show / Hide Lumaq", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&toggle_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("Lumaq Screen Recorder")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_state,
            commands::get_stats,
            commands::get_monitors,
            commands::get_windows,
            commands::get_audio_inputs,
            commands::get_audio_outputs,
            commands::get_hardware_capabilities,
            commands::start_recording,
            commands::pause_recording,
            commands::resume_recording,
            commands::stop_recording,
            commands::open_recording_folder,
            commands::open_recording_file,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Lumaq application");
}
