import { useEffect, useRef, useState, useCallback } from "react";
import { api } from "../api";

interface KeyCaptureProps {
  value: { primary_key: number; modifiers: number[]; combo_keys?: number[] };
  onChange: (keys: { primary_key: number; modifiers: number[]; combo_keys?: number[] }) => void;
  device?: string;
  allowCombo?: boolean;
}

const VK_NAMES: Record<number, string> = {
  0x08: "Backspace", 0x09: "Tab", 0x0D: "Enter", 0x10: "Shift",
  0x11: "Ctrl", 0x12: "Alt", 0x13: "Pause", 0x14: "CapsLock",
  0x1B: "Esc", 0x20: "Space", 0x21: "PageUp", 0x22: "PageDown",
  0x23: "End", 0x24: "Home", 0x25: "←", 0x26: "↑",
  0x27: "→", 0x28: "↓", 0x2C: "PrtSc", 0x2D: "Insert",
  0x2E: "Delete", 0x5B: "Win", 0x5C: "Win", 0x5D: "Menu",
  0x60: "Num0", 0x61: "Num1", 0x62: "Num2", 0x63: "Num3",
  0x64: "Num4", 0x65: "Num5", 0x66: "Num6", 0x67: "Num7",
  0x68: "Num8", 0x69: "Num9", 0x6A: "Num*", 0x6B: "Num+",
  0x6C: "NumEnter", 0x6D: "Num-", 0x6E: "Num.", 0x6F: "Num/",
  0x70: "F1", 0x71: "F2", 0x72: "F3", 0x73: "F4",
  0x74: "F5", 0x75: "F6", 0x76: "F7", 0x77: "F8",
  0x78: "F9", 0x79: "F10", 0x7A: "F11", 0x7B: "F12",
  0x90: "NumLock", 0x91: "ScrLk",
  0xA0: "LShift", 0xA1: "RShift", 0xA2: "LCtrl", 0xA3: "RCtrl",
  0xA4: "LAlt", 0xA5: "RAlt",
  0xAD: "静音", 0xAE: "音量-", 0xAF: "音量+",
  0xB0: "下一曲", 0xB1: "上一曲", 0xB2: "停止", 0xB3: "播放",
  0xBA: ";", 0xBB: "=", 0xBC: ",", 0xBD: "-",
  0xBE: ".", 0xBF: "/", 0xC0: "`", 0xDB: "[",
  0xDC: "\\", 0xDD: "]", 0xDE: "'",
};

const MOUSE_NAMES: Record<number, string> = {
  1: "左键", 2: "右键", 4: "中键", 5: "侧键1", 6: "侧键2",
};

const XBOX_NAMES: Record<number, string> = {
  0: "A", 1: "B", 2: "X", 3: "Y",
  4: "LB", 5: "RB", 6: "LT", 7: "RT",
  8: "View", 9: "Menu",
  10: "L3", 11: "R3",
  12: "↑", 13: "↓", 14: "←", 15: "→",
  16: "Xbox",
};

const PS5_NAMES: Record<number, string> = {
  0: "×", 1: "○", 2: "□", 3: "△",
  4: "L1", 5: "R1", 6: "L2", 7: "R2",
  8: "Share", 9: "Options",
  10: "L3", 11: "R3",
  12: "↑", 13: "↓", 14: "←", 15: "→",
  16: "PS", 17: "触摸板", 18: "静音",
};

export function vkToName(vk: number): string {
  if (VK_NAMES[vk]) return VK_NAMES[vk];
  if (vk >= 0x30 && vk <= 0x39) return String.fromCharCode(vk);
  if (vk >= 0x41 && vk <= 0x5A) return String.fromCharCode(vk);
  return `键${vk}`;
}

export function buttonToName(code: number, device: string): string {
  if (device === "mouse") return MOUSE_NAMES[code] || `鼠标${code}`;
  if (device === "ps5_gamepad") return PS5_NAMES[code] || `按键${code}`;
  if (device === "xbox_gamepad") return XBOX_NAMES[code] || `按键${code}`;
  return `按键${code}`;
}

const MODIFIER_VK = new Set([0x10, 0x11, 0x12, 0x5B, 0x5C, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5]);

export default function KeyCapture({ value, onChange, device = "keyboard", allowCombo = false }: KeyCaptureProps) {
  const [capturing, setCapturing] = useState(false);
  const [heldKeys, setHeldKeys] = useState<Set<number>>(new Set());
  const heldRef = useRef<Set<number>>(new Set());
  const capturingRef = useRef(false);
  const onChangeRef = useRef(onChange);
  const timerRef = useRef<number | undefined>(undefined);
  const prevRef = useRef<Set<number>>(new Set());

  useEffect(() => { capturingRef.current = capturing; }, [capturing]);
  useEffect(() => { onChangeRef.current = onChange; }, [onChange]);

  const cancelCapture = useCallback(() => {
    setCapturing(false);
    setHeldKeys(new Set());
    heldRef.current = new Set();
    prevRef.current.clear();
    if (timerRef.current) { cancelAnimationFrame(timerRef.current); timerRef.current = undefined; }
  }, []);

  // Global Escape
  useEffect(() => {
    const onEsc = (e: KeyboardEvent) => {
      if (e.keyCode === 27 && capturingRef.current) { e.preventDefault(); e.stopPropagation(); cancelCapture(); }
    };
    window.addEventListener("keydown", onEsc, true);
    return () => window.removeEventListener("keydown", onEsc, true);
  }, [cancelCapture]);

  // Keyboard handlers
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    e.preventDefault(); e.stopPropagation();
    if (e.keyCode === 27) return;
    heldRef.current.add(e.keyCode);
    setHeldKeys(new Set(heldRef.current));
  }, []);

  const handleKeyUp = useCallback((e: KeyboardEvent) => {
    e.preventDefault(); e.stopPropagation();
    if (e.keyCode === 27) return;
    if (!MODIFIER_VK.has(e.keyCode)) {
      onChangeRef.current({ primary_key: e.keyCode, modifiers: [...heldRef.current].filter(k => MODIFIER_VK.has(k)) });
      cancelCapture(); return;
    }
    heldRef.current.delete(e.keyCode);
    setHeldKeys(new Set(heldRef.current));
  }, [cancelCapture]);

  // Mouse handler
  const handleMouseDown = useCallback((e: MouseEvent) => {
    e.preventDefault(); e.stopPropagation();
    const map: Record<number, number> = { 0: 1, 1: 4, 2: 2, 3: 5, 4: 6 };
    onChangeRef.current({ primary_key: map[e.button] || e.button + 1, modifiers: [] });
    cancelCapture();
  }, [cancelCapture]);

  // Gamepad polling — try browser API first, fallback to Rust backend
  const heldGamepadRef = useRef<Set<number>>(new Set());
  const pressOrderRef = useRef<number[]>([]);

  const startGamepadPolling = useCallback(() => {
    prevRef.current.clear();
    heldGamepadRef.current.clear();
    pressOrderRef.current = [];
    let settled = false;
    let hadPress = false; // Track if any button was pressed during this capture

    const poll = async () => {
      if (settled) return;

      const currentHeld = new Set<number>();

      // Try browser Gamepad API first
      try {
        const gamepads = navigator.getGamepads();
        for (const gp of gamepads) {
          if (!gp) continue;
          for (let i = 0; i < gp.buttons.length; i++) {
            if (gp.buttons[i].pressed) currentHeld.add(i);
          }
        }
      } catch { /* API not available */ }

      // Fallback: Rust backend
      if (currentHeld.size === 0) {
        try {
          const result = await api.pollGamepadButtons();
          for (const [, buttons] of result) {
            for (let bit = 0; bit < 21; bit++) {
              if ((buttons & (1 << bit)) !== 0) currentHeld.add(bit);
            }
          }
        } catch { /* ignore */ }
      }

      // Track press order
      for (const btn of currentHeld) {
        if (!heldGamepadRef.current.has(btn)) {
          pressOrderRef.current.push(btn);
        }
      }
      heldGamepadRef.current = currentHeld;

      if (currentHeld.size > 0) {
        hadPress = true;
      }

      if (allowCombo) {
        // Combo mode: wait for all buttons to be released after at least one press
        if (hadPress && currentHeld.size === 0 && pressOrderRef.current.length > 0) {
          settled = true;
          const allPressed = [...pressOrderRef.current];
          if (allPressed.length === 1) {
            // Single key
            onChangeRef.current({ primary_key: allPressed[0], modifiers: [], combo_keys: [] });
          } else {
            // Combo: last pressed is primary, rest are combo
            const primary = allPressed[allPressed.length - 1];
            const combo = allPressed.slice(0, -1);
            onChangeRef.current({ primary_key: primary, modifiers: [], combo_keys: combo });
          }
          cancelCapture(); return;
        }
      } else {
        // Single mode: capture first pressed button immediately
        for (const btn of currentHeld) {
          if (!prevRef.current.has(btn)) {
            settled = true;
            onChangeRef.current({ primary_key: btn, modifiers: [], combo_keys: [] });
            cancelCapture(); return;
          }
        }
      }

      prevRef.current = currentHeld;
      timerRef.current = requestAnimationFrame(poll);
    };
    timerRef.current = requestAnimationFrame(poll);
  }, [cancelCapture]);

  // Start/stop capture
  useEffect(() => {
    if (!capturing) return;
    if (device === "keyboard") {
      window.addEventListener("keydown", handleKeyDown, true);
      window.addEventListener("keyup", handleKeyUp, true);
      return () => { window.removeEventListener("keydown", handleKeyDown, true); window.removeEventListener("keyup", handleKeyUp, true); };
    }
    if (device === "mouse") {
      window.addEventListener("mousedown", handleMouseDown, true);
      const prevent = (e: Event) => e.preventDefault();
      window.addEventListener("contextmenu", prevent, true);
      return () => { window.removeEventListener("mousedown", handleMouseDown, true); window.removeEventListener("contextmenu", prevent, true); };
    }
    if (device === "xbox_gamepad" || device === "ps5_gamepad") {
      startGamepadPolling();
      return () => { if (timerRef.current) { cancelAnimationFrame(timerRef.current); timerRef.current = undefined; } };
    }
  }, [capturing, device, handleKeyDown, handleKeyUp, handleMouseDown, startGamepadPolling]);

  useEffect(() => () => { if (timerRef.current) cancelAnimationFrame(timerRef.current); }, []);

  // Display
  const hasValue = value.primary_key > 0;
  const isGamepadDevice = device === "xbox_gamepad" || device === "ps5_gamepad";

  const displayKeys = () => {
    const nameFn = isGamepadDevice ? (k: number) => buttonToName(k, device) : vkToName;
    const keys = [...value.modifiers, value.primary_key].filter(k => k > 0);
    const comboKeys = (value.combo_keys || []).filter(k => k > 0);
    return (
      <div className="captured-keys">
        {comboKeys.map((k, i) => (
          <span key={"c"+i}>
            <span className="key-badge">{nameFn(k)}</span>
            <span style={{ color: "var(--text-muted)", margin: "0 2px" }}>+</span>
          </span>
        ))}
        {keys.map((k, i) => (
          <span key={i}>
            {i > 0 && <span style={{ color: "var(--text-muted)", margin: "0 2px" }}>+</span>}
            <span className="key-badge">{nameFn(k)}</span>
          </span>
        ))}
      </div>
    );
  };

  const displayButton = () => (
    <div className="captured-keys">
      <span className="key-badge">{buttonToName(value.primary_key, device)}</span>
    </div>
  );

  const placeholder = () => {
    if (device === "mouse") return "点击捕获鼠标按键";
    if (device === "xbox_gamepad" || device === "ps5_gamepad") return "点击捕获手柄按键";
    return "点击捕获按键";
  };

  return (
    <div
      className={`key-capture-box ${capturing ? "capturing" : ""}`}
      onClick={() => { if (!capturing) setCapturing(true); }}
      tabIndex={0}
    >
      {capturing ? (
        <>
          <div style={{ fontSize: 13, color: "var(--accent)" }}>
            {device === "keyboard"
              ? (heldKeys.size > 0 ? [...heldKeys].map(vkToName).join(" + ") : "请按下按键...")
              : (device === "mouse" ? "请点击鼠标按键..." : "请按手柄任意按键...")}
          </div>
          <div className="hint">松开按键完成捕获 · Esc 取消</div>
        </>
      ) : hasValue ? (
        device === "keyboard" ? displayKeys() : displayButton()
      ) : (
        <>
          <div style={{ fontSize: 13, color: "var(--text-muted)" }}>{placeholder()}</div>
          {device === "keyboard" && <div className="hint">支持组合键（Ctrl/Shift/Alt + 按键）</div>}
        </>
      )}
    </div>
  );
}
