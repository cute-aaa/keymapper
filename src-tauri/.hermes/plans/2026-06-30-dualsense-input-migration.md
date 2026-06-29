# KeyMapper: 用 dualsense-input 替换 SDL2 + 直接 HID 读取

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** 将 KeyMapper 的 PS5 DualSense 手柄读取方式从 SDL2 DirectInput + 直接 HID 改为 dualsense-input 库 (hidapi)

**Architecture:** dualsense-input 通过 hidapi 直接读取 HID report，支持全部按键（含 Touchpad、Mute），比 SDL2 方案更简洁。保留 WinMM 用于 Xbox 等其他手柄。

**Tech Stack:** Rust, hidapi (via dualsense-input), WinMM

**公共接口不变:**
- `start_gamepad_polling()` / `get_gamepad_devices()`
- `poll_all_gamepad_buttons()` → `Vec<(String, u32)>`
- `diagnose_gamepad(duration_ms)` → `Vec<String>`
- `dualsense_hid::reset_dualsense()` → `bool`

---

### Task 1: 添加 dualsense-input 依赖，移除 SDL2

**Objective:** 更新 Cargo.toml

**Files:** `G:\Projnew\ai\keymapper\src-tauri\Cargo.toml`

- 添加 `dualsense-input = "0.1"`
- 移除 `sdl2 = "0.38"`

### Task 2: 重写 dualsense_hid 模块为 hidapi 方案

**Objective:** 用 hidapi 替换 SetupDi + ReadFile 的复杂 HID 读取

**Files:** `G:\Projnew\ai\keymapper\src-tauri\src\engine\gamepad.rs`

- 移除整个 `dualsense_hid` 模块（行103-725）
- 用简单的 hidapi polling 替换
- 保持 `poll_dualsense_hid()` 和 `reset_dualsense()` 接口

### Task 3: 替换 poll_all_gamepad_buttons 中的 SDL2 调用

**Objective:** 用 hidapi polling 替换 SDL2 polling

- DualSense: 用 dualsense-input 的 poll() 读取按钮
- Xbox/其他: 保留 WinMM
- 移除 SDL2 初始化和调用

### Task 4: 替换 diagnose_gamepad 中的 SDL2 调用

**Objective:** 诊断函数也改用 hidapi

### Task 5: 移除 sdl2_gamepad 模块

**Objective:** 删除不再需要的整个 SDL2 模块（行964-1073）

### Task 6: 验证编译通过

**Objective:** cargo check 确认无错误
