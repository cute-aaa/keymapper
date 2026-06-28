import { useEffect, useRef, useState } from "react";
import { api, type LogEntry } from "../api";

export default function Logs() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [filter, setFilter] = useState<string>("all");
  const containerRef = useRef<HTMLDivElement>(null);
  const timerRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    // Poll logs from backend
    const loadLogs = async () => {
      try {
        const entries = await api.getLogs();
        setLogs(entries);
      } catch {
        // ignore errors during polling
      }
    };

    loadLogs();
    timerRef.current = window.setInterval(loadLogs, 500);
    return () => clearInterval(timerRef.current);
  }, []);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  const filtered = filter === "all"
    ? logs
    : logs.filter((l) => l.level === filter);

  return (
    <div className="page">
      <div className="page-header">
        <h2>程序日志</h2>
        <div className="actions">
          <select
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            style={{
              padding: "6px 10px",
              borderRadius: 6,
              border: "1px solid var(--border)",
              background: "var(--bg-elevated)",
              color: "var(--text-primary)",
              fontSize: 12,
            }}
          >
            <option value="all">全部</option>
            <option value="debug">Debug</option>
            <option value="info">Info</option>
            <option value="warn">Warn</option>
            <option value="error">Error</option>
          </select>
          <button className="btn btn-danger" onClick={async () => {
            await api.clearLogs();
            setLogs([]);
          }}>清空</button>
        </div>
      </div>
      <div className="page-body">
        <div className="log-container" ref={containerRef}>
          {filtered.length === 0 ? (
            <div style={{ color: "var(--text-muted)", textAlign: "center", padding: 40 }}>
              暂无日志
            </div>
          ) : (
            filtered.map((log, i) => (
              <div key={i} className="log-entry">
                <span className="log-time">{log.time}</span>
                <span className={`log-level ${log.level}`}>
                  [{log.level.toUpperCase()}]
                </span>
                <span className="log-msg">{log.message}</span>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
