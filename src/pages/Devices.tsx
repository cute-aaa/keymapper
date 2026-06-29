import { useEffect, useState } from "react";
import { api, type DeviceInfo } from "../api";

const BUILTIN_DEVICES: DeviceInfo[] = [
  { id: "builtin_keyboard", name: "键盘", device_type: "keyboard", connected: true, port: undefined },
  { id: "builtin_mouse", name: "鼠标", device_type: "mouse", connected: true, port: undefined },
];

export default function Devices() {
  const [devices, setDevices] = useState<DeviceInfo[]>(BUILTIN_DEVICES);
  const [loading, setLoading] = useState(false);
  const [diagResult, setDiagResult] = useState<string[]>([]);
  const [diagRunning, setDiagRunning] = useState(false);

  const loadDevices = async () => {
    setLoading(true);
    try {
      const external = await api.getDevices();
      const merged = [...BUILTIN_DEVICES];
      for (const d of external) { if (!merged.some((m) => m.id === d.id)) merged.push(d); }
      setDevices(merged);
    } catch { setDevices(BUILTIN_DEVICES); }
    setLoading(false);
  };

  useEffect(() => {
    loadDevices();
    const timer = window.setInterval(loadDevices, 2000);
    return () => clearInterval(timer);
  }, []);

  const runDiagnose = async () => {
    setDiagRunning(true);
    setDiagResult(["正在读取手柄数据（15秒），请逐个按下手柄按键..."]);
    try {
      const result = await api.diagnoseGamepad(15000);
      setDiagResult(result);
    } catch (e) {
      setDiagResult([`诊断失败: ${e}`]);
    }
    setDiagRunning(false);
  };

  const deviceIcon = (type: string) => {
    switch (type) {
      case "keyboard": return "⌨";
      case "mouse": return "🖱";
      case "xbox_gamepad": case "ps5_gamepad": return "🎮";
      default: return "🔌";
    }
  };

  const deviceTypeName = (type: string) => {
    switch (type) {
      case "keyboard": return "键盘";
      case "mouse": return "鼠标";
      case "xbox_gamepad": return "Xbox 手柄";
      case "ps5_gamepad": return "PS5 手柄";
      default: return type;
    }
  };

  const hasGamepad = devices.some((d) => d.device_type === "xbox_gamepad" || d.device_type === "ps5_gamepad");

  return (
    <div className="page">
      <div className="page-header">
        <h2>设备管理</h2>
        <div className="actions">
          <button className="btn" onClick={loadDevices} disabled={loading}>
            {loading ? "刷新中..." : "↻ 刷新"}
          </button>
          {hasGamepad && (
            <>
              <button className="btn btn-primary" onClick={runDiagnose} disabled={diagRunning}>
                {diagRunning ? "诊断中..." : "🔍 诊断手柄"}
              </button>
              <button className="btn" onClick={async () => {
                const ok = await api.resetDualsense();
                alert(ok ? "手柄已重置" : "重置失败（未找到 DualSense MI_00）");
              }}>🔄 重置手柄</button>
            </>
          )}
        </div>
      </div>
      <div className="page-body">
        <div className="device-grid">
          {devices.map((d) => (
            <div key={d.id} className="device-card">
              <div className="device-icon">{deviceIcon(d.device_type)}</div>
              <div className="device-name">{d.name}</div>
              <div className="device-type">{deviceTypeName(d.device_type)}</div>
              <div className="status">
                <span className={`status-dot ${d.connected ? "online" : "offline"}`} />
                {d.connected ? "已连接" : "未连接"}
              </div>
            </div>
          ))}
        </div>

        {!hasGamepad && (
          <div style={{ marginTop: 24, padding: "12px 16px", background: "var(--bg-elevated)", border: "1px solid var(--border)", borderRadius: 8, fontSize: 13, color: "var(--text-secondary)" }}>
            💡 未检测到手柄。请连接 Xbox/PS5 手柄并按任意按键。
          </div>
        )}

        {diagResult.length > 0 && (
          <div style={{ marginTop: 16, padding: "12px 16px", background: "var(--bg)", border: "1px solid var(--border)", borderRadius: 8 }}>
            <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8, color: "var(--text-secondary)" }}>诊断结果</div>
            <pre style={{ fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--text-primary)", whiteSpace: "pre-wrap", lineHeight: 1.6, margin: 0 }}>
              {diagResult.join("\n")}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
