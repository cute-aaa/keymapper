use crate::config::types::DeviceInfo;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::info;

lazy_static::lazy_static! {
    static ref GAMEPAD_DEVICES: Arc<Mutex<Vec<DeviceInfo>>> = Arc::new(Mutex::new(Vec::new()));
    static ref GAMEPAD_POLLING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

pub fn start_gamepad_polling() {
    let mut polling = GAMEPAD_POLLING.lock();
    *polling = true;
    info!("Gamepad polling started");
    let devices = GAMEPAD_DEVICES.clone();
    std::thread::spawn(move || {
        while *GAMEPAD_POLLING.lock() {
            update_devices(&devices);
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });
}

pub fn get_gamepad_devices() -> Vec<DeviceInfo> {
    GAMEPAD_DEVICES.lock().clone()
}

// ---- WinMM joystick API ----
#[cfg(windows)]
mod winmm {
    #[repr(C)]
    pub struct JOYINFOEX {
        pub dwSize: u32,
        pub dwFlags: u32,
        pub dwXpos: u32,
        pub dwYpos: u32,
        pub dwZpos: u32,
        pub dwRpos: u32,
        pub dwUpos: u32,
        pub dwVpos: u32,
        pub dwButtons: u32,
        pub dwButtonNumber: u32,
        pub dwPOV: u32,
        pub dwReserved1: u32,
        pub dwReserved2: u32,
    }

    #[repr(C)]
    pub struct JOYCAPSW {
        pub wMid: u16,
        pub wPid: u16,
        pub szPname: [u16; 32],
        pub wXmin: u32, pub wXmax: u32,
        pub wYmin: u32, pub wYmax: u32,
        pub wZmin: u32, pub wZmax: u32,
        pub wNumButtons: u32,
        pub wPeriodMin: u32, pub wPeriodMax: u32,
        pub wRmin: u32, pub wRmax: u32,
        pub wUmin: u32, pub wUmax: u32,
        pub wVmin: u32, pub wVmax: u32,
        pub wCaps: u32, pub wMaxAxes: u32, pub wNumAxes: u32, pub wMaxButtons: u32,
        pub szRegKey: [u16; 32],
        pub szOEMVxD: [u16; 260],
    }

    impl Default for JOYCAPSW { fn default() -> Self { unsafe { std::mem::zeroed() } } }
    impl Default for JOYINFOEX { fn default() -> Self { unsafe { std::mem::zeroed() } } }

    extern "system" {
        pub fn joyGetPosEx(uJoyID: u32, pji: *mut JOYINFOEX) -> u32;
        pub fn joyGetDevCapsW(uJoyID: u32, pjc: *mut JOYCAPSW, cbjc: u32) -> u32;
    }

    pub const JOY_RETURNBUTTONS: u32 = 0x80;
    pub const JOY_RETURNPOV: u32 = 0x01;

    /// Poll joystick: returns (buttons, pov, name, vid)
    pub fn poll(joy_id: u32) -> Option<(u32, u32, String, u16)> {
        let mut caps = JOYCAPSW::default();
        let r = unsafe { joyGetDevCapsW(joy_id, &mut caps, std::mem::size_of::<JOYCAPSW>() as u32) };
        if r != 0 { return None; }

        let name_len = caps.szPname.iter().position(|&c| c == 0).unwrap_or(32);
        let raw_name = String::from_utf16_lossy(&caps.szPname[..name_len]);

        let name = match caps.wMid {
            0x054C => "PS5 手柄".to_string(),
            0x045E => "Xbox 手柄".to_string(),
            0x057E => "Nintendo 手柄".to_string(),
            _ => raw_name,
        };

        let mut info = JOYINFOEX::default();
        info.dwSize = std::mem::size_of::<JOYINFOEX>() as u32;
        info.dwFlags = JOY_RETURNBUTTONS | JOY_RETURNPOV;
        let r = unsafe { joyGetPosEx(joy_id, &mut info) };
        if r != 0 { return None; }

        Some((info.dwButtons, info.dwPOV, name, caps.wMid))
    }
}

// ---- Device detection ----

#[cfg(windows)]
fn update_devices(devices: &Arc<Mutex<Vec<DeviceInfo>>>) {
    use windows::Win32::UI::Input::XboxController::*;
    use windows::Win32::UI::Input::*;

    let mut devs = devices.lock();
    devs.clear();

    // Use WinMM for all gamepad detection (works for both Xbox and PS5)
    // This avoids XInput+WinMM duplicate detection
    let mut seen_names = std::collections::HashSet::new();

    for joy_id in 0..16u32 {
        if let Some((_buttons, _pov, name, vid)) = winmm::poll(joy_id) {
            if seen_names.contains(&name) { continue; }
            seen_names.insert(name.clone());

            let dt = match vid {
                0x054C => crate::config::types::DeviceType::Ps5Gamepad,
                _ => crate::config::types::DeviceType::XboxGamepad,
            };

            devs.push(DeviceInfo {
                id: format!("joy_{}", joy_id), name,
                device_type: dt, connected: true, port: None,
            });
        }
    }
}

#[cfg(not(windows))]
fn update_devices(_: &Arc<Mutex<Vec<DeviceInfo>>>) {}

// ---- Button mapping ----

/// W3C Standard Gamepad button names
pub const W3C_NAMES: &[&str] = &[
    "A/×", "B/○", "X/□", "Y/△",     // 0-3
    "LB/L1", "RB/R1", "LT/L2", "RT/R2", // 4-7
    "View/Share", "Menu/Options",          // 8-9
    "L3", "R3",                            // 10-11
    "↑", "↓", "←", "→",                   // 12-15
    "Home/PS",                              // 16
];

/// Convert raw XInput button bitmask to W3C Standard indices
pub fn xinput_to_w3c(raw: u16) -> u32 {
    let mut b: u32 = 0;
    if raw & 0x1000 != 0 { b |= 1 << 0; }  // A → W3C 0
    if raw & 0x2000 != 0 { b |= 1 << 1; }  // B → W3C 1
    if raw & 0x4000 != 0 { b |= 1 << 2; }  // X → W3C 2
    if raw & 0x8000 != 0 { b |= 1 << 3; }  // Y → W3C 3
    if raw & 0x0100 != 0 { b |= 1 << 4; }  // LB → W3C 4
    if raw & 0x0200 != 0 { b |= 1 << 5; }  // RB → W3C 5
    if raw & 0x0020 != 0 { b |= 1 << 8; }  // View → W3C 8
    if raw & 0x0010 != 0 { b |= 1 << 9; }  // Menu → W3C 9
    if raw & 0x0040 != 0 { b |= 1 << 10; } // L3 → W3C 10
    if raw & 0x0080 != 0 { b |= 1 << 11; } // R3 → W3C 11
    if raw & 0x0001 != 0 { b |= 1 << 12; } // Up → W3C 12
    if raw & 0x0002 != 0 { b |= 1 << 13; } // Down → W3C 13
    if raw & 0x0004 != 0 { b |= 1 << 14; } // Left → W3C 14
    if raw & 0x0008 != 0 { b |= 1 << 15; } // Right → W3C 15
    b
}

/// Convert WinMM DualSense button bitmask to W3C Standard indices
/// WinMM order: 0=□, 1=×, 2=○, 3=△, 4=L1, 5=R1, 6=L2, 7=R2, 8=Share, 9=Options, 10=L3, 11=R3, 12=PS
pub fn winmm_to_w3c(dw_buttons: u32, dw_pov: u32) -> u32 {
    let mut b: u32 = 0;
    if dw_buttons & (1 << 1) != 0 { b |= 1 << 0; } // × → W3C 0
    if dw_buttons & (1 << 2) != 0 { b |= 1 << 1; } // ○ → W3C 1
    if dw_buttons & (1 << 0) != 0 { b |= 1 << 2; } // □ → W3C 2
    if dw_buttons & (1 << 3) != 0 { b |= 1 << 3; } // △ → W3C 3
    if dw_buttons & (1 << 4) != 0 { b |= 1 << 4; }
    if dw_buttons & (1 << 5) != 0 { b |= 1 << 5; }
    if dw_buttons & (1 << 6) != 0 { b |= 1 << 6; }
    if dw_buttons & (1 << 7) != 0 { b |= 1 << 7; }
    if dw_buttons & (1 << 8) != 0 { b |= 1 << 8; }
    if dw_buttons & (1 << 9) != 0 { b |= 1 << 9; }
    if dw_buttons & (1 << 10) != 0 { b |= 1 << 10; }
    if dw_buttons & (1 << 11) != 0 { b |= 1 << 11; }
    if dw_buttons & (1 << 12) != 0 { b |= 1 << 16; }
    // D-pad from POV
    if dw_pov != 0xFFFF && dw_pov != 0xFFFFFFFF {
        let deg = dw_pov / 100;
        if deg == 0 || deg <= 22 || deg > 337 { b |= 1 << 12; }
        if deg >= 23 && deg <= 67   { b |= 1 << 12; b |= 1 << 15; }
        if deg >= 68 && deg <= 112  { b |= 1 << 15; }
        if deg >= 113 && deg <= 157 { b |= 1 << 13; b |= 1 << 15; }
        if deg >= 158 && deg <= 202 { b |= 1 << 13; }
        if deg >= 203 && deg <= 247 { b |= 1 << 13; b |= 1 << 14; }
        if deg >= 248 && deg <= 292 { b |= 1 << 14; }
        if deg >= 293 && deg <= 337 { b |= 1 << 12; b |= 1 << 14; }
    }
    b
}

/// Poll all gamepad buttons → Vec<(device_id, w3c_bitmask)>
pub fn poll_all_gamepad_buttons() -> Vec<(String, u32)> {
    let mut result = Vec::new();
    #[cfg(windows)]
    {
        let devices = GAMEPAD_DEVICES.lock();
        for dev in devices.iter() {
            if dev.id.starts_with("joy_") {
                if let Ok(joy_id) = dev.id[4..].parse::<u32>() {
                    if let Some((buttons, pov, _name, _vid)) = winmm::poll(joy_id) {
                        let bitmask = winmm_to_w3c(buttons, pov);
                        result.push((dev.id.clone(), bitmask));
                    }
                }
            }
        }
    }
    result
}

/// Diagnose: test XInput and WinMM for N seconds
pub fn diagnose_gamepad(duration_ms: u64) -> Vec<String> {
    let mut results = Vec::new();
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::XboxController::*;

        results.push("=== XInput Test ===".to_string());
        for i in 0..4u32 {
            let mut s = XINPUT_STATE::default();
            if unsafe { XInputGetState(i, &mut s) } == 0 {
                results.push(format!("  Port {}: connected, buttons=0x{:04X}", i, s.Gamepad.wButtons.0));
            } else {
                results.push(format!("  Port {}: not connected", i));
            }
        }

        results.push("=== WinMM Test ===".to_string());
        for joy_id in 0..4u32 {
            if let Some((buttons, pov, name, vid)) = winmm::poll(joy_id) {
                results.push(format!("  Joy {}: {} (VID=0x{:04X}), buttons=0x{:08X}, pov={}", joy_id, name, vid, buttons, pov));
            }
        }

        results.push(format!("=== Polling for {}ms ===", duration_ms));
        let start = std::time::Instant::now();
        let mut prev_xinput = [0u16; 4];
        let mut prev_joy = [0u32; 16];
        while start.elapsed().as_millis() < duration_ms as u128 {
            // XInput
            for i in 0..4u32 {
                let mut s = XINPUT_STATE::default();
                if unsafe { XInputGetState(i, &mut s) } == 0 {
                    let btns = s.Gamepad.wButtons.0;
                    if btns != prev_xinput[i as usize] {
                        results.push(format!("  XInput[{}] buttons: 0x{:04X} → 0x{:04X} (W3C: {})", i, prev_xinput[i as usize], btns, xinput_to_w3c(btns)));
                        prev_xinput[i as usize] = btns;
                    }
                }
            }
            // WinMM
            for joy_id in 0..16u32 {
                if let Some((buttons, pov, _name, _vid)) = winmm::poll(joy_id) {
                    if buttons != prev_joy[joy_id as usize] {
                        results.push(format!("  Joy[{}] buttons: 0x{:08X} → 0x{:08X} (W3C: {})", joy_id, prev_joy[joy_id as usize], buttons, winmm_to_w3c(buttons, pov)));
                        prev_joy[joy_id as usize] = buttons;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
    results
}
