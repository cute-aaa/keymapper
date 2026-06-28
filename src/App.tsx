import { useState, useEffect, Component, type ReactNode } from "react";
import Mappings from "./pages/Mappings";
import Devices from "./pages/Devices";
import Recorder from "./pages/Recorder";
import Logs from "./pages/Logs";
import Settings from "./pages/Settings";
import { api } from "./api";
import "./styles/global.css";

type Page = "mappings" | "devices" | "recorder" | "logs" | "settings";

const navItems: { key: Page; label: string; icon: string }[] = [
  { key: "mappings", label: "映射", icon: "⌨" },
  { key: "devices", label: "设备", icon: "🎮" },
  { key: "recorder", label: "录制", icon: "⏺" },
  { key: "logs", label: "日志", icon: "📋" },
  { key: "settings", label: "设置", icon: "⚙" },
];

class ErrorBoundary extends Component<{ children: ReactNode }, { error: string | null }> {
  state = { error: null as string | null };
  static getDerivedStateFromError(error: Error) {
    return { error: error.message };
  }
  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: 40, color: "var(--error)", fontFamily: "monospace" }}>
          <h2>渲染错误</h2>
          <pre>{this.state.error}</pre>
          <button onClick={() => this.setState({ error: null })} style={{ marginTop: 16, padding: "8px 16px", cursor: "pointer" }}>重试</button>
        </div>
      );
    }
    return this.props.children;
  }
}

function Titlebar() {
  const minimize = () => {
    import("@tauri-apps/api/core").then(({ invoke }) => invoke("window_minimize"));
  };
  const maximize = () => {
    import("@tauri-apps/api/core").then(({ invoke }) => invoke("window_maximize"));
  };
  const close = () => {
    import("@tauri-apps/api/core").then(({ invoke }) => invoke("window_close"));
  };

  return (
    <div className="titlebar" data-tauri-drag-region>
      <span className="titlebar-title">KeyMapper</span>
      <div className="titlebar-controls">
        <button className="titlebar-btn" onClick={minimize} title="最小化">
          <svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"/></svg>
        </button>
        <button className="titlebar-btn" onClick={maximize} title="最大化">
          <svg width="10" height="10" viewBox="0 0 10 10"><rect width="10" height="10" fill="none" stroke="currentColor" strokeWidth="1"/></svg>
        </button>
        <button className="titlebar-btn close" onClick={close} title="关闭">
          <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1 1L9 9M9 1L1 9" stroke="currentColor" strokeWidth="1.2"/></svg>
        </button>
      </div>
    </div>
  );
}

export default function App() {
  const [page, setPage] = useState<Page>("mappings");
  const [tauriOk, setTauriOk] = useState<boolean | null>(null);
  const [errMsg, setErrMsg] = useState<string>("");
  const [theme, setTheme] = useState<string>(() => localStorage.getItem("theme") || "dark");

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("theme", theme);
  }, [theme]);

  useEffect(() => {
    api.getConfig()
      .then(() => {
        setTauriOk(true);
        const saved = localStorage.getItem("theme");
        if (saved) setTheme(saved);
      })
      .catch((e) => {
        setTauriOk(false);
        setErrMsg(String(e));
      });
  }, []);

  if (tauriOk === null) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100vh", color: "var(--text-secondary)", flexDirection: "column", gap: 12 }}>
        <div style={{ fontSize: 24 }}>⏳</div>
        <div>正在连接 Tauri 后端...</div>
      </div>
    );
  }

  if (!tauriOk) {
    return (
      <div style={{ padding: 40, color: "var(--error)", fontFamily: "monospace" }}>
        <h2>Tauri IPC 连接失败</h2>
        <pre style={{ marginTop: 16, padding: 16, background: "var(--bg-elevated)", borderRadius: 8, overflow: "auto" }}>{errMsg}</pre>
        <p style={{ marginTop: 16, color: "var(--text-secondary)" }}>请检查控制台输出</p>
      </div>
    );
  }

  const renderPage = () => {
    switch (page) {
      case "mappings": return <Mappings />;
      case "devices": return <Devices />;
      case "recorder": return <Recorder />;
      case "logs": return <Logs />;
      case "settings": return <Settings theme={theme} setTheme={setTheme} />;
    }
  };

  return (
    <ErrorBoundary>
      <Titlebar />
      <div className="app">
        <aside className="sidebar">
          <div className="logo">
            <span className="logo-icon">🎮</span>
            <span className="logo-text">KeyMapper</span>
          </div>
          <nav className="nav">
            {navItems.map((item) => (
              <button
                key={item.key}
                className={`nav-item ${page === item.key ? "active" : ""}`}
                onClick={() => setPage(item.key)}
              >
                <span className="nav-icon">{item.icon}</span>
                <span className="nav-label">{item.label}</span>
              </button>
            ))}
          </nav>
          <div className="sidebar-footer">
            <span className="version">v0.1.0</span>
          </div>
        </aside>
        <main className="main">{renderPage()}</main>
      </div>
    </ErrorBoundary>
  );
}
