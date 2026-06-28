import { invoke } from "@tauri-apps/api/core";

// Types
export interface MappingRule {
  id: string;
  name: string;
  is_enabled: boolean;
  priority: number;
  source: InputSource;
  targets: InputAction[];
  conditions: MappingCondition[];
  advanced: MappingAdvanced;
}

export interface InputSource {
  device: string;
  primary_key: number;
  modifiers: number[];
  mode: string;
  axis_threshold?: number;
  direction?: string;
}

export interface InputAction {
  action_type: string;
  output_device: string;
  output_key: number;
  output_modifiers: number[];
  duration_ms: number;
}

export interface MappingCondition {
  process?: string;
  window_title?: string;
}

export interface MappingAdvanced {
  delay_before_ms: number;
  delay_between_ms: number;
  repeat_count: number;
  repeat_interval_ms: number;
  consume_input: boolean;
}

export interface DeviceInfo {
  id: string;
  name: string;
  device_type: string;
  connected: boolean;
  port?: string;
}

export interface RecordedEvent {
  timestamp: string;
  device: string;
  key_name: string;
  key_code: number;
  action: string;
  mapped: boolean;
  mapping_rule?: string;
  delay_ms?: number;
}

export interface AppConfig {
  version: string;
  created_at: string;
  auto_start: boolean;
  start_minimized: boolean;
  minimize_to_tray: boolean;
  active_profile_id: string;
  profiles: Profile[];
}

export interface Profile {
  id: string;
  name: string;
  is_active: boolean;
  mappings: MappingRule[];
  conditions: MappingCondition[];
}

export interface LogEntry {
  time: string;
  level: string;
  message: string;
}

export const api = {
  // Config
  getConfig: () => invoke<AppConfig>("get_config"),
  updateSettings: (settings: Record<string, unknown>) =>
    invoke<AppConfig>("update_settings", { settings }),

  // Mappings
  getMappings: () => invoke<MappingRule[]>("get_mappings"),
  addMapping: (rule: MappingRule) => invoke<MappingRule[]>("add_mapping", { rule }),
  removeMapping: (id: string) => invoke<MappingRule[]>("remove_mapping", { id }),
  updateMapping: (rule: MappingRule) => invoke<MappingRule[]>("update_mapping", { rule }),
  toggleMapping: (id: string, enabled: boolean) =>
    invoke<MappingRule[]>("toggle_mapping", { id, enabled }),

  // Config file
  importConfigFile: (path: string) => invoke<AppConfig>("import_config_file", { path }),
  exportConfigFile: (path: string) => invoke<void>("export_config_file", { path }),

  // Devices
  getDevices: () => invoke<DeviceInfo[]>("get_devices"),
  refreshDevices: () => invoke<DeviceInfo[]>("refresh_devices"),

  // Recording
  startRecording: () => invoke<void>("start_recording"),
  stopRecording: () => invoke<void>("stop_recording"),
  getRecordedEvents: () => invoke<RecordedEvent[]>("get_recorded_events"),
  clearRecordedEvents: () => invoke<void>("clear_recorded_events"),
  exportEvents: (format: string) => invoke<string>("export_events", { format }),

  // Logs
  getLogs: () => invoke<LogEntry[]>("get_logs"),
  clearLogs: () => invoke<void>("clear_logs"),

  // Gamepad
  pollGamepadButtons: () => invoke<[string, number][]>("poll_gamepad_buttons"),
  diagnoseGamepad: (durationMs: number) => invoke<string[]>("diagnose_gamepad", { durationMs }),
};
