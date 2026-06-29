use crate::config::types::{DeviceType, InputSource, LogEntry, MappingRule, RecordedEvent, TriggerMode};
use crate::engine::mapper::ENGINE;
use crate::engine::simulate;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::info;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Play a short beep sound as mapping feedback
pub fn play_sound_feedback() {
    unsafe {
        // MessageBeep from Win32::System::Diagnostics::Debug
        use windows::Win32::System::Diagnostics::Debug::MessageBeep;
        use windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE;
        let _ = MessageBeep(MESSAGEBOX_STYLE(0x00000040)); // MB_ICONASTERISK
    }
}

/// Trigger gamepad vibration. Tries DualSense hidapi first, then XInput.
pub fn trigger_vibration(intensity: u16, duration_ms: u32) {
    std::thread::spawn(move || {
        let rumble_val = (intensity >> 8) as u8; // Convert 0-65535 to 0-255

        // Try DualSense hidapi rumble first
        let ds_rumbled = crate::engine::gamepad::dualsense_hid::set_dualsense_rumble(rumble_val, rumble_val);

        // Also try XInput for Xbox controllers
        #[cfg(windows)]
        {
            use windows::Win32::UI::Input::XboxController::*;
            let vibration = XINPUT_VIBRATION {
                wLeftMotorSpeed: intensity,
                wRightMotorSpeed: intensity,
            };
            for port in 0..4u32 {
                unsafe { let _ = XInputSetState(port, &vibration); }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));

        // Stop vibration
        if ds_rumbled {
            crate::engine::gamepad::dualsense_hid::stop_dualsense_rumble();
        }
        #[cfg(windows)]
        {
            use windows::Win32::UI::Input::XboxController::*;
            let stop = XINPUT_VIBRATION { wLeftMotorSpeed: 0, wRightMotorSpeed: 0 };
            for port in 0..4u32 {
                unsafe { let _ = XInputSetState(port, &stop); }
            }
        }
    });
}

lazy_static::lazy_static! {
    static ref RECORDED_EVENTS: Arc<Mutex<Vec<RecordedEvent>>> = Arc::new(Mutex::new(Vec::new()));
    static ref IS_RECORDING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref LAST_EVENT_TIME: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    static ref LOGS: Arc<Mutex<Vec<LogEntry>>> = Arc::new(Mutex::new(Vec::new()));
    static ref PRESSED_KEYS: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
}

pub fn start_recording() {
    let mut recording = IS_RECORDING.lock();
    *recording = true;
    push_log("info", "Recording started");
    info!("Recording started");
}

pub fn stop_recording() {
    let mut recording = IS_RECORDING.lock();
    *recording = false;
    push_log("info", "Recording stopped");
    info!("Recording stopped");
}

pub fn is_recording() -> bool { *IS_RECORDING.lock() }
pub fn get_recorded_events() -> Vec<RecordedEvent> { RECORDED_EVENTS.lock().clone() }
pub fn clear_recorded_events() { RECORDED_EVENTS.lock().clear(); }
pub fn get_logs() -> Vec<LogEntry> { LOGS.lock().clone() }
pub fn clear_logs() { LOGS.lock().clear(); }

pub fn push_log(level: &str, message: &str) {
    let mut logs = LOGS.lock();
    logs.push(LogEntry {
        time: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
        level: level.to_string(),
        message: message.to_string(),
    });
    if logs.len() > 5000 { logs.remove(0); }
}

fn record_event(event: RecordedEvent) {
    if is_recording() {
        let mut events = RECORDED_EVENTS.lock();
        events.push(event);
        if events.len() > 10000 { events.remove(0); }
    }
}

fn vk_to_name(vk: u16) -> String {
    match vk {
        0x08 => "Backspace".to_string(), 0x09 => "Tab".to_string(), 0x0D => "Enter".to_string(),
        0x10 => "Shift".to_string(), 0x11 => "Ctrl".to_string(), 0x12 => "Alt".to_string(),
        0x13 => "Pause".to_string(), 0x14 => "CapsLock".to_string(), 0x1B => "Escape".to_string(),
        0x20 => "Space".to_string(), 0x21 => "PageUp".to_string(), 0x22 => "PageDown".to_string(),
        0x23 => "End".to_string(), 0x24 => "Home".to_string(), 0x25 => "Left".to_string(),
        0x26 => "Up".to_string(), 0x27 => "Right".to_string(), 0x28 => "Down".to_string(),
        0x2C => "PrintScreen".to_string(), 0x2D => "Insert".to_string(), 0x2E => "Delete".to_string(),
        0x5B => "Win".to_string(), 0x5C => "Win".to_string(),
        0x60 => "Num0".to_string(), 0x61 => "Num1".to_string(), 0x62 => "Num2".to_string(),
        0x63 => "Num3".to_string(), 0x64 => "Num4".to_string(), 0x65 => "Num5".to_string(),
        0x66 => "Num6".to_string(), 0x67 => "Num7".to_string(), 0x68 => "Num8".to_string(),
        0x69 => "Num9".to_string(), 0x6A => "Num*".to_string(), 0x6B => "Num+".to_string(),
        0x6D => "Num-".to_string(), 0x6E => "Num.".to_string(), 0x6F => "Num/".to_string(),
        0x70..=0x7B => format!("F{}", vk - 0x6F),
        0x90 => "NumLock".to_string(), 0x91 => "ScrollLock".to_string(),
        0xA0 => "LShift".to_string(), 0xA1 => "RShift".to_string(),
        0xA2 => "LCtrl".to_string(), 0xA3 => "RCtrl".to_string(),
        0xA4 => "LAlt".to_string(), 0xA5 => "RAlt".to_string(),
        0xAD => "VolumeMute".to_string(), 0xAE => "VolumeDown".to_string(), 0xAF => "VolumeUp".to_string(),
        0xB0 => "MediaNext".to_string(), 0xB1 => "MediaPrev".to_string(),
        0xB2 => "MediaStop".to_string(), 0xB3 => "MediaPlay".to_string(),
        _ => {
            if (0x30..=0x39).contains(&vk) { char::from_u32(vk as u32).unwrap_or('?').to_string() }
            else if (0x41..=0x5A).contains(&vk) { char::from_u32(vk as u32).unwrap_or('?').to_string() }
            else { format!("VK_0x{:02X}", vk) }
        }
    }
}

pub fn mouse_button_name(button: u16) -> String {
    match button {
        0x0001 => "Mouse Left".to_string(), 0x0002 => "Mouse Right".to_string(),
        0x0004 => "Mouse Middle".to_string(), 0x0005 => "XButton1".to_string(),
        0x0006 => "XButton2".to_string(), 0x0007 => "WheelUp".to_string(),
        0x0008 => "WheelDown".to_string(), 0x0009 => "WheelLeft".to_string(),
        0x000A => "WheelRight".to_string(),
        _ => format!("Mouse_0x{:04X}", button),
    }
}

pub fn gamepad_button_name(idx: u16, is_ps: bool) -> String {
    if is_ps {
        // PS5 button names
        match idx {
            0 => "×", 1 => "○", 2 => "□", 3 => "△",
            4 => "L1", 5 => "R1", 6 => "L2", 7 => "R2",
            8 => "Share", 9 => "Options",
            10 => "L3", 11 => "R3",
            12 => "↑", 13 => "↓", 14 => "←", 15 => "→",
            16 => "PS",
            17 => "触摸板",
            18 => "静音",
            19 => "L2数字",
            20 => "R2数字",
            _ => return format!("按键{}", idx),
        }.to_string()
    } else {
        // Xbox button names
        match idx {
            0 => "A", 1 => "B", 2 => "X", 3 => "Y",
            4 => "LB", 5 => "RB", 6 => "LT", 7 => "RT",
            8 => "View", 9 => "Menu",
            10 => "L3", 11 => "R3",
            12 => "↑", 13 => "↓", 14 => "←", 15 => "→",
            16 => "Xbox",
            17 => "Touchpad",
            18 => "Mute",
            19 => "LT Digital",
            20 => "RT Digital",
            _ => return format!("按键{}", idx),
        }.to_string()
    }
}

unsafe fn get_current_modifiers() -> Vec<u32> {
    let mut mods = Vec::new();
    if (GetKeyState(VK_SHIFT.0 as i32) & 0x8000u16 as i16) != 0 { mods.push(0x10); }
    if (GetKeyState(VK_CONTROL.0 as i32) & 0x8000u16 as i16) != 0 { mods.push(0x11); }
    if (GetKeyState(VK_MENU.0 as i32) & 0x8000u16 as i16) != 0 { mods.push(0x12); }
    mods
}

#[cfg(windows)]
pub unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk = kb.vkCode as u16;
        let action = match wparam.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => "Press",
            WM_KEYUP | WM_SYSKEYUP => "Release",
            _ => "Unknown",
        };

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        let mut last = LAST_EVENT_TIME.lock();
        let delay = if *last == 0 { 0 } else { now - *last };
        *last = now;

        let trigger_mode = if action == "Press" { TriggerMode::Press } else { TriggerMode::Release };
        let modifiers = get_current_modifiers();
        let source = InputSource {
            device: DeviceType::Keyboard, primary_key: vk as u32, modifiers: modifiers.clone(),
            mode: trigger_mode, axis_threshold: None, direction: None, combo_keys: Vec::new(),
        };

        let matches = ENGINE.find_matching_rules(&source, 0);
        let mut consumed = false;

        if !matches.is_empty() {
            for rule in &matches {
                if ENGINE.is_exempt(vk as u32) { continue; }
                push_log("info", &format!("Rule matched: {} ({} -> {})", rule.name, vk_to_name(vk),
                    rule.targets.iter().map(|t| format!("0x{:02X}", t.output_key)).collect::<Vec<_>>().join(", ")));

                // Feedback
                if rule.sound_feedback { play_sound_feedback(); }

                for target in &rule.targets {
                    if rule.advanced.delay_before_ms > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(rule.advanced.delay_before_ms as u64));
                    }
                    // Press modifiers first
                    for mod_vk in &target.output_modifiers {
                        ENGINE.add_exempt(*mod_vk);
                        simulate::simulate_key_press(*mod_vk);
                    }
                    match target.action_type {
                        crate::config::types::ActionType::KeyClick => { ENGINE.add_exempt(target.output_key); simulate::simulate_key_click(target.output_key, target.duration_ms); }
                        crate::config::types::ActionType::KeyPress => { ENGINE.add_exempt(target.output_key); simulate::simulate_key_press(target.output_key); }
                        crate::config::types::ActionType::KeyRelease => { ENGINE.add_exempt(target.output_key); simulate::simulate_key_release(target.output_key); }
                        crate::config::types::ActionType::MouseWheel => { simulate::simulate_mouse_wheel(target.output_key as i32); }
                        _ => {}
                    }
                    // Release modifiers after
                    for mod_vk in &target.output_modifiers {
                        simulate::simulate_key_release(*mod_vk);
                    }
                }
                if rule.advanced.consume_input { consumed = true; }
            }
        }

        record_event(RecordedEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            device: "Keyboard".to_string(), key_name: vk_to_name(vk), key_code: vk as u32,
            action: action.to_string(), mapped: !matches.is_empty(),
            mapping_rule: matches.first().map(|r| r.name.clone()), delay_ms: Some(delay),
        });

        if action == "Press" {
            let mut pressed = PRESSED_KEYS.lock();
            if !pressed.contains(&(vk as u32)) { pressed.push(vk as u32); }
        } else {
            PRESSED_KEYS.lock().retain(|&k| k != vk as u32);
        }

        if consumed { return LRESULT(1); }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(windows)]
pub unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let ms = *(lparam.0 as *const MSLLHOOKSTRUCT);
        let (action, button) = match wparam.0 as u32 {
            WM_LBUTTONDOWN => ("Press", 0x0001u16), WM_LBUTTONUP => ("Release", 0x0001),
            WM_RBUTTONDOWN => ("Press", 0x0002), WM_RBUTTONUP => ("Release", 0x0002),
            WM_MBUTTONDOWN => ("Press", 0x0004), WM_MBUTTONUP => ("Release", 0x0004),
            WM_XBUTTONDOWN => { let x = ((ms.mouseData >> 16) & 0xFFFF) as u16; ("Press", if x == 1 { 0x0005 } else { 0x0006 }) }
            WM_XBUTTONUP => { let x = ((ms.mouseData >> 16) & 0xFFFF) as u16; ("Release", if x == 1 { 0x0005 } else { 0x0006 }) }
            WM_MOUSEWHEEL => { let d = ((ms.mouseData >> 16) & 0xFFFF) as i16; ("Press", if d > 0 { 0x0007 } else { 0x0008 }) }
            WM_MOUSEHWHEEL => { let d = ((ms.mouseData >> 16) & 0xFFFF) as i16; ("Press", if d > 0 { 0x0009 } else { 0x000A }) }
            _ => ("Unknown", 0u16),
        };

        if button != 0 {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
            let mut last = LAST_EVENT_TIME.lock();
            let delay = if *last == 0 { 0 } else { now - *last };
            *last = now;

            let trigger_mode = if action == "Press" { TriggerMode::Press } else { TriggerMode::Release };
            let source = InputSource {
                device: DeviceType::Mouse, primary_key: button as u32, modifiers: Vec::new(),
                mode: trigger_mode, axis_threshold: None, direction: None, combo_keys: Vec::new(),
            };
            let matches = ENGINE.find_matching_rules(&source, 0);
            let mut consumed = false;

            if !matches.is_empty() {
                for rule in &matches {
                    push_log("info", &format!("Mouse rule matched: {}", rule.name));
                    for target in &rule.targets {
                        if rule.advanced.delay_before_ms > 0 {
                            std::thread::sleep(std::time::Duration::from_millis(rule.advanced.delay_before_ms as u64));
                        }
                        for mod_vk in &target.output_modifiers {
                            ENGINE.add_exempt(*mod_vk);
                            simulate::simulate_key_press(*mod_vk);
                        }
                        match target.action_type {
                            crate::config::types::ActionType::KeyClick => { ENGINE.add_exempt(target.output_key); simulate::simulate_key_click(target.output_key, target.duration_ms); }
                            crate::config::types::ActionType::KeyPress => { ENGINE.add_exempt(target.output_key); simulate::simulate_key_press(target.output_key); }
                            crate::config::types::ActionType::KeyRelease => { ENGINE.add_exempt(target.output_key); simulate::simulate_key_release(target.output_key); }
                            _ => {}
                        }
                        for mod_vk in &target.output_modifiers {
                            simulate::simulate_key_release(*mod_vk);
                        }
                    }
                    if rule.advanced.consume_input { consumed = true; }
                }
            }

            record_event(RecordedEvent {
                timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
                device: "Mouse".to_string(), key_name: mouse_button_name(button), key_code: button as u32,
                action: action.to_string(), mapped: !matches.is_empty(),
                mapping_rule: matches.first().map(|r| r.name.clone()), delay_ms: Some(delay),
            });

            if consumed { return LRESULT(1); }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(windows)]
pub fn start_gamepad_recording() {
    use crate::engine::gamepad::poll_all_gamepad_buttons;
    std::thread::spawn(move || {
        let mut prev_buttons: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        loop {
            let all_buttons = poll_all_gamepad_buttons();
            for (dev_id, buttons, vid) in &all_buttons {
                let prev = prev_buttons.get(dev_id).copied().unwrap_or(0);
                let changed = *buttons ^ prev;
                if changed != 0 {
                    for bit in 0..21u32 {
                        let mask = 1u32 << bit;
                        if changed & mask != 0 {
                            let pressed = buttons & mask != 0;
                            let action = if pressed { "Press" } else { "Release" };
                            let is_ps = *vid == 0x054C;
                            let device_name = if is_ps { "PS5" } else { "Xbox" }.to_string();
                            let name = gamepad_button_name(bit as u16, is_ps);
                            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
                            let mut last = LAST_EVENT_TIME.lock();
                            let delay = if *last == 0 { 0 } else { now.saturating_sub(*last) };
                            *last = now;

                            // Rule matching for gamepad
                            if pressed {
                                let trigger_mode = TriggerMode::Press;
                                let device_type = if is_ps { DeviceType::Ps5Gamepad } else { DeviceType::XboxGamepad };
                                let source = InputSource {
                                    device: device_type, primary_key: bit, modifiers: Vec::new(),
                                    mode: trigger_mode, axis_threshold: None, direction: None, combo_keys: Vec::new(),
                                };
                                let matches = ENGINE.find_matching_rules(&source, *buttons);
                                for rule in &matches {
                                    push_log("info", &format!("Gamepad rule matched: {} ({} -> {})", rule.name, name,
                                        rule.targets.iter().map(|t| format!("0x{:02X}", t.output_key)).collect::<Vec<_>>().join(", ")));

                                    // Sound feedback
                                    if rule.sound_feedback { play_sound_feedback(); }

                                    // Vibration feedback (gamepad source only)
                                    if rule.vibration_feedback {
                                        trigger_vibration(rule.vibration_intensity as u16 * 257, rule.vibration_duration_ms);
                                    }

                                    for target in &rule.targets {
                                        if rule.advanced.delay_before_ms > 0 {
                                            std::thread::sleep(std::time::Duration::from_millis(rule.advanced.delay_before_ms as u64));
                                        }
                                        for mod_vk in &target.output_modifiers {
                                            ENGINE.add_exempt(*mod_vk);
                                            simulate::simulate_key_press(*mod_vk);
                                        }
                                        match target.action_type {
                                            crate::config::types::ActionType::KeyClick => { ENGINE.add_exempt(target.output_key); simulate::simulate_key_click(target.output_key, target.duration_ms); }
                                            crate::config::types::ActionType::KeyPress => { ENGINE.add_exempt(target.output_key); simulate::simulate_key_press(target.output_key); }
                                            crate::config::types::ActionType::KeyRelease => { ENGINE.add_exempt(target.output_key); simulate::simulate_key_release(target.output_key); }
                                            crate::config::types::ActionType::MouseWheel => { simulate::simulate_mouse_wheel(target.output_key as i32); }
                                            _ => {}
                                        }
                                        for mod_vk in &target.output_modifiers {
                                            simulate::simulate_key_release(*mod_vk);
                                        }
                                    }
                                }
                            }

                            if is_recording() {
                                record_event(RecordedEvent {
                                    timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
                                    device: device_name,
                                    key_name: name, key_code: bit,
                                    action: action.to_string(), mapped: false,
                                    mapping_rule: None, delay_ms: Some(delay),
                                });
                            }
                        }
                    }
                }
                prev_buttons.insert(dev_id.clone(), *buttons);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    });
}
