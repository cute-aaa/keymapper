import { useEffect, useState } from "react";
import { api, type AppConfig } from "../api";

interface SettingsProps {
  theme: string;
  setTheme: (t: string) => void;
}

export default function Settings({ theme, setTheme }: SettingsProps) {
  const [config, setConfig] = useState<AppConfig | null>(null);

  useEffect(() => {
    api.getConfig().then(setConfig);
  }, []);

  const update = async (key: string, value: unknown) => {
    const updated = await api.updateSettings({ [key]: value });
    setConfig(updated);
  };

  if (!config) return null;

  return (
    <div className="page">
      <div className="page-header">
        <h2>设置</h2>
      </div>
      <div className="page-body" style={{ maxWidth: 500 }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
          <section>
            <h3 style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, color: "var(--text-secondary)" }}>
              外观
            </h3>
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <SettingRow label="主题" desc="切换亮色/暗色主题">
                <div style={{ display: "flex", gap: 4 }}>
                  <button
                    className={`btn btn-sm ${theme === "dark" ? "btn-primary" : ""}`}
                    onClick={() => setTheme("dark")}
                  >
                    🌙 暗色
                  </button>
                  <button
                    className={`btn btn-sm ${theme === "light" ? "btn-primary" : ""}`}
                    onClick={() => setTheme("light")}
                  >
                    ☀️ 亮色
                  </button>
                </div>
              </SettingRow>
            </div>
          </section>

          <section>
            <h3 style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, color: "var(--text-secondary)" }}>
              通用
            </h3>
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <SettingRow label="开机自启动" desc="Windows 启动时自动运行">
                <button
                  className={`toggle ${config.auto_start ? "on" : ""}`}
                  onClick={() => update("auto_start", !config.auto_start)}
                />
              </SettingRow>
              <SettingRow label="启动时最小化" desc="程序启动后直接最小化到托盘">
                <button
                  className={`toggle ${config.start_minimized ? "on" : ""}`}
                  onClick={() => update("start_minimized", !config.start_minimized)}
                />
              </SettingRow>
              <SettingRow label="关闭时最小化到托盘" desc="点击关闭按钮时隐藏到系统托盘而非退出">
                <button
                  className={`toggle ${config.minimize_to_tray ? "on" : ""}`}
                  onClick={() => update("minimize_to_tray", !config.minimize_to_tray)}
                />
              </SettingRow>
            </div>
          </section>

          <section>
            <h3 style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, color: "var(--text-secondary)" }}>
              配置文件
            </h3>
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn" onClick={async () => {
                const cfg = await api.importConfigFile("config.json");
                setConfig(cfg);
              }}>
                导入配置
              </button>
              <button className="btn" onClick={() => api.exportConfigFile("config-export.json")}>
                导出配置
              </button>
            </div>
          </section>

          <section>
            <h3 style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, color: "var(--text-secondary)" }}>
              Profile 管理
            </h3>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {config.profiles.map((p) => (
                <div
                  key={p.id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "8px 12px",
                    borderRadius: 8,
                    background: p.id === config.active_profile_id ? "var(--accent-dim)" : "var(--bg-elevated)",
                    border: `1px solid ${p.id === config.active_profile_id ? "var(--accent)" : "var(--border)"}`,
                  }}
                >
                  <span style={{ fontSize: 13, fontWeight: p.id === config.active_profile_id ? 600 : 400 }}>
                    {p.name}
                  </span>
                  {p.id !== config.active_profile_id && (
                    <button className="btn btn-sm" onClick={() => api.updateSettings({ active_profile_id: p.id }).then(setConfig)}>
                      切换
                    </button>
                  )}
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}

function SettingRow({
  label,
  desc,
  children,
}: {
  label: string;
  desc: string;
  children: React.ReactNode;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "10px 14px",
        borderRadius: 8,
        background: "var(--bg-elevated)",
        border: "1px solid var(--border)",
      }}
    >
      <div>
        <div style={{ fontSize: 13, fontWeight: 500 }}>{label}</div>
        <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 2 }}>{desc}</div>
      </div>
      {children}
    </div>
  );
}
