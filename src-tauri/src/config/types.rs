use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Keyboard,
    Mouse,
    XboxGamepad,
    Ps5Gamepad,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode {
    Press,
    Release,
    Hold,
    Tap,
    DoubleTap,
    Chord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    KeyPress,
    KeyRelease,
    KeyClick,
    MouseMove,
    MouseWheel,
    GamepadButton,
    Delay,
    Repeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSource {
    pub device: DeviceType,
    pub primary_key: u32,
    #[serde(default)]
    pub modifiers: Vec<u32>,
    pub mode: TriggerMode,
    #[serde(default)]
    pub axis_threshold: Option<f32>,
    #[serde(default)]
    pub direction: Option<String>,
    /// Additional keys that must be pressed together with primary_key (combo mode)
    #[serde(default)]
    pub combo_keys: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAction {
    pub action_type: ActionType,
    pub output_device: DeviceType,
    pub output_key: u32,
    #[serde(default)]
    pub output_modifiers: Vec<u32>,
    #[serde(default)]
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingAdvanced {
    #[serde(default)]
    pub delay_before_ms: u32,
    #[serde(default)]
    pub delay_between_ms: u32,
    #[serde(default)]
    pub repeat_count: i32,
    #[serde(default)]
    pub repeat_interval_ms: u32,
    #[serde(default)]
    pub consume_input: bool,
}

impl Default for MappingAdvanced {
    fn default() -> Self {
        Self {
            delay_before_ms: 0,
            delay_between_ms: 0,
            repeat_count: 0,
            repeat_interval_ms: 0,
            consume_input: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingCondition {
    #[serde(default)]
    pub process: Option<String>,
    #[serde(default)]
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub is_enabled: bool,
    #[serde(default)]
    pub priority: i32,
    pub source: InputSource,
    pub targets: Vec<InputAction>,
    #[serde(default)]
    pub conditions: Vec<MappingCondition>,
    #[serde(default)]
    pub advanced: MappingAdvanced,
    /// Play a sound when this mapping triggers
    #[serde(default)]
    pub sound_feedback: bool,
    /// Vibrate gamepad when this mapping triggers (gamepad source only)
    #[serde(default)]
    pub vibration_feedback: bool,
    /// Vibration intensity 0-255 (default 128)
    #[serde(default = "default_vibration_intensity")]
    pub vibration_intensity: u8,
    /// Vibration duration in ms (default 200)
    #[serde(default = "default_vibration_duration")]
    pub vibration_duration_ms: u32,
}

fn default_true() -> bool {
    true
}

fn default_vibration_intensity() -> u8 {
    128
}

fn default_vibration_duration() -> u32 {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub mappings: Vec<MappingRule>,
    #[serde(default)]
    pub conditions: Vec<MappingCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: String,
    pub created_at: String,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default)]
    pub minimize_to_tray: bool,
    #[serde(default)]
    pub active_profile_id: String,
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_profile = Profile {
            id: uuid::Uuid::new_v4().to_string(),
            name: "默认".to_string(),
            is_active: true,
            mappings: Vec::new(),
            conditions: Vec::new(),
        };
        let active_id = default_profile.id.clone();
        Self {
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            auto_start: false,
            start_minimized: false,
            minimize_to_tray: true,
            active_profile_id: active_id,
            profiles: vec![default_profile],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub connected: bool,
    pub port: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub timestamp: String,
    pub device: String,
    pub key_name: String,
    pub key_code: u32,
    pub action: String,
    pub mapped: bool,
    pub mapping_rule: Option<String>,
    pub delay_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub message: String,
}
