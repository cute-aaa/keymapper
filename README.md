# KeyMapper

Windows 按键映射器 — 键盘/鼠标/手柄映射，支持 PS5 DualSense 直读、事件录制与回放。

## 功能

### 映射引擎
- **键盘**：完整 VK 码 (0x01–0xFF)，含 F1–F24、多媒体键
- **鼠标**：左/右/中/X1/X2 按键，滚轮
- **手柄**：Xbox (XInput) / PS5 (hidapi 直读，含触摸板、静音键)
- **组合键**：源端支持组合键（手柄 L1+× 等），目标端支持修饰键组合（Ctrl+Alt+T）
- **反馈**：映射触发时可选声音提示、手柄震动（XInput/DualSense hidapi）

### 事件录制
- 全局录制键盘/鼠标/手柄事件
- 双通道键盘捕获（系统钩子 + 前端监听）
- 智能合并（按下+释放、连续滚轮）
- 序列管理：保存/加载/导出 JSON/导入
- 回放：可配置次数和速度，高亮当前事件

### 设备管理
- Xbox 手柄：WinMM + XInput，VID 0x045E
- PS5 手柄：hidapi 直连（dualsense-input 库），VID 0x054C
- 按键名称：PS5 用 ×○□△，Xbox 用 ABXY

### 系统集成
- 最小化到系统托盘
- 开机自启（注册表 Run 键）
- 深色/浅色主题
- 无边框窗口 + 自定义标题栏

## 截图

> TODO: 添加截图

## 安装

### 前置要求

- Windows 10/11
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (C++ 桌面开发工作负载)
- [Windows SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-sdk/) (10.0.26100 或更高)
- [Node.js](https://nodejs.org/) (18+)
- [Rust](https://rustup.rs/) (stable)

## 构建

### 开发模式

```bash
# 安装前端依赖
npm install

# 启动开发服务器（自动检测 MSVC/SDK）
tauri-dev.bat
```

### 生产构建

```bash
# 构建（自动检测 MSVC/SDK，嵌入前端）
npx tauri build
```

产物在 `src-tauri/target/release/bundle/` 下：
- `msi/` — MSI 安装包
- `exe` — 独立可执行文件

## 项目结构

```
keymapper/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs              # 入口 + hook 安装
│   │   ├── commands.rs          # Tauri IPC 命令
│   │   ├── engine/
│   │   │   ├── hook.rs          # 键盘/鼠标钩子 + 录制
│   │   │   ├── simulate.rs      # SendInput 封装
│   │   │   ├── gamepad.rs       # WinMM + DualSense hidapi
│   │   │   └── mapper.rs        # 映射匹配
│   │   ├── config/types.rs      # 数据结构
│   │   ├── tray.rs              # 系统托盘
│   │   └── autostart.rs         # 开机自启
│   └── icons/                   # 应用图标
├── src/                          # React 前端
│   ├── App.tsx
│   ├── api.ts
│   ├── pages/
│   │   ├── Mappings.tsx         # 映射规则管理
│   │   ├── Devices.tsx          # 设备展示
│   │   ├── Recorder.tsx         # 事件录制与回放
│   │   ├── Logs.tsx             # 日志
│   │   └── Settings.tsx         # 设置
│   ├── components/
│   │   └── KeyCapture.tsx       # 按键捕获组件
│   └── styles/global.css
├── cargo-build.bat               # Cargo 构建脚本
├── tauri-dev.bat                 # 开发模式脚本
└── README.md
```

## 技术栈

| 层 | 技术 |
|---|------|
| 前端 | React 18 + TypeScript + Vite |
| 后端 | Rust + Tauri 2 |
| Win32 API | windows 0.58（钩子、SendInput、XInput） |
| PS5 手柄 | dualsense-input (hidapi) |
| 手柄检测 | WinMM |
| 序列化 | serde + serde_json |

## 关键技术细节

- **WH_KEYBOARD_LL 超时**：回调中使用 `AtomicBool`/`AtomicU64` 替代 `Mutex`，避免 Windows 静默移除 hook
- **DualSense 震动**：使用 `hid_write`（`HidD_SetOutputReport`），不是 `send_feature_report`
- **本应用键盘捕获**：`WH_KEYBOARD_LL` 无法捕获同应用输入，通过前端 `keydown/keyup` 监听补充
- **手柄 VID 区分**：通过 VID 0x054C (Sony) / 0x045E (Microsoft) 区分 PS5/Xbox

## 依赖

| crate | 用途 |
|-------|------|
| tauri 2 | GUI 框架 |
| windows 0.58 | Win32 API |
| dualsense-input | PS5 hidapi 直读 + 震动 |
| parking_lot | 高性能读写锁 |
| serde / serde_json | 序列化 |
| uuid | 映射规则 ID |
| chrono | 时间戳 |
| tracing | 结构化日志 |

## 许可证

[MIT](LICENSE)
