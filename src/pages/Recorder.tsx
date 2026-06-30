import { useEffect, useRef, useState, useCallback, Fragment } from "react";
import { api, type RecordedEvent } from "../api";
import { vkToName } from "../components/KeyCapture";

interface SavedSequence {
  id: string;
  name: string;
  events: RecordedEvent[];
  savedAt: string;
  replayKey?: string;
}

const STORAGE_KEY = "km_saved_sequences";

/** Parse timestamp "HH:MM:SS.mmm" to milliseconds from midnight */
function tsToMs(ts: string): number {
  const m = ts.match(/(\d{1,2}):(\d{2}):(\d{2})[.](\d{3})/);
  if (!m) return 0;
  return (+m[1]) * 3600000 + (+m[2]) * 60000 + (+m[3]) * 1000 + (+m[4]);
}

/** Format milliseconds to "SS.mmm" or "MM:SS.mmm" */
function fmtElapsed(ms: number): string {
  if (ms < 0) ms = 0;
  const s = Math.floor(ms / 1000);
  const msR = ms % 1000;
  if (s < 60) return `${s}.${String(msR).padStart(3, "0")}`;
  const m = Math.floor(s / 60);
  const sR = s % 60;
  return `${m}:${String(sR).padStart(2, "0")}.${String(msR).padStart(3, "0")}`;
}

/** Convert absolute timestamps to elapsed from first event */
function toElapsed(events: { timestamp: string }[]): string[] {
  if (events.length === 0) return [];
  const base = tsToMs(events[0].timestamp);
  return events.map(e => fmtElapsed(tsToMs(e.timestamp) - base));
}
function genId() { return Date.now().toString(36) + Math.random().toString(36).slice(2, 6); }

function keyToVk(key: string, keyCode: number): number {
  if (keyCode > 0) return keyCode;
  const map: Record<string, number> = {
    "Backspace": 0x08, "Tab": 0x09, "Enter": 0x0D, "Shift": 0x10, "Control": 0x11,
    "Alt": 0x12, "CapsLock": 0x14, "Escape": 0x1B, "Space": 0x20,
    "ArrowLeft": 0x25, "ArrowUp": 0x26, "ArrowRight": 0x27, "ArrowDown": 0x28,
    "Delete": 0x2E, "F1": 0x70, "F2": 0x71, "F3": 0x72, "F4": 0x73, "F5": 0x74,
    "F6": 0x75, "F7": 0x76, "F8": 0x77, "F9": 0x78, "F10": 0x79, "F11": 0x7A, "F12": 0x7B,
  };
  if (map[key]) return map[key];
  if (key.length === 1) return key.toUpperCase().charCodeAt(0);
  return 0;
}

const VK_TO_NAME: Record<number, string> = {};
for (let i = 0x08; i <= 0xFF; i++) { try { VK_TO_NAME[i] = vkToName(i); } catch {} }

// Keyboard keys for datalist
const KB_KEYS = [
  "A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
  "0","1","2","3","4","5","6","7","8","9",
  "F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12",
  "Shift","Ctrl","Alt","LShift","RShift","LCtrl","RCtrl","LAlt","RAlt",
  "Space","Enter","Tab","Esc","Backspace","Delete","Insert",
  "↑","↓","←","→","Home","End","PageUp","PageDown",
];

// Mouse buttons for dropdown
const MOUSE_KEYS = [
  { v: "鼠标左键", l: "鼠标左键" },
  { v: "鼠标右键", l: "鼠标右键" },
  { v: "鼠标中键", l: "鼠标中键" },
  { v: "Mouse4", l: "前进 (Mouse4)" },
  { v: "Mouse5", l: "后退 (Mouse5)" },
  { v: "WheelUp", l: "滚轮 ↑" },
  { v: "WheelDown", l: "滚轮 ↓" },
  { v: "WheelLeft", l: "滚轮 ←" },
  { v: "WheelRight", l: "滚轮 →" },
];

// PS5 buttons for dropdown
const PS5_KEYS = [
  { v: "×", l: "× (A)" }, { v: "○", l: "○ (B)" }, { v: "□", l: "□ (X)" }, { v: "△", l: "△ (Y)" },
  { v: "L1", l: "L1 (LB)" }, { v: "R1", l: "R1 (RB)" }, { v: "L2", l: "L2 (LT)" }, { v: "R2", l: "R2 (RT)" },
  { v: "Share", l: "Share" }, { v: "Options", l: "Options" },
  { v: "L3", l: "L3 (按下)" }, { v: "R3", l: "R3 (按下)" },
  { v: "PS", l: "PS" }, { v: "触摸板", l: "触摸板" }, { v: "静音", l: "静音" },
  { v: "↑", l: "↑" }, { v: "↓", l: "↓" }, { v: "←", l: "←" }, { v: "→", l: "→" },
];

// Xbox buttons for dropdown
const XBOX_KEYS = [
  { v: "A", l: "A" }, { v: "B", l: "B" }, { v: "X", l: "X" }, { v: "Y", l: "Y" },
  { v: "LB", l: "LB" }, { v: "RB", l: "RB" }, { v: "LT", l: "LT" }, { v: "RT", l: "RT" },
  { v: "View", l: "View" }, { v: "Menu", l: "Menu" },
  { v: "L3", l: "L3 (按下)" }, { v: "R3", l: "R3 (按下)" },
  { v: "Xbox", l: "Xbox" },
  { v: "↑", l: "↑" }, { v: "↓", l: "↓" }, { v: "←", l: "←" }, { v: "→", l: "→" },
];

function getKeyOptions(device: string): { v: string; l: string }[] {
  if (device === "Mouse") return MOUSE_KEYS;
  if (device === "PS5") return PS5_KEYS;
  if (device === "Xbox") return XBOX_KEYS;
  return []; // Keyboard uses datalist
}

export default function Recorder() {
  const [recording, setRecording] = useState(false);
  const [events, setEvents] = useState<RecordedEvent[]>([]);
  const [mergeUpDown, setMergeUpDown] = useState(true);
  const [savedSeqs, setSavedSeqs] = useState<SavedSequence[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [pendingSave, setPendingSave] = useState<RecordedEvent[] | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");
  const [hoverSeqId, setHoverSeqId] = useState<string | null>(null);
  const [hoverEventIdx, setHoverEventIdx] = useState<number | null>(null);
  const [replayingId, setReplayingId] = useState<string | null>(null);
  const [pausedId, setPausedId] = useState<string | null>(null);
  const [replayCount, setReplayCount] = useState(1);
  const [replaySpeed, setReplaySpeed] = useState(1.0);
  const [hoverReplayBtn, setHoverReplayBtn] = useState<string | null>(null);
  const [replayCurrentIdx, setReplayCurrentIdx] = useState<number>(-1);
  const [capturingKeyId, setCapturingKeyId] = useState<string | null>(null);
  const timerRef = useRef<number | undefined>(undefined);
  const eventsEndRef = useRef<HTMLDivElement>(null);
  const replayAbortRef = useRef<AbortController | null>(null);
  const replayEventRefs = useRef<Map<number, HTMLDivElement>>(new Map());

  useEffect(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed)) setSavedSeqs(parsed.filter(s => s && s.id && s.name && Array.isArray(s.events)));
      }
    } catch {}
  }, []);

  const persistSeqs = (seqs: SavedSequence[]) => { setSavedSeqs(seqs); localStorage.setItem(STORAGE_KEY, JSON.stringify(seqs)); };

  const toggleRecording = useCallback(async () => {
    if (recording) {
      await api.stopRecording();
      const evts = await api.getRecordedEvents();
      setEvents(evts);
      if (evts.length > 0) setPendingSave(evts);
      setRecording(false);
    } else {
      await api.clearRecordedEvents(); setEvents([]); setPendingSave(null);
      await api.startRecording(); setRecording(true);
    }
  }, [recording]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "F9" && !e.ctrlKey && !e.altKey && !e.shiftKey) { e.preventDefault(); toggleRecording(); return; }
      for (const seq of savedSeqs) {
        if (seq.replayKey && e.key === seq.replayKey && !e.ctrlKey && !e.altKey && !e.shiftKey) { e.preventDefault(); doReplay(seq.id); return; }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [toggleRecording, savedSeqs]);

  useEffect(() => {
    if (recording) {
      timerRef.current = window.setInterval(async () => { setEvents(await api.getRecordedEvents()); }, 50);
    } else { clearInterval(timerRef.current); }
    return () => clearInterval(timerRef.current);
  }, [recording]);

  useEffect(() => {
    if (recording && eventsEndRef.current) eventsEndRef.current.scrollIntoView({ behavior: "smooth" });
  }, [events, recording]);

  // Frontend keyboard capture
  useEffect(() => {
    if (!recording) return;
    const kd = (e: KeyboardEvent) => { const vk = keyToVk(e.key, e.keyCode); if (vk) api.recordFrontendKey(vk, "Press"); };
    const ku = (e: KeyboardEvent) => { const vk = keyToVk(e.key, e.keyCode); if (vk) api.recordFrontendKey(vk, "Release"); };
    window.addEventListener("keydown", kd, true); window.addEventListener("keyup", ku, true);
    return () => { window.removeEventListener("keydown", kd, true); window.removeEventListener("keyup", ku, true); };
  }, [recording]);

  const handleSavePending = (name?: string) => {
    if (!pendingSave) return;
    const seq: SavedSequence = { id: genId(), name: name?.trim() || `序列 ${savedSeqs.length + 1}`, events: [...pendingSave], savedAt: new Date().toLocaleString("zh-CN") };
    persistSeqs([seq, ...savedSeqs.filter(s => s.name !== seq.name)]); setPendingSave(null); setEvents([]);
  };

  const doReplay = async (id: string) => {
    if (replayingId === id) {
      // Stop
      replayAbortRef.current?.abort();
      setReplayingId(null); setPausedId(null); setReplayCurrentIdx(-1); setExpandedId(null);
      return;
    }
    if (pausedId === id) {
      // Resume — not fully implemented, just restart
      setPausedId(null);
    }
    const seq = savedSeqs.find(s => s.id === id);
    if (!seq || seq.events.length === 0) return;
    setReplayingId(id); setExpandedId(id); setReplayCurrentIdx(0);
    const ac = new AbortController(); replayAbortRef.current = ac;
    for (let rep = 0; rep < replayCount && !ac.signal.aborted; rep++) {
      for (let i = 0; i < seq.events.length && !ac.signal.aborted; i++) {
        setReplayCurrentIdx(i);
        // Scroll current event into view (centered)
        requestAnimationFrame(() => {
          const el = replayEventRefs.current.get(i);
          if (el) el.scrollIntoView({ behavior: "smooth", block: "center" });
        });
        const ev = seq.events[i];
        if (ev.delay_ms && ev.delay_ms > 0) {
          await new Promise(r => setTimeout(r, ev.delay_ms! / replaySpeed));
        }
      }
    }
    setReplayingId(null); setReplayCurrentIdx(-1);
  };

  const doReplayCurrent = async () => {
    if (events.length === 0) return;
    setReplayingId("current");
    try { await api.replayEvents(replayCount, replaySpeed); } catch {}
    setTimeout(() => setReplayingId(null), 500);
  };

  const handleDeleteSeq = (id: string) => { persistSeqs(savedSeqs.filter(s => s.id !== id)); if (expandedId === id) setExpandedId(null); selectedIds.delete(id); setSelectedIds(new Set(selectedIds)); };
  const handleRename = (id: string) => { persistSeqs(savedSeqs.map(s => s.id === id ? { ...s, name: editingName || s.name } : s)); setEditingId(null); };
  const handleSetReplayKey = (id: string, key: string) => { persistSeqs(savedSeqs.map(s => s.id === id ? { ...s, replayKey: key || undefined } : s)); };
  const handleDeleteEvent = (seqId: string, idx: number) => { persistSeqs(savedSeqs.map(s => { if (s.id !== seqId) return s; const evts = [...s.events]; evts.splice(idx, 1); return { ...s, events: evts }; })); };
  const handleUpdateEvent = (seqId: string, idx: number, patch: Partial<RecordedEvent>) => { persistSeqs(savedSeqs.map(s => { if (s.id !== seqId) return s; const evts = [...s.events]; evts[idx] = { ...evts[idx], ...patch }; return { ...s, events: evts }; })); };
  const handleExportSeq = (seq: SavedSequence) => { const blob = new Blob([JSON.stringify(seq, null, 2)], { type: "application/json" }); const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = `${seq.name}.json`; a.click(); };
  const handleImport = () => { const input = document.createElement("input"); input.type = "file"; input.accept = ".json"; input.multiple = true; input.onchange = async (e) => { const files = Array.from((e.target as HTMLInputElement).files || []); const imported: SavedSequence[] = []; for (const f of files) { try { const d = JSON.parse(await f.text()); const seqs = Array.isArray(d) ? d : [d]; for (const s of seqs) { if (s.events && Array.isArray(s.events)) { if (!s.id) s.id = genId(); if (!s.name) s.name = f.name.replace(/\.json$/i, ""); if (!s.savedAt) s.savedAt = new Date().toLocaleString("zh-CN"); imported.push(s); } } } catch {} } if (imported.length) persistSeqs([...imported, ...savedSeqs.filter(s => !imported.some(i => i.name === s.name))]); }; input.click(); };
  const toggleSelect = (id: string, e: React.MouseEvent) => { const n = new Set(selectedIds); if (e.ctrlKey || e.metaKey) { n.has(id) ? n.delete(id) : n.add(id); } else { n.clear(); n.add(id); } setSelectedIds(n); };
  const handleBatchDelete = () => { if (!selectedIds.size || !confirm(`删除 ${selectedIds.size} 个序列？`)) return; persistSeqs(savedSeqs.filter(s => !selectedIds.has(s.id))); setSelectedIds(new Set()); };
  const handleBatchExport = () => { const sel = savedSeqs.filter(s => selectedIds.has(s.id)); if (!sel.length) return; const blob = new Blob([JSON.stringify(sel.length === 1 ? sel[0] : sel, null, 2)], { type: "application/json" }); const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = sel.length === 1 ? `${sel[0].name}.json` : `sequences_${sel.length}.json`; a.click(); };

  const getActionLabel = (a: string) => a === "Press" ? "按下" : a === "Release" ? "释放" : a;
  const getActionColor = (a: string) => a === "Press" ? "var(--success)" : a === "Release" ? "var(--warning)" : "var(--accent)";

  const displayEvents = mergeUpDown
    ? events.reduce<RecordedEvent[]>((acc, e) => {
        const last = acc[acc.length - 1];
        if (last && last.device === e.device && last.key_code === e.key_code && last.action === "Press" && e.action === "Release") { last.action = "按下+释放" as any; if (e.delay_ms) last.delay_ms = (last.delay_ms || 0) + e.delay_ms; return acc; }
        const wheelRe = /^Wheel(Up|Down|Left|Right)/;
        if (wheelRe.test(e.key_name)) {
          const dir = e.key_name.replace(/^Wheel/, "").replace(/[×x]\d+$/, "");
          if (last && last.device === "Mouse" && wheelRe.test(last.key_name)) {
            const ldir = last.key_name.replace(/^Wheel/, "").replace(/[×x]\d+$/, "");
            if (ldir === dir || (ldir === "Up" && dir === "Down") || (ldir === "Down" && dir === "Up") || (ldir === "Left" && dir === "Right") || (ldir === "Right" && dir === "Left")) {
              const pm = last.key_name.match(/[×x](\d+)$/); const pc = pm ? parseInt(pm[1]) : 1;
              last.key_name = `Wheel${dir}×${pc + 1}`; last.key_code = e.key_code; if (e.delay_ms) last.delay_ms = (last.delay_ms || 0) + e.delay_ms; return acc;
            }
          }
          e.key_name = `Wheel${dir}`;
        }
        acc.push({ ...e }); return acc;
      }, [])
    : events;

  // Capture replay key
  useEffect(() => {
    if (!capturingKeyId) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault(); e.stopPropagation();
      if (e.key === "Escape") { setCapturingKeyId(null); return; }
      handleSetReplayKey(capturingKeyId, e.key);
      setCapturingKeyId(null);
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [capturingKeyId]);

  const isSeqActive = (id: string) => expandedId === id || editingId === id || replayingId === id || selectedIds.has(id);

  return (
    <div className="page">
      <div className="page-header">
        <h2>事件录制</h2>
        <div className="actions">
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, color: "var(--text-secondary)" }}>
            <button className={`toggle ${mergeUpDown ? "on" : ""}`} onClick={() => setMergeUpDown(!mergeUpDown)} />智能合并</label>
          <button className="btn" onClick={handleImport}>📥 导入</button>
          {recording && <button className="btn btn-danger" onClick={async () => { await api.clearRecordedEvents(); setEvents([]); }}>清空</button>}
          <button className={`btn ${recording ? "btn-danger" : "btn-primary"}`} onClick={toggleRecording}>{recording ? "⏹ 停止 (F9)" : "⏺ 开始录制 (F9)"}</button>
        </div>
      </div>
      <div className="page-body">

        {/* Saved sequences — hidden during recording */}
        {!recording && <div style={{ marginBottom: 12 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontSize: 14, fontWeight: 500 }}>已保存的序列 ({savedSeqs.length})</span>
            {selectedIds.size > 0 && (<div style={{ display: "flex", gap: 6, alignItems: "center" }}>
              <span style={{ fontSize: 12, color: "var(--accent)" }}>已选 {selectedIds.size} 个</span>
              <button className="btn btn-sm" onClick={handleBatchExport}>批量导出</button>
              <button className="btn btn-sm btn-danger" onClick={handleBatchDelete}>批量删除</button>
              <button className="btn btn-sm" onClick={() => setSelectedIds(new Set())}>取消</button>
            </div>)}
          </div>
          {savedSeqs.length === 0 ? (
            <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: 0, padding: 12, background: "var(--surface)", borderRadius: 8 }}>暂无保存的序列。录制结束后点击「保存」添加。</p>
          ) : (
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border)" }}>
                  <th style={{ textAlign: "left", padding: "4px 8px", color: "var(--text-muted)", fontWeight: 400, width: 10 }}> </th>
                  <th style={{ textAlign: "left", padding: "4px 8px", color: "var(--text-muted)", fontWeight: 400 }}>名称</th>
                  <th style={{ textAlign: "left", padding: "4px 8px", color: "var(--text-muted)", fontWeight: 400, width: 50 }}>事件</th>
                  <th style={{ textAlign: "left", padding: "4px 8px", color: "var(--text-muted)", fontWeight: 400, width: 50 }}>快捷键</th>
                  <th style={{ textAlign: "left", padding: "4px 8px", color: "var(--text-muted)", fontWeight: 400, width: 140 }}>保存时间</th>
                  <th style={{ textAlign: "right", padding: "4px 8px", color: "var(--text-muted)", fontWeight: 400, width: 120 }}>操作</th>
                </tr>
              </thead>
              <tbody>
              {savedSeqs.map((seq) => {
                const isActive = isSeqActive(seq.id);
                const isHover = hoverSeqId === seq.id;
                const showUI = isActive || isHover;
                const isReplaying = replayingId === seq.id;
                const isReplayHover = hoverReplayBtn === seq.id;
                return (
                  <Fragment key={seq.id}>
                  <tr
                    onMouseEnter={() => setHoverSeqId(seq.id)}
                    onMouseLeave={() => { setHoverSeqId(null); setHoverReplayBtn(null); }}
                    onClick={(e) => { if (e.ctrlKey || e.metaKey) { toggleSelect(seq.id, e); } else if (!isReplaying) { setExpandedId(expandedId === seq.id ? null : seq.id); } }}
                    style={{
                      cursor: "pointer",
                      background: isReplaying ? "var(--accent-bg)" : selectedIds.has(seq.id) ? "var(--accent-bg)" : expandedId === seq.id ? "var(--hover)" : isHover ? "var(--hover)" : "transparent",
                      borderLeft: isReplaying ? "3px solid var(--accent)" : selectedIds.has(seq.id) ? "3px solid var(--accent)" : "3px solid transparent",
                      transition: "all 0.15s",
                    }}>
                    <td style={{ padding: "4px 8px", fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--text-muted)" }}>{expandedId === seq.id ? "▼" : "›"}</td>
                    <td style={{ padding: "4px 8px" }}>
                      {editingId === seq.id ? (
                        <input type="text" value={editingName} autoFocus onChange={(e) => setEditingName(e.target.value)} onBlur={() => handleRename(seq.id)} onKeyDown={(e) => { if (e.key === "Enter") handleRename(seq.id); }} onClick={(e) => e.stopPropagation()} style={{ width: "100%", padding: "2px 6px", fontSize: 12 }} />
                      ) : (
                        <span style={{ fontWeight: isReplaying ? 600 : 500, color: isReplaying ? "var(--accent)" : undefined }}>{seq.name}</span>
                      )}
                      {isReplaying && <span style={{ fontSize: 10, color: "var(--accent)", marginLeft: 6 }}>回放中 {replayCurrentIdx + 1}/{seq.events.length}</span>}
                    </td>
                    <td style={{ padding: "4px 8px", color: "var(--text-secondary)" }}>{seq.events.length}</td>
                    <td style={{ padding: "4px 8px" }}>{seq.replayKey && <span className="key-badge" style={{ fontSize: 9 }}>{seq.replayKey}</span>}</td>
                    <td style={{ padding: "4px 8px", color: "var(--text-muted)", fontSize: 10 }}>{seq.savedAt}</td>
                    <td style={{ padding: "4px 8px", textAlign: "right" }}>
                      {showUI && (
                        <div style={{ display: "inline-flex", gap: 3, alignItems: "center" }}>
                          {/* Replay area: hover expands to button group */}
                          <div
                            onMouseEnter={() => setHoverReplayBtn(seq.id)}
                            onMouseLeave={() => setHoverReplayBtn(null)}
                            style={{ display: "flex", alignItems: "center", gap: 3, transition: "all 0.2s ease" }}>
                            {isReplaying ? (
                              // Replaying: show stop button
                              <button className="btn btn-sm btn-danger" style={{ padding: "1px 8px", fontSize: 11 }}
                                onClick={(e) => { e.stopPropagation(); doReplay(seq.id); }}>■ 停止</button>
                            ) : isReplayHover ? (
                              // Hover: expand to button group
                              <div style={{ display: "flex", alignItems: "center", gap: 3, background: "var(--hover)", borderRadius: 4, padding: "1px 4px", animation: "fadeIn 0.15s" }}>
                                <select value={replayCount} onChange={(e) => { e.stopPropagation(); setReplayCount(parseInt(e.target.value)); }} onClick={(e) => e.stopPropagation()}
                                  style={{ padding: "0 2px", fontSize: 10, width: 36, border: "1px solid var(--border)", borderRadius: 3, background: "var(--surface)" }}>
                                  {[1,2,3,5,10,20].map(n => <option key={n} value={n}>{n}次</option>)}
                                </select>
                                <select value={replaySpeed} onChange={(e) => { e.stopPropagation(); setReplaySpeed(parseFloat(e.target.value)); }} onClick={(e) => e.stopPropagation()}
                                  style={{ padding: "0 2px", fontSize: 10, width: 40, border: "1px solid var(--border)", borderRadius: 3, background: "var(--surface)" }}>
                                  {[0.5,1,1.5,2,4].map(s => <option key={s} value={s}>{s}x</option>)}
                                </select>
                                <button className="btn btn-sm btn-primary" style={{ padding: "1px 8px", fontSize: 11 }}
                                  onClick={(e) => { e.stopPropagation(); doReplay(seq.id); }}>▶ 回放</button>
                              </div>
                            ) : (
                              // Default: single replay button
                              <button className="btn btn-sm" style={{ padding: "1px 6px", fontSize: 11 }}
                                onClick={(e) => { e.stopPropagation(); doReplay(seq.id); }}>▶</button>
                            )}
                          </div>
                          {!isReplaying && <>
                            <button className="btn btn-sm" style={{ padding: "1px 4px", fontSize: 11 }} onClick={(e) => { e.stopPropagation(); setEditingId(seq.id); setEditingName(seq.name); setExpandedId(seq.id); }}>✏️</button>
                            <button className="btn btn-sm" style={{ padding: "1px 4px", fontSize: 11 }} onClick={(e) => { e.stopPropagation(); handleExportSeq(seq); }}>💾</button>
                            <button className="btn btn-sm btn-danger" style={{ padding: "1px 4px", fontSize: 11 }} onClick={(e) => { e.stopPropagation(); handleDeleteSeq(seq.id); }}>✕</button>
                          </>}
                        </div>
                      )}
                    </td>
                  </tr>
                    {/* Expanded: events list */}
                    {expandedId === seq.id && (
                      <tr><td colSpan={6} style={{ padding: 0, background: "var(--surface)", borderBottom: "1px solid var(--border)" }}><div>
                        {/* Edit bar */}
                        <div style={{ display: "flex", gap: 8, alignItems: "center", padding: "4px 10px", borderBottom: "1px solid var(--border)", fontSize: 11 }}>
                          <span style={{ color: "var(--text-secondary)" }}>快捷键:</span>
                          {capturingKeyId === seq.id ? (
                            <span style={{ color: "var(--accent)", cursor: "pointer" }} onClick={() => setCapturingKeyId(null)}>按下按键捕获... (点击取消)</span>
                          ) : (
                            <button className="btn btn-sm" style={{ padding: "0 6px", fontSize: 10 }} onClick={(e) => { e.stopPropagation(); setCapturingKeyId(seq.id); }}>
                              {seq.replayKey || "捕获"}
                            </button>
                          )}
                          {seq.replayKey && <button className="btn btn-sm" style={{ padding: "0 4px", fontSize: 10 }} onClick={(e) => { e.stopPropagation(); handleSetReplayKey(seq.id, ""); }}>清除</button>}
                        </div>
                        {/* Event table */}
                        <div style={{ maxHeight: 240, overflowY: "auto" }}>
                          {(() => { const elapsed = toElapsed(seq.events); return (
                          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
                            <thead>
                              <tr style={{ borderBottom: "1px solid var(--border)" }}>
                                <th style={{ textAlign: "left", padding: "2px 6px", color: "var(--text-muted)", fontWeight: 400, width: 60 }}>时间</th>
                                <th style={{ textAlign: "left", padding: "2px 6px", color: "var(--text-muted)", fontWeight: 400, width: 50 }}>设备</th>
                                <th style={{ textAlign: "left", padding: "2px 6px", color: "var(--text-muted)", fontWeight: 400 }}>按键</th>
                                <th style={{ textAlign: "left", padding: "2px 6px", color: "var(--text-muted)", fontWeight: 400, width: 60 }}>动作</th>
                                <th style={{ textAlign: "left", padding: "2px 6px", color: "var(--text-muted)", fontWeight: 400, width: 60 }}>延时</th>
                              </tr>
                            </thead>
                            <tbody>
                              {seq.events.map((ev, i) => {
                                const isEvHover = hoverEventIdx === i && expandedId === seq.id && !isReplaying;
                                const isReplayEv = isReplaying && replayCurrentIdx === i;
                                return (
                                  <tr key={i}
                                    ref={(el) => { if (el) replayEventRefs.current.set(i, el); else replayEventRefs.current.delete(i); }}
                                    onMouseEnter={() => setHoverEventIdx(i)}
                                    onMouseLeave={() => setHoverEventIdx(null)}
                                    style={{
                                      background: isReplayEv ? "var(--accent-bg)" : isEvHover ? "var(--hover)" : "transparent",
                                      borderLeft: isReplayEv ? "3px solid var(--accent)" : "3px solid transparent",
                                      transition: "all 0.15s",
                                    }}>
                                    <td style={{ fontFamily: "var(--font-mono)", color: "var(--text-muted)", fontSize: 10, padding: "2px 6px" }}>{elapsed[i]}</td>
                                    {isEvHover ? (
                                      <>
                                        <td style={{ padding: "2px 4px" }}>
                                          <select value={ev.device} onChange={(e) => handleUpdateEvent(seq.id, i, { device: e.target.value })} style={{ padding: "0 2px", fontSize: 10, width: 50 }}>
                                            <option value="Keyboard">键盘</option><option value="Mouse">鼠标</option><option value="PS5">PS5</option><option value="Xbox">Xbox</option>
                                          </select>
                                        </td>
                                        <td style={{ padding: "2px 4px" }}>
                                          {ev.device === "Keyboard" ? (
                                            <input type="text" value={ev.key_name} list="kb-keys" onChange={(e) => handleUpdateEvent(seq.id, i, { key_name: e.target.value })} style={{ width: 60, padding: "0 2px", fontSize: 10 }} />
                                          ) : (
                                            <select value={ev.key_name} onChange={(e) => handleUpdateEvent(seq.id, i, { key_name: e.target.value })} style={{ padding: "0 2px", fontSize: 10, minWidth: 60 }}>
                                              {getKeyOptions(ev.device).map(k => <option key={k.v} value={k.v}>{k.l}</option>)}
                                            </select>
                                          )}
                                        </td>
                                        <td style={{ padding: "2px 4px" }}>
                                          <select value={ev.action} onChange={(e) => handleUpdateEvent(seq.id, i, { action: e.target.value })} style={{ padding: "0 2px", fontSize: 10, width: 55 }}>
                                            <option value="Press">按下</option><option value="Release">释放</option><option value="按下+释放">按下+释放</option>
                                          </select>
                                        </td>
                                        <td style={{ padding: "2px 4px", display: "flex", gap: 3, alignItems: "center" }}>
                                          <input type="number" value={ev.delay_ms ?? ""} placeholder="ms" onChange={(e) => handleUpdateEvent(seq.id, i, { delay_ms: e.target.value ? parseInt(e.target.value) : undefined })} style={{ width: 40, padding: "0 2px", fontSize: 10 }} />
                                          <button className="btn btn-sm" style={{ padding: "0 3px", fontSize: 9 }} onClick={() => handleDeleteEvent(seq.id, i)}>✕</button>
                                        </td>
                                      </>
                                    ) : (
                                      <>
                                        <td style={{ padding: "2px 6px", color: isReplayEv ? "var(--accent)" : undefined }}>{ev.device}</td>
                                        <td style={{ padding: "2px 6px" }}><span className="key-badge" style={{ fontSize: 10, ...(isReplayEv ? { background: "var(--accent)", color: "#fff" } : {}) }}>{ev.key_name}</span></td>
                                        <td style={{ padding: "2px 6px", color: getActionColor(ev.action), fontWeight: isReplayEv ? 600 : 400 }}>{getActionLabel(ev.action)}</td>
                                        <td style={{ padding: "2px 6px", fontFamily: "var(--font-mono)", color: "var(--text-muted)", fontSize: 10 }}>{ev.delay_ms != null ? `+${ev.delay_ms}ms` : ""}</td>
                                      </>
                                    )}
                                  </tr>
                                );
                              })}
                            </tbody>
                          </table>
                          ); })()}
                        </div>
                      </div></td></tr>
                    )}
                  </Fragment>
                );
              })}
              </tbody>
            </table>
          )}
        </div>}

        <datalist id="kb-keys">{KB_KEYS.map(k => <option key={k} value={k} />)}</datalist>

        {/* Pending save */}
        {pendingSave && !recording && (
          <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12, padding: "8px 12px", background: "var(--surface)", borderRadius: 8, border: "1px solid var(--accent)" }}>
            <span style={{ fontSize: 13, fontWeight: 500 }}>🎬 新录制 ({pendingSave.length} 事件)</span>
            <div style={{ flex: 1 }} />
            <input type="text" placeholder="序列名称（可选）" id="pending-name" style={{ width: 160, padding: "4px 8px", fontSize: 12 }} />
            <button className="btn btn-primary btn-sm" onClick={() => { const el = document.getElementById("pending-name") as HTMLInputElement; handleSavePending(el?.value); }}>💾 保存</button>
            <button className="btn btn-sm" onClick={() => { setPendingSave(null); setEvents([]); }}>丢弃</button>
          </div>
        )}

        {/* Current events replay bar */}
        {events.length > 0 && !recording && !pendingSave && (
          <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 12, padding: "8px 12px", background: "var(--surface)", borderRadius: 8 }}>
            <span style={{ fontSize: 13, fontWeight: 500 }}>当前事件 ({events.length})</span>
            <div style={{ flex: 1 }} />
            <button className="btn btn-primary btn-sm" onClick={doReplayCurrent} disabled={replayingId !== null}>{replayingId === "current" ? "回放中..." : "▶ 回放"}</button>
          </div>
        )}

        {/* Events table */}
        {displayEvents.length === 0 ? (
          <div className="empty-state">{recording && <><div className="icon">⏺</div><p>正在录制...</p></>}</div>
        ) : (
          <table className="data-table">
            <thead><tr><th>时间</th><th>设备</th><th>按键</th><th>键码</th><th>动作</th><th>延时</th></tr></thead>
            <tbody>
              {(() => { const elapsed = toElapsed(displayEvents); return displayEvents.map((e, i) => (
                <tr key={i}>
                  <td style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>{elapsed[i]}</td>
                  <td>{e.device}</td>
                  <td><span className="key-badge">{e.key_name}</span></td>
                  <td style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-secondary)" }}>0x{e.key_code.toString(16).toUpperCase().padStart(2, "0")}</td>
                  <td><span style={{ color: getActionColor(e.action) }}>{getActionLabel(e.action)}</span></td>
                  <td style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>{e.delay_ms != null ? `+${e.delay_ms}` : "-"}</td>
                </tr>
              )); })()}
            </tbody>
          </table>
        )}
        <div ref={eventsEndRef} />
      </div>
    </div>
  );
}
