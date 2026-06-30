use tracing::info;

#[cfg(windows)]
pub fn set_autostart(enable: bool) -> Result<(), String> {
    use windows::Win32::System::Registry::{
        HKEY_CURRENT_USER, KEY_SET_VALUE, KEY_READ,
        RegOpenKeyExW, RegSetValueExW, RegDeleteValueW, RegCloseKey, REG_SZ,
    };
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::core::w;

    let exe_path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    let key_path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let value_name = w!("KeyMapper");

    unsafe {
        let mut hkey = Default::default();
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path,
            0,
            KEY_SET_VALUE | KEY_READ,
            &mut hkey,
        );

        if result != ERROR_SUCCESS {
            return Err(format!("Failed to open registry key: {:?}", result));
        }

        if enable {
            let exe_wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();
            let data = std::slice::from_raw_parts(
                exe_wide.as_ptr() as *const u8,
                exe_wide.len() * 2,
            );
            let result = RegSetValueExW(hkey, value_name, 0, REG_SZ, Some(data));
            RegCloseKey(hkey).ok();
            if result != ERROR_SUCCESS {
                return Err(format!("Failed to set registry value: {:?}", result));
            }
            info!("Autostart enabled: {}", exe_path);
        } else {
            let result = RegDeleteValueW(hkey, value_name);
            RegCloseKey(hkey).ok();
            if result != ERROR_SUCCESS {
                return Err(format!("Failed to delete registry value: {:?}", result));
            }
            info!("Autostart disabled");
        }
    }

    Ok(())
}
