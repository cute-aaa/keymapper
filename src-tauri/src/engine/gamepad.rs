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

// ---- DualSense HID direct reading (SetupDi + ReadFile) ----
#[cfg(windows)]
pub mod dualsense_hid {
    use windows::core::GUID;
    use windows::Win32::Devices::DeviceAndDriverInstallation::*;
    use windows::Win32::Foundation::*;
    use windows::Win32::Storage::FileSystem::*;
    use windows::Win32::System::IO::*;
    use windows::Win32::System::Threading::*;
    use parking_lot::Mutex;
    use tracing::{debug, warn};

    // DualSense USB identifiers
    const DS_VID: &str = "VID_054C";
    const DS_PID: &str = "PID_0CE6";
    const DS_MI: &str = "MI_00";  // Gamepad interface (not MI_03 motion sensor)

    // HID input report layout for DualSense USB (Report ID 0x01, 64 bytes)
    // Byte 0: Report ID (prepended by HID driver when reading)
    // Bytes 1-4: Stick axes (LX, LY, RX, RY)
    // Bytes 5-6: Trigger analog values (L2, R2)
    // Byte 7 (digitalKeys[0]): face buttons + d-pad
    //   bit 7: Triangle, bit 6: Circle, bit 5: Cross, bit 4: Square
    //   bits 3-0: D-pad (0=Up,1=UpRight,2=Right,...,7=UpLeft,8=Released)
    // Byte 8 (digitalKeys[1]): shoulder/special
    //   bit 5: Options, bit 4: Share, bit 3: R2, bit 2: L2, bit 1: R1, bit 0: L1
    // Byte 9 (digitalKeys[2]): special buttons
    //   bit 4: R3, bit 3: L3, bit 2: Mute, bit 1: Touchpad, bit 0: PS
    const DK0_OFFSET: usize = 7;
    const DK1_OFFSET: usize = 8;
    const DK2_OFFSET: usize = 9;
    const REPORT_SIZE: usize = 64;

    /// Cached HID reader state for a DualSense device
    struct HidReader {
        handle: HANDLE,
        event: HANDLE,
        overlapped: OVERLAPPED,
        buffer: [u8; REPORT_SIZE],
        read_pending: bool,
        last_buttons: u32,
    }

    // Safety: HANDLE is an isize wrapper; we use it only on one thread at a time (guarded by Mutex)
    unsafe impl Send for HidReader {}
    unsafe impl Sync for HidReader {}

    lazy_static::lazy_static! {
        static ref DS_READER: Mutex<Option<HidReader>> = Mutex::new(None);
        static ref DS_NOT_FOUND: Mutex<bool> = Mutex::new(false);
    }

    /// GUID_DEVINTERFACE_HID = {4d1e55b2-f16f-11cf-88cb-001111000030}
    fn guid_devinterface_hid() -> GUID {
        GUID {
            data1: 0x4D1E55B2,
            data2: 0xF16F,
            data3: 0x11CF,
            data4: [0x88, 0xCB, 0x00, 0x11, 0x11, 0x00, 0x00, 0x30],
        }
    }

    // CM_Get_Device_Interface_List from cfgmgr32.dll — finds all registered
    // device interfaces including ones that SetupDi misses (e.g. DualSense MI_00).
    type CONFIGRET = u32;
    const CR_SUCCESS: CONFIGRET = 0;
    const CM_GET_DEVICE_INTERFACE_LIST_PRESENT: u32 = 0;      // only currently 'live' interfaces
    const CM_GET_DEVICE_INTERFACE_LIST_ALL_DEVICES: u32 = 1;  // all registered, live or not

    #[link(name = "cfgmgr32")]
    extern "system" {
        fn CM_Get_Device_Interface_List_SizeW(
            pul_len: *mut u32,
            interface_class_guid: *const GUID,
            p_device_id: windows::core::PCWSTR,
            ul_flags: u32,
        ) -> CONFIGRET;

        fn CM_Get_Device_Interface_ListW(
            interface_class_guid: *const GUID,
            p_device_id: windows::core::PCWSTR,
            buffer: windows::core::PWSTR,
            buffer_len: u32,
            ul_flags: u32,
        ) -> CONFIGRET;
    }

    /// Parse a double-null-terminated multi-string (UTF-16) into individual strings.
    fn parse_multi_string(buf: &[u16]) -> Vec<String> {
        let mut result = Vec::new();
        let mut start = 0;
        for i in 0..buf.len() {
            if buf[i] == 0 {
                if i == start {
                    break; // double null = end of list
                }
                let s = String::from_utf16_lossy(&buf[start..i]);
                result.push(s);
                start = i + 1;
            }
        }
        result
    }

    /// Find the DualSense MI_00 HID device path via CM_Get_Device_Interface_List.
    /// This enumerates ALL registered HID device interfaces, including ones claimed
    /// by the HID driver that SetupDi misses (like DualSense MI_00).
    fn find_dualsense_device_path() -> Option<String> {
        unsafe {
            let hid_guid = guid_devinterface_hid();
            let pcwstr_null = windows::core::PCWSTR::null();

            // --- Pass 1: Use CM_Get_Device_Interface_List with PRESENT flag ---
            let mut buf_len: u32 = 0;
            let cr = CM_Get_Device_Interface_List_SizeW(
                &mut buf_len,
                &hid_guid,
                pcwstr_null,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            );
            if cr != CR_SUCCESS || buf_len == 0 {
                warn!("CM_Get_Device_Interface_List_SizeW failed: CR={}", cr);
                return None;
            }

            // buf_len is the number of characters (including multi-null terminator)
            // Allocate wide-char buffer
            let mut wide_buf: Vec<u16> = vec![0u16; buf_len as usize];
            let cr = CM_Get_Device_Interface_ListW(
                &hid_guid,
                pcwstr_null,
                windows::core::PWSTR(wide_buf.as_mut_ptr()),
                buf_len,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            );
            if cr != CR_SUCCESS {
                warn!("CM_Get_Device_Interface_ListW (present) failed: CR={}", cr);
                return None;
            }

            let paths = parse_multi_string(&wide_buf);
            debug!("CM (present) found {} HID interfaces", paths.len());

            // Log all Sony devices found for debugging
            for p in &paths {
                let upper = p.to_uppercase();
                if upper.contains("054C") || upper.contains("0CE6") {
                    tracing::info!("CM (present) Sony device: {}", p);
                }
            }

            // Look for DualSense MI_00
            for p in &paths {
                let upper = p.to_uppercase();
                if upper.contains(DS_VID) && upper.contains(DS_PID) && upper.contains(DS_MI) {
                    tracing::info!("Found DualSense MI_00 via CM (present): {}", p);
                    return Some(p.clone());
                }
            }

            // If MI_00 not found, try the composite parent (no MI_ suffix)
            for p in &paths {
                let upper = p.to_uppercase();
                if upper.contains(DS_VID) && upper.contains(DS_PID) && !upper.contains("&MI_") {
                    tracing::info!("Trying DualSense composite parent (no MI): {}", p);
                    let test_wide: Vec<u16> = p.encode_utf16().chain(std::iter::once(0)).collect();
                    let test_handle = CreateFileW(
                        windows::core::PCWSTR(test_wide.as_ptr()),
                        GENERIC_READ.0,
                        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                        None,
                        OPEN_EXISTING,
                        FILE_ATTRIBUTE_NORMAL,
                        None,
                    );
                    if let Ok(h) = test_handle {
                        tracing::info!("Successfully opened DualSense composite parent!");
                        let _ = CloseHandle(h);
                        return Some(p.clone());
                    }
                }
            }

            // --- Pass 2: Enumerate ALL interfaces (not just present) ---
            // Sometimes the device is registered but not flagged "present" yet.
            debug!("MI_00 not found among present interfaces, trying ALL...");
            buf_len = 0;
            let cr = CM_Get_Device_Interface_List_SizeW(
                &mut buf_len,
                &hid_guid,
                pcwstr_null,
                CM_GET_DEVICE_INTERFACE_LIST_ALL_DEVICES,
            );
            if cr == CR_SUCCESS && buf_len > 0 {
                let mut wide_buf2: Vec<u16> = vec![0u16; buf_len as usize];
                let cr = CM_Get_Device_Interface_ListW(
                    &hid_guid,
                    pcwstr_null,
                    windows::core::PWSTR(wide_buf2.as_mut_ptr()),
                    buf_len,
                    CM_GET_DEVICE_INTERFACE_LIST_ALL_DEVICES,
                );
                if cr == CR_SUCCESS {
                    let paths2 = parse_multi_string(&wide_buf2);
                    debug!("CM (all) found {} HID interfaces", paths2.len());

                    // Log all Sony devices
                    for p in &paths2 {
                        let upper = p.to_uppercase();
                        if upper.contains("054C") || upper.contains("0CE6") {
                            tracing::info!("CM (all) Sony device: {}", p);
                        }
                    }

                    // Look for DualSense MI_00
                    for p in &paths2 {
                        let upper = p.to_uppercase();
                        if upper.contains(DS_VID) && upper.contains(DS_PID) && upper.contains(DS_MI) {
                            tracing::info!("Found DualSense MI_00 via CM (all): {}", p);
                            return Some(p.clone());
                        }
                    }

                    // If MI_00 still not found, check if any DualSense interface exists at all
                    // (MI_03 etc.) and log a warning
                    for p in &paths2 {
                        let upper = p.to_uppercase();
                        if upper.contains(DS_VID) && upper.contains(DS_PID) {
                            tracing::warn!("DualSense found but not MI_00: {}", p);
                        }
                    }
                }
            }

            // --- Pass 3: Fallback to SetupDi for backwards compatibility ---
            // In case CM found nothing at all (unlikely but defensive)
            debug!("CM enumeration did not find MI_00, falling back to SetupDi...");
            let dev_info = match SetupDiGetClassDevsW(
                Some(&hid_guid),
                windows::core::PCWSTR::null(),
                HWND::default(),
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            ) {
                Ok(h) => h,
                Err(e) => {
                    warn!("SetupDiGetClassDevsW failed: {}", e);
                    return None;
                }
            };

            let mut result: Option<String> = None;
            let mut index: u32 = 0;

            loop {
                let mut iface_data: SP_DEVICE_INTERFACE_DATA = std::mem::zeroed();
                iface_data.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;

                if SetupDiEnumDeviceInterfaces(
                    dev_info,
                    None,
                    &hid_guid,
                    index,
                    &mut iface_data,
                ).is_err() {
                    break;
                }

                index += 1;

                let mut detail_size: u32 = 0;
                let _ = SetupDiGetDeviceInterfaceDetailW(
                    dev_info, &iface_data, None, 0, Some(&mut detail_size), None,
                );
                if detail_size == 0 { continue; }

                let layout = match std::alloc::Layout::from_size_align(detail_size as usize, 4) {
                    Ok(l) => l,
                    Err(_) => continue,
                };
                let detail_buf = std::alloc::alloc(layout) as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
                if detail_buf.is_null() { continue; }
                (*detail_buf).cbSize = 8;

                let ok = SetupDiGetDeviceInterfaceDetailW(
                    dev_info, &iface_data, Some(detail_buf), detail_size, Some(&mut detail_size), None,
                );
                if ok.is_err() {
                    std::alloc::dealloc(detail_buf as *mut u8, layout);
                    continue;
                }

                let path_ptr = (*detail_buf).DevicePath.as_ptr();
                let mut len = 0usize;
                while len < 512 {
                    if *path_ptr.add(len) == 0 { break; }
                    len += 1;
                }
                let path_wide = std::slice::from_raw_parts(path_ptr, len);
                let path = String::from_utf16_lossy(path_wide);

                let path_upper = path.to_uppercase();
                if path_upper.contains(DS_VID) && path_upper.contains(DS_PID) {
                    if path_upper.contains(DS_MI) {
                        tracing::info!("Found DualSense MI_00 via SetupDi fallback: {}", path);
                        result = Some(path);
                        std::alloc::dealloc(detail_buf as *mut u8, layout);
                        break;
                    }
                    if path_upper.contains("MI_03") {
                        tracing::info!("Skipping MI_03 (motion sensor, no button data)");
                    }
                }

                std::alloc::dealloc(detail_buf as *mut u8, layout);
            }

            let _ = SetupDiDestroyDeviceInfoList(dev_info);
            result
        }
    }

    /// Open the DualSense HID device and create an event for overlapped I/O
    fn open_dualsense_hid(path: &str) -> Option<HidReader> {
        unsafe {
            // Convert path to wide string for CreateFileW
            let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

            // Try GENERIC_READ | GENERIC_WRITE first, fall back to read-only
            let desired_access: u32 = GENERIC_READ.0 | GENERIC_WRITE.0;
            let share_mode = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
            let flags = FILE_FLAG_OVERLAPPED | FILE_ATTRIBUTE_NORMAL;

            let handle = CreateFileW(
                windows::core::PCWSTR(path_wide.as_ptr()),
                desired_access,
                share_mode,
                None,
                OPEN_EXISTING,
                flags,
                None,
            );

            let handle = match handle {
                Ok(h) => h,
                Err(_) => {
                    // Try read-only access
                    match CreateFileW(
                        windows::core::PCWSTR(path_wide.as_ptr()),
                        GENERIC_READ.0,
                        share_mode,
                        None,
                        OPEN_EXISTING,
                        flags,
                        None,
                    ) {
                        Ok(h) => h,
                        Err(e) => {
                            warn!("Failed to open DualSense HID device: {}", e);
                            return None;
                        }
                    }
                }
            };

            let event = match CreateEventW(None, true, false, windows::core::w!("")) {
                Ok(e) => e,
                Err(e) => {
                    warn!("CreateEventW failed: {}", e);
                    let _ = CloseHandle(handle);
                    return None;
                }
            };

            let mut overlapped: OVERLAPPED = std::mem::zeroed();
            overlapped.hEvent = event;

            Some(HidReader {
                handle,
                event,
                overlapped,
                buffer: [0u8; REPORT_SIZE],
                read_pending: false,
                last_buttons: 0,
            })
        }
    }

    /// Start an overlapped ReadFile on the HID device.
    /// Returns true if the read completed immediately (data available).
    fn start_read(reader: &mut HidReader) -> bool {
        unsafe {
            let _ = ResetEvent(reader.event);
            reader.buffer = [0u8; REPORT_SIZE];
            let result = ReadFile(
                reader.handle,
                Some(&mut reader.buffer),
                None,
                Some(&mut reader.overlapped),
            );
            if result.is_ok() {
                // Completed immediately
                reader.read_pending = false;
                true
            } else {
                reader.read_pending = true;
                false
            }
        }
    }

    /// Check if a pending read has completed and retrieve data.
    /// Returns true if data is available in reader.buffer.
    fn try_complete_read(reader: &mut HidReader, timeout_ms: u32) -> bool {
        unsafe {
            if !reader.read_pending {
                // No pending read - data was already available from start_read
                return true;
            }
            let wait = WaitForSingleObject(reader.event, timeout_ms);
            if wait == WAIT_OBJECT_0 {
                let mut bytes_read: u32 = 0;
                let ok = GetOverlappedResult(
                    reader.handle,
                    &reader.overlapped,
                    &mut bytes_read,
                    false,
                );
                reader.read_pending = false;
                ok.is_ok() && bytes_read as usize >= DK2_OFFSET + 1
            } else {
                false
            }
        }
    }

    /// Parse DualSense digital keys bytes into a W3C-compatible bitmask.
    /// Standard W3C buttons (0-16) use the same indices as winmm_to_w3c.
    /// Extended buttons: 17=Touchpad, 18=Mute, 19=L2 Digital, 20=R2 Digital.
    fn parse_dualsense_report(buffer: &[u8; REPORT_SIZE]) -> u32 {
        let dk0 = buffer[DK0_OFFSET]; // Face buttons + d-pad
        let dk1 = buffer[DK1_OFFSET]; // Shoulder/special
        let dk2 = buffer[DK2_OFFSET]; // L3, R3, Mute, Touchpad, PS

        let mut b: u32 = 0;

        // Face buttons (dk0)
        if dk0 & (1 << 7) != 0 { b |= 1 << 3; } // Triangle → W3C 3 (Y/△)
        if dk0 & (1 << 6) != 0 { b |= 1 << 1; } // Circle → W3C 1 (B/○)
        if dk0 & (1 << 5) != 0 { b |= 1 << 0; } // Cross → W3C 0 (A/×)
        if dk0 & (1 << 4) != 0 { b |= 1 << 2; } // Square → W3C 2 (X/□)

        // D-pad (dk0 bits 3-0): 0=Up,1=UpRight,2=Right,3=DownRight,4=Down,5=DownLeft,6=Left,7=UpLeft,8+=Released
        let dpad = dk0 & 0x0F;
        match dpad {
            0 => { b |= 1 << 12; }              // Up
            1 => { b |= 1 << 12; b |= 1 << 15; } // Up+Right
            2 => { b |= 1 << 15; }              // Right
            3 => { b |= 1 << 13; b |= 1 << 15; } // Down+Right
            4 => { b |= 1 << 13; }              // Down
            5 => { b |= 1 << 13; b |= 1 << 14; } // Down+Left
            6 => { b |= 1 << 14; }              // Left
            7 => { b |= 1 << 12; b |= 1 << 14; } // Up+Left
            _ => {}                              // 8 or other = released
        }

        // Shoulder/special buttons (dk1)
        if dk1 & (1 << 0) != 0 { b |= 1 << 4; }  // L1 → W3C 4
        if dk1 & (1 << 1) != 0 { b |= 1 << 5; }  // R1 → W3C 5
        if dk1 & (1 << 2) != 0 { b |= 1 << 19; } // L2 (digital) → W3C 19
        if dk1 & (1 << 3) != 0 { b |= 1 << 20; } // R2 (digital) → W3C 20
        if dk1 & (1 << 4) != 0 { b |= 1 << 8; }  // Share → W3C 8
        if dk1 & (1 << 5) != 0 { b |= 1 << 9; }  // Options → W3C 9

        // Special buttons (dk2)
        if dk2 & (1 << 0) != 0 { b |= 1 << 16; } // PS → W3C 16
        if dk2 & (1 << 1) != 0 { b |= 1 << 17; } // Touchpad → W3C 17
        if dk2 & (1 << 2) != 0 { b |= 1 << 18; } // Mute → W3C 18
        if dk2 & (1 << 3) != 0 { b |= 1 << 10; } // L3 → W3C 10
        if dk2 & (1 << 4) != 0 { b |= 1 << 11; } // R3 → W3C 11

        b
    }

    /// Poll the DualSense via direct HID reading.
    /// Returns a W3C bitmask including all buttons: standard (0-16) plus
    /// touchpad (17), mute (18), L2 digital (19), R2 digital (20).
    pub fn poll_dualsense_hid() -> Option<u32> {
        // Skip if we already know the device isn't available
        if *DS_NOT_FOUND.lock() {
            return None;
        }

        let mut reader_guard = DS_READER.lock();

        // Lazy initialization: find and open device if not connected
        if reader_guard.is_none() {
            match find_dualsense_device_path() {
                Some(path) => {
                    match open_dualsense_hid(&path) {
                        Some(reader) => {
                            *reader_guard = Some(reader);
                            if let Some(r) = reader_guard.as_mut() {
                                start_read(r);
                            }
                        }
                        None => {
                            *DS_NOT_FOUND.lock() = true;
                            return None;
                        }
                    }
                }
                None => {
                    *DS_NOT_FOUND.lock() = true;
                    return None;
                }
            }
        }

        let reader = reader_guard.as_mut().unwrap();

        // Try to complete pending read (0ms timeout = non-blocking check)
        if try_complete_read(reader, 0) {
            reader.last_buttons = parse_dualsense_report(&reader.buffer);
            // Start next read immediately
            start_read(reader);
        } else if reader.read_pending {
            // Try once more with a short timeout to reduce latency
            if try_complete_read(reader, 2) {
                reader.last_buttons = parse_dualsense_report(&reader.buffer);
                start_read(reader);
            }
        }

        Some(reader.last_buttons)
    }

    /// Disconnect the cached HID reader (e.g. on device removal)
    pub fn disconnect_dualsense_hid() {
        let mut reader_guard = DS_READER.lock();
        if let Some(reader) = reader_guard.take() {
            unsafe {
                if reader.read_pending {
                    let _ = CancelIo(reader.handle);
                    WaitForSingleObject(reader.event, 100);
                }
                let _ = CloseHandle(reader.event);
                let _ = CloseHandle(reader.handle);
            }
        }
    }

    /// Reset DualSense controller state (triggers, vibration, LED).
    /// Sends an output report that clears any configuration set by other apps.
    pub fn reset_dualsense() -> bool {
        let path = match find_dualsense_device_path() {
            Some(p) => p,
            None => return false,
        };

        let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            // Open with write access
            let handle = match CreateFileW(
                windows::core::PCWSTR(path_wide.as_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | FILE_ATTRIBUTE_NORMAL,
                None,
            ) {
                Ok(h) => h,
                Err(_) => return false,
            };

            // Build output report (USB report ID 0x02, 64 bytes)
            // Based on DualSense Tester website output struct
            let mut report = [0u8; 64];
            report[0] = 0x02; // Report ID

            // validFlag0: enable vibration (bits 0,1) and triggers (bits 2,3)
            report[1] = 0x0F; // bits 0-3

            // bcVibrationRight=0, bcVibrationLeft=0 (bytes 2,3)
            report[2] = 0;
            report[3] = 0;

            // adaptiveTriggerRightMode=0 (byte 10) — off
            report[10] = 0;
            // adaptiveTriggerRightParam0-9 = 0 (bytes 11-20) — already 0

            // adaptiveTriggerLeftMode=0 (byte 21) — off
            report[21] = 0;
            // adaptiveTriggerLeftParam0-9 = 0 (bytes 22-31) — already 0

            // Write the report
            let mut bytes_written: u32 = 0;
            let mut overlapped: OVERLAPPED = std::mem::zeroed();
            let event = match CreateEventW(None, true, false, windows::core::w!("")) {
                Ok(e) => e,
                Err(_) => { let _ = CloseHandle(handle); return false; }
            };
            overlapped.hEvent = event;

            let ok = WriteFile(
                handle,
                Some(&report),
                Some(&mut bytes_written),
                Some(&mut overlapped),
            );

            if ok.is_err() {
                // Wait for async completion
                WaitForSingleObject(event, 1000);
            }

            let _ = CloseHandle(event);
            let _ = CloseHandle(handle);
            true
        }
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

/// W3C Standard Gamepad button names (indices 0-20)
pub const W3C_NAMES: &[&str] = &[
    "A/×", "B/○", "X/□", "Y/△",     // 0-3
    "LB/L1", "RB/R1", "LT/L2", "RT/R2", // 4-7
    "View/Share", "Menu/Options",          // 8-9
    "L3", "R3",                            // 10-11
    "↑", "↓", "←", "→",                   // 12-15
    "Home/PS",                             // 16
    "Touchpad",                            // 17
    "Mute",                                // 18
    "L2 Digital",                          // 19
    "R2 Digital",                          // 20
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
///
/// For PS5 DualSense devices, this merges WinMM data (standard buttons + d-pad)
/// with direct HID reading (touchpad, mute, PS, L3, R3, L2/R2 digital).
/// When a DualSense is detected, HID is used as the authoritative source since
/// it reports ALL buttons including ones WinMM doesn't expose.
pub fn poll_all_gamepad_buttons() -> Vec<(String, u32)> {
    let mut result = Vec::new();
    #[cfg(windows)]
    {
        let devices = GAMEPAD_DEVICES.lock();
        let mut has_dualsense = false;

        for dev in devices.iter() {
            if dev.id.starts_with("joy_") {
                if let Ok(joy_id) = dev.id[4..].parse::<u32>() {
                    if let Some((buttons, pov, _name, vid)) = winmm::poll(joy_id) {
                        let bitmask;

                        // For DualSense devices, use HID as the authoritative source
                        // since it reports all buttons including touchpad, mute, PS, L3, R3
                        if vid == 0x054C {
                            has_dualsense = true;
                            bitmask = match dualsense_hid::poll_dualsense_hid() {
                                Some(hid_buttons) => hid_buttons,
                                None => winmm_to_w3c(buttons, pov), // Fallback to WinMM
                            };
                        } else {
                            bitmask = winmm_to_w3c(buttons, pov);
                        }

                        result.push((dev.id.clone(), bitmask));
                    }
                }
            }
        }

        // If no DualSense found via WinMM but one is connected via HID,
        // add it as a standalone device (SetupDi finds it independently)
        if !has_dualsense {
            if let Some(hid_buttons) = dualsense_hid::poll_dualsense_hid() {
                result.push(("dualsense_hid".to_string(), hid_buttons));
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

        results.push("=== DualSense HID Test ===".to_string());
        match dualsense_hid::poll_dualsense_hid() {
            Some(btns) => results.push(format!("  DualSense HID: buttons=0x{:08X}", btns)),
            None => results.push("  DualSense HID: not found or failed to open".to_string()),
        }

        results.push(format!("=== Polling for {}ms ===", duration_ms));
        let start = std::time::Instant::now();
        let mut prev_xinput = [0u16; 4];
        let mut prev_joy = [0u32; 16];
        let mut prev_hid: u32 = 0;
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
            // DualSense HID
            if let Some(hid_btns) = dualsense_hid::poll_dualsense_hid() {
                if hid_btns != prev_hid {
                    results.push(format!("  DualSense HID: buttons=0x{:08X}", hid_btns));
                    prev_hid = hid_btns;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
    results
}
