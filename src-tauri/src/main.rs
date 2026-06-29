#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tracing_subscriber;

mod autostart;
mod commands;
mod config;
mod engine;
mod tray;

fn main() {
    tracing_subscriber::fmt::init();

    let config_manager = Arc::new(config::ConfigManager::new());

    // Load initial mapping rules into the engine
    {
        let cfg = config_manager.get_config();
        if let Some(profile) = cfg.profiles.iter().find(|p| p.id == cfg.active_profile_id) {
            engine::mapper::ENGINE.load_rules(profile.mappings.clone());
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(commands::AppState {
            config: config_manager,
        })
        .setup(|app| {
            // Setup system tray
            tray::setup_tray(app.handle())?;

            // Start gamepad polling
            engine::gamepad::start_gamepad_polling();

            // Setup keyboard/mouse hooks on Windows
            #[cfg(windows)]
            unsafe {
                use windows::Win32::System::LibraryLoader::GetModuleHandleW;
                use windows::Win32::UI::WindowsAndMessaging::*;

                let h_instance = GetModuleHandleW(None).unwrap();

                let _kb_hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(engine::hook::keyboard_hook_proc),
                    h_instance,
                    0,
                );

                let _mouse_hook = SetWindowsHookExW(
                    WH_MOUSE_LL,
                    Some(engine::hook::mouse_hook_proc),
                    h_instance,
                    0,
                );

                engine::hook::push_log("info", "输入钩子已安装");
                tracing::info!("Input hooks installed");

                // Start gamepad recording thread
                engine::hook::start_gamepad_recording();
                engine::hook::push_log("info", "手柄监听已启动");
            }

            engine::hook::push_log("info", "KeyMapper 已启动");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::update_settings,
            commands::get_mappings,
            commands::add_mapping,
            commands::remove_mapping,
            commands::update_mapping,
            commands::toggle_mapping,
            commands::import_config_file,
            commands::export_config_file,
            commands::get_devices,
            commands::refresh_devices,
            commands::start_recording,
            commands::stop_recording,
            commands::get_recorded_events,
            commands::clear_recorded_events,
            commands::export_events,
            commands::get_logs,
            commands::clear_logs,
            commands::poll_gamepad_buttons,
            commands::diagnose_gamepad,
            commands::reset_dualsense,
            commands::window_minimize,
            commands::window_maximize,
            commands::window_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
