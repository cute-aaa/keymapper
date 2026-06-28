use crate::config::types::*;
use crate::config::ConfigManager;
use crate::engine::{hook, gamepad};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub config: Arc<ConfigManager>,
}

fn get_active_mappings(config: &ConfigManager) -> Vec<MappingRule> {
    let cfg = config.get_config();
    cfg.profiles
        .iter()
        .find(|p| p.id == cfg.active_profile_id)
        .map(|p| p.mappings.clone())
        .unwrap_or_default()
}

fn sync_engine(config: &ConfigManager) {
    let mappings = get_active_mappings(config);
    crate::engine::mapper::ENGINE.load_rules(mappings);
}

// ── Window control commands ──

#[tauri::command]
pub fn window_minimize(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.minimize();
    }
}

#[tauri::command]
pub fn window_maximize(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_maximized().unwrap_or(false) {
            let _ = window.unmaximize();
        } else {
            let _ = window.maximize();
        }
    }
}

#[tauri::command]
pub fn window_close(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    }
}

// ── Config commands ──

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config.get_config()
}

#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, settings: serde_json::Value) -> AppConfig {
    state.config.update_config(|cfg| {
        if let Some(auto_start) = settings.get("auto_start").and_then(|v| v.as_bool()) {
            cfg.auto_start = auto_start;
            hook::push_log("info", &format!("开机自启动: {}", if auto_start { "开启" } else { "关闭" }));
            #[cfg(windows)]
            {
                let _ = crate::autostart::set_autostart(auto_start);
            }
        }
        if let Some(start_minimized) = settings.get("start_minimized").and_then(|v| v.as_bool()) {
            cfg.start_minimized = start_minimized;
            hook::push_log("info", &format!("启动最小化: {}", if start_minimized { "开启" } else { "关闭" }));
        }
        if let Some(minimize_to_tray) = settings.get("minimize_to_tray").and_then(|v| v.as_bool()) {
            cfg.minimize_to_tray = minimize_to_tray;
            hook::push_log("info", &format!("关闭时最小化到托盘: {}", if minimize_to_tray { "开启" } else { "关闭" }));
        }
        if let Some(active_profile_id) = settings.get("active_profile_id").and_then(|v| v.as_str()) {
            cfg.active_profile_id = active_profile_id.to_string();
            hook::push_log("info", &format!("切换 Profile: {}", active_profile_id));
        }
    })
}

// ── Mapping commands ──

#[tauri::command]
pub fn get_mappings(state: State<'_, AppState>) -> Vec<MappingRule> {
    get_active_mappings(&state.config)
}

#[tauri::command]
pub fn add_mapping(state: State<'_, AppState>, rule: MappingRule) -> Vec<MappingRule> {
    hook::push_log("info", &format!("添加映射规则: {}", rule.name));
    state.config.update_config(|cfg| {
        if let Some(profile) = cfg.profiles.iter_mut().find(|p| p.id == cfg.active_profile_id) {
            profile.mappings.push(rule);
        }
    });
    sync_engine(&state.config);
    get_active_mappings(&state.config)
}

#[tauri::command]
pub fn remove_mapping(state: State<'_, AppState>, id: String) -> Vec<MappingRule> {
    hook::push_log("info", &format!("删除映射规则: {}", id));
    state.config.update_config(|cfg| {
        if let Some(profile) = cfg.profiles.iter_mut().find(|p| p.id == cfg.active_profile_id) {
            profile.mappings.retain(|r| r.id != id);
        }
    });
    sync_engine(&state.config);
    get_active_mappings(&state.config)
}

#[tauri::command]
pub fn update_mapping(state: State<'_, AppState>, rule: MappingRule) -> Vec<MappingRule> {
    hook::push_log("info", &format!("更新映射规则: {}", rule.name));
    state.config.update_config(|cfg| {
        if let Some(profile) = cfg.profiles.iter_mut().find(|p| p.id == cfg.active_profile_id) {
            if let Some(existing) = profile.mappings.iter_mut().find(|r| r.id == rule.id) {
                *existing = rule;
            }
        }
    });
    sync_engine(&state.config);
    get_active_mappings(&state.config)
}

#[tauri::command]
pub fn toggle_mapping(state: State<'_, AppState>, id: String, enabled: bool) -> Vec<MappingRule> {
    state.config.update_config(|cfg| {
        if let Some(profile) = cfg.profiles.iter_mut().find(|p| p.id == cfg.active_profile_id) {
            if let Some(rule) = profile.mappings.iter_mut().find(|r| r.id == id) {
                rule.is_enabled = enabled;
                hook::push_log("info", &format!("映射规则 '{}' {}", rule.name, if enabled { "启用" } else { "禁用" }));
            }
        }
    });
    sync_engine(&state.config);
    get_active_mappings(&state.config)
}

#[tauri::command]
pub fn import_config_file(state: State<'_, AppState>, path: String) -> Result<AppConfig, String> {
    hook::push_log("info", &format!("导入配置: {}", path));
    state.config.import_from_file(&path)
}

#[tauri::command]
pub fn export_config_file(state: State<'_, AppState>, path: String) -> Result<(), String> {
    hook::push_log("info", &format!("导出配置: {}", path));
    state.config.export_to_file(&path)
}

// ── Device commands ──

#[tauri::command]
pub fn get_devices() -> Vec<DeviceInfo> {
    gamepad::get_gamepad_devices()
}

#[tauri::command]
pub fn refresh_devices() -> Vec<DeviceInfo> {
    hook::push_log("info", "刷新设备列表");
    gamepad::get_gamepad_devices()
}

// ── Recording commands ──

#[tauri::command]
pub fn start_recording() {
    hook::start_recording();
}

#[tauri::command]
pub fn stop_recording() {
    hook::stop_recording();
}

#[tauri::command]
pub fn get_recorded_events() -> Vec<RecordedEvent> {
    hook::get_recorded_events()
}

#[tauri::command]
pub fn clear_recorded_events() {
    hook::clear_recorded_events();
}

#[tauri::command]
pub fn export_events(format: String) -> String {
    let events = hook::get_recorded_events();
    match format.as_str() {
        "json" => serde_json::to_string_pretty(&events).unwrap_or_default(),
        "csv" => {
            let mut csv = "timestamp,device,key_name,key_code,action,mapped,delay_ms\n".to_string();
            for e in &events {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{}\n",
                    e.timestamp, e.device, e.key_name, e.key_code, e.action, e.mapped,
                    e.delay_ms.unwrap_or(0)
                ));
            }
            csv
        }
        _ => serde_json::to_string_pretty(&events).unwrap_or_default(),
    }
}

// ── Log commands ──

#[tauri::command]
pub fn get_logs() -> Vec<LogEntry> {
    hook::get_logs()
}

#[tauri::command]
pub fn poll_gamepad_buttons() -> Vec<(String, u32)> {
    crate::engine::gamepad::poll_all_gamepad_buttons()
}

#[tauri::command]
pub fn diagnose_gamepad(duration_ms: u64) -> Vec<String> {
    crate::engine::gamepad::diagnose_gamepad(duration_ms)
}

#[tauri::command]
pub fn clear_logs() {
    hook::clear_logs();
}
