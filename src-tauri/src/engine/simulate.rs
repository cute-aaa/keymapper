use tracing::info;

#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::*;

pub fn simulate_key_press(vk: u32) {
    #[cfg(windows)]
    unsafe {
        let mut input = INPUT::default();
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki.wVk = VIRTUAL_KEY(vk as u16);
        input.Anonymous.ki.dwFlags = KEYEVENTF_EXTENDEDKEY;
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
    info!("Simulated key press: 0x{:02X}", vk);
}

pub fn simulate_key_release(vk: u32) {
    #[cfg(windows)]
    unsafe {
        let mut input = INPUT::default();
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki.wVk = VIRTUAL_KEY(vk as u16);
        input.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY;
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
    info!("Simulated key release: 0x{:02X}", vk);
}

pub fn simulate_key_click(vk: u32, duration_ms: u32) {
    simulate_key_press(vk);
    if duration_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));
    }
    simulate_key_release(vk);
}

pub fn simulate_mouse_click(button: u32) {
    simulate_mouse_press(button);
    std::thread::sleep(std::time::Duration::from_millis(10));
    simulate_mouse_release(button);
}

pub fn simulate_mouse_press(button: u32) {
    #[cfg(windows)]
    unsafe {
        let flag = match button {
            0x0001 => MOUSEEVENTF_LEFTDOWN,
            0x0002 => MOUSEEVENTF_RIGHTDOWN,
            0x0004 => MOUSEEVENTF_MIDDLEDOWN,
            _ => return,
        };
        let mut input = INPUT::default();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi.dwFlags = flag;
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn simulate_mouse_release(button: u32) {
    #[cfg(windows)]
    unsafe {
        let flag = match button {
            0x0001 => MOUSEEVENTF_LEFTUP,
            0x0002 => MOUSEEVENTF_RIGHTUP,
            0x0004 => MOUSEEVENTF_MIDDLEUP,
            _ => return,
        };
        let mut input = INPUT::default();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi.dwFlags = flag;
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn simulate_mouse_wheel(delta: i32) {
    #[cfg(windows)]
    unsafe {
        let mut input = INPUT::default();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi.mouseData = delta as u32;
        input.Anonymous.mi.dwFlags = MOUSEEVENTF_WHEEL;
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
    info!("Simulated mouse wheel: {}", delta);
}
