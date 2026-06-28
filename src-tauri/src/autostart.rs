use tracing::info;

#[cfg(windows)]
pub fn set_autostart(enable: bool) -> Result<(), String> {
    use std::process::Command;

    let exe_path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    if enable {
        // Add to Windows startup registry
        Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "KeyMapper",
                "/t",
                "REG_SZ",
                "/d",
                &exe_path,
                "/f",
            ])
            .output()
            .map_err(|e| e.to_string())?;
        info!("Autostart enabled: {}", exe_path);
    } else {
        // Remove from Windows startup registry
        Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "KeyMapper",
                "/f",
            ])
            .output()
            .map_err(|e| e.to_string())?;
        info!("Autostart disabled");
    }

    Ok(())
}
