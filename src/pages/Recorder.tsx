import { useEffect, useRef, useState } from "react";
import { api, type RecordedEvent } from "../api";

export default function Recorder() {
  const [recording, setRecording] = useState(false);
  const [events, setEvents] = useState<RecordedEvent[]>([]);
  const [mergeUpDown, setMergeUpDown] = useState(false);
  const timerRef = useRef<number | undefined>(undefined);

  const toggleRecording = async () => {
    if (recording) {
      await api.stopRecording();
    } else {
      await api.startRecording();
    }
    setRecording(!recording);
  };

  useEffect(() => {
    if (recording) {
      timerRef.current = window.setInterval(async () => {
        const evts = await api.getRecordedEvents();
        setEvents(evts);
      }, 50);
    } else {
      clearInterval(timerRef.current);
    }
    return () => clearInterval(timerRef.current);
  }, [recording]);

  const handleClear = async () => {
    await api.clearRecordedEvents();
    setEvents([]);
  };

  const handleExport = async (format: string) => {
    const data = await api.exportEvents(format);
    const blob = new Blob([data], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `events.${format}`;
    a.click();
    URL.revokeObjectURL(url);
    // Flash button text as feedback
    const btn = document.querySelector(`[data-export="${format}"]`);
    if (btn) {
      const orig = btn.textContent;
      btn.textContent = "✓ 已导出";
      setTimeout(() => { btn.textContent = orig; }, 1500);
    }
  };

  // Enhanced merge: merge key press/release, mouse wheel into scroll distance,
  // mouse movement into final coordinates
  const displayEvents = mergeUpDown
    ? events.reduce<RecordedEvent[]>((acc, e) => {
        const last = acc[acc.length - 1];

        // Merge key/button press+release (all devices)
        if (last && last.device === e.device && last.key_code === e.key_code &&
            last.action === "Press" && e.action === "Release") {
          last.action = "按下+释放" as any;
          if (e.delay_ms !== undefined) last.delay_ms = (last.delay_ms || 0) + e.delay_ms;
          return acc;
        }

        // Merge consecutive mouse wheel events into scroll distance
        if (e.device === "Mouse" && (e.key_name === "WheelUp" || e.key_name === "WheelDown")) {
          if (last && last.device === "Mouse" && (last.key_name === "WheelUp" || last.key_name === "WheelDown")) {
            // Accumulate scroll delta
            const prevDelta = last.key_name === "WheelUp" ? 1 : -1;
            const currDelta = e.key_name === "WheelUp" ? 1 : -1;
            const totalDelta = prevDelta + currDelta;
            last.key_name = totalDelta > 0 ? `WheelUp×${Math.abs(totalDelta)}` : `WheelDown×${Math.abs(totalDelta)}`;
            last.key_code = e.key_code;
            if (e.delay_ms !== undefined) last.delay_ms = (last.delay_ms || 0) + e.delay_ms;
            return acc;
          }
        }

        // Merge consecutive horizontal mouse wheel events
        if (e.device === "Mouse" && (e.key_name === "WheelLeft" || e.key_name === "WheelRight")) {
          if (last && last.device === "Mouse" && (last.key_name === "WheelLeft" || last.key_name === "WheelRight")) {
            const prevDelta = last.key_name === "WheelRight" ? 1 : -1;
            const currDelta = e.key_name === "WheelRight" ? 1 : -1;
            const totalDelta = prevDelta + currDelta;
            last.key_name = totalDelta > 0 ? `WheelRight×${Math.abs(totalDelta)}` : `WheelLeft×${Math.abs(totalDelta)}`;
            last.key_code = e.key_code;
            if (e.delay_ms !== undefined) last.delay_ms = (last.delay_ms || 0) + e.delay_ms;
            return acc;
          }
        }

        // Merge mouse press+release
        if (last && last.device === e.device && last.key_code === e.key_code &&
            last.device === "Mouse" && last.action === "Press" && e.action === "Release") {
          last.action = "点击" as any;
          if (e.delay_ms !== undefined) last.delay_ms = (last.delay_ms || 0) + e.delay_ms;
          return acc;
        }

        acc.push({ ...e });
        return acc;
      }, [])
    : events;

  return (
    <div className="page">
      <div className="page-header">
        <h2>事件录制</h2>
        <div className="actions">
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, color: "var(--text-secondary)" }}>
            <button
              className={`toggle ${mergeUpDown ? "on" : ""}`}
              onClick={() => setMergeUpDown(!mergeUpDown)}
            />
            智能合并
          </label>
          <button className="btn" data-export="csv" onClick={() => handleExport("csv")}>导出 CSV</button>
          <button className="btn" data-export="json" onClick={() => handleExport("json")}>导出 JSON</button>
          <button className="btn btn-danger" onClick={handleClear}>清空</button>
          <button
            className={`btn ${recording ? "btn-danger" : "btn-primary"}`}
            onClick={toggleRecording}
          >
            {recording ? "⏹ 停止" : "⏺ 开始录制"}
          </button>
        </div>
      </div>
      <div className="page-body">
        {events.length === 0 ? (
          <div className="empty-state">
            <div className="icon">⏺</div>
            <p>{recording ? "正在录制，请操作键盘/鼠标/手柄..." : "点击开始录制以记录输入事件"}</p>
          </div>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th>时间</th>
                <th>设备</th>
                <th>按键</th>
                <th>键码</th>
                <th>动作</th>
                <th>延时(ms)</th>
              </tr>
            </thead>
            <tbody>
              {displayEvents.map((e, i) => (
                <tr key={i}>
                  <td style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>{e.timestamp}</td>
                  <td>{e.device}</td>
                  <td><span className="key-badge">{e.key_name}</span></td>
                  <td style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-secondary)" }}>
                    0x{e.key_code.toString(16).toUpperCase().padStart(2, "0")}
                  </td>
                  <td>
                    <span style={{
                      color: e.action === "Press" ? "var(--success)"
                        : e.action === "Release" ? "var(--warning)"
                        : "var(--accent)"
                    }}>
                      {e.action === "Press" ? "按下"
                        : e.action === "Release" ? "释放"
                        : e.action}
                    </span>
                  </td>
                  <td style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>
                    {e.delay_ms !== undefined ? `+${e.delay_ms}` : "-"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
