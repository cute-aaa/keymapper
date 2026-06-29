import { useEffect, useState, useRef } from "react";
import { api, type MappingRule } from "../api";
import { v4 as uuidv4 } from "uuid";
import KeyCapture, { vkToName, buttonToName } from "../components/KeyCapture";

export default function Mappings() {
  const [mappings, setMappings] = useState<MappingRule[]>([]);
  const [editing, setEditing] = useState<MappingRule | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [search, setSearch] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);

  const loadMappings = async () => {
    const rules = await api.getMappings();
    setMappings(rules);
  };

  useEffect(() => { loadMappings(); }, []);

  // Ctrl+F to focus search
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "f") {
        e.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const handleAdd = () => {
    const lastSourceDevice = localStorage.getItem("km_last_source_device") || "keyboard";
    const lastTargetDevice = localStorage.getItem("km_last_target_device") || "keyboard";
    setEditing({
      id: uuidv4(),
      name: "",
      is_enabled: true,
      priority: mappings.length,
      source: { device: lastSourceDevice, primary_key: 0, modifiers: [], mode: "press", combo_keys: [] },
      targets: [{ action_type: "key_click", output_device: lastTargetDevice, output_key: 0, output_modifiers: [], duration_ms: 0 }],
      conditions: [],
      advanced: { delay_before_ms: 0, delay_between_ms: 0, repeat_count: 0, repeat_interval_ms: 0, consume_input: true },
      sound_feedback: false,
      vibration_feedback: false,
      vibration_intensity: 128,
      vibration_duration_ms: 200,
    });
    setIsAdding(true);
  };

  const handleSave = async () => {
    if (!editing) return;
    const rule = { ...editing };
    // Auto-generate name if empty
    if (!rule.name.trim()) {
      const srcParts: string[] = [];
      if (rule.source.modifiers.length > 0) {
        srcParts.push(...rule.source.modifiers.map(v => rule.source.device === "keyboard" ? vkToName(v) : buttonToName(v, rule.source.device)));
      }
      srcParts.push(rule.source.device === "keyboard" ? vkToName(rule.source.primary_key) : buttonToName(rule.source.primary_key, rule.source.device));
      if (rule.source.combo_keys && rule.source.combo_keys.length > 0) {
        srcParts.push(...rule.source.combo_keys.map(v => rule.source.device === "keyboard" ? vkToName(v) : buttonToName(v, rule.source.device)));
      }
      const src = srcParts.join("+");
      const tgtParts: string[] = [];
      for (const t of rule.targets) {
        const parts: string[] = [];
        if (t.output_modifiers) parts.push(...t.output_modifiers.map(vkToName));
        parts.push(t.output_device === "keyboard" ? vkToName(t.output_key) : buttonToName(t.output_key, t.output_device));
        tgtParts.push(parts.join("+"));
      }
      rule.name = `${src} → ${tgtParts.join(", ")}`;
    }
    if (isAdding) {
      await api.addMapping(rule);
    } else {
      await api.updateMapping(rule);
    }
    setEditing(null);
    loadMappings();
  };

  const handleDelete = async (id: string) => {
    if (confirm("确定删除此映射规则？")) {
      await api.removeMapping(id);
      loadMappings();
    }
  };

  const handleToggle = async (id: string, enabled: boolean) => {
    await api.toggleMapping(id, enabled);
    loadMappings();
  };

  const isGamepad = editing?.source.device === "xbox_gamepad" || editing?.source.device === "ps5_gamepad";

  // Filter mappings by search
  const filtered = search.trim()
    ? mappings.filter(m => {
        const q = search.toLowerCase();
        const srcName = m.source.device === "keyboard"
          ? vkToName(m.source.primary_key)
          : buttonToName(m.source.primary_key, m.source.device);
        return m.name.toLowerCase().includes(q) || srcName.toLowerCase().includes(q);
      })
    : mappings;

  return (
    <div className="page">
      <div className="page-header">
        <h2>映射规则</h2>
        <div className="actions">
          <button className="btn btn-primary" onClick={handleAdd}>+ 添加规则</button>
        </div>
      </div>
      <div className="page-body">
        {mappings.length > 0 && (
          <div style={{ marginBottom: 12 }}>
            <input
              ref={searchRef}
              type="text"
              placeholder="搜索规则名称或按键..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              style={{ width: "100%", padding: "6px 10px", fontSize: 13 }}
            />
          </div>
        )}
        {filtered.length === 0 ? (
          <div className="empty-state">
            <div className="icon">⌨</div>
            <p>{mappings.length === 0 ? "暂无映射规则，点击上方按钮添加" : "无匹配规则"}</p>
          </div>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th style={{ width: 40 }}>启用</th>
                <th>名称</th>
                <th>源按键</th>
                <th>目标按键</th>
                <th>设备</th>
                <th style={{ width: 120 }}>操作</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((m) => (
                <tr key={m.id}>
                  <td>
                    <button
                      className={`toggle ${m.is_enabled ? "on" : ""}`}
                      onClick={() => handleToggle(m.id, !m.is_enabled)}
                    />
                  </td>
                  <td>{m.name || "未命名"}</td>
                  <td>
                    <span className="key-badge">{
                      m.source.device === "keyboard"
                        ? vkToName(m.source.primary_key)
                        : buttonToName(m.source.primary_key, m.source.device)
                    }</span>
                    {m.source.modifiers.map((mod, i) => (
                      <span key={i}>
                        {" + "}<span className="key-badge">{
                          m.source.device === "keyboard" ? vkToName(mod) : buttonToName(mod, m.source.device)
                        }</span>
                      </span>
                    ))}
                    {(m.source.combo_keys || []).map((ck, i) => (
                      <span key={"c"+i}>
                        {" + "}<span className="key-badge">{
                          m.source.device === "keyboard" ? vkToName(ck) : buttonToName(ck, m.source.device)
                        }</span>
                      </span>
                    ))}
                  </td>
                  <td>
                    {m.targets.map((t, i) => (
                      <span key={i}>
                        {i > 0 && " → "}
                        {t.output_modifiers.map((mod, j) => (
                          <span key={"m"+j}>
                            <span className="key-badge">{vkToName(mod)}</span>{" + "}
                          </span>
                        ))}
                        <span className="key-badge">{
                          t.output_device === "keyboard"
                            ? vkToName(t.output_key)
                            : buttonToName(t.output_key, t.output_device)
                        }</span>
                      </span>
                    ))}
                  </td>
                  <td>{m.source.device === "keyboard" ? "键盘" : m.source.device === "mouse" ? "鼠标" : m.source.device === "xbox_gamepad" ? "Xbox" : "PS5"}</td>
                  <td>
                    <div style={{ display: "inline-flex", gap: 4 }}>
                      <button className="btn btn-sm" onClick={() => { setEditing(m); setIsAdding(false); }}>编辑</button>
                      <button className="btn btn-sm btn-danger" onClick={() => handleDelete(m.id)}>删除</button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {editing && (
        <div className="modal-overlay" onClick={() => setEditing(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()} style={{ width: 560 }}>
            <div className="modal-header">
              <h3>{isAdding ? "添加规则" : "编辑规则"}</h3>
              <button className="btn-icon" onClick={() => setEditing(null)}>✕</button>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label>规则名称（留空自动生成）</label>
                <input
                  type="text"
                  value={editing.name}
                  onChange={(e) => setEditing({ ...editing, name: e.target.value })}
                  placeholder="留空则自动生成：源按键 → 目标按键"
                />
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>源设备</label>
                  <select
                    value={editing.source.device}
                    onChange={(e) => {
                      localStorage.setItem("km_last_source_device", e.target.value);
                      setEditing({
                        ...editing,
                        source: { ...editing.source, device: e.target.value, primary_key: 0, modifiers: [], combo_keys: [] }
                      });
                    }}
                  >
                    <option value="keyboard">键盘</option>
                    <option value="mouse">鼠标</option>
                    <option value="xbox_gamepad">Xbox 手柄</option>
                    <option value="ps5_gamepad">PS5 手柄</option>
                  </select>
                </div>
                <div className="form-group">
                  <label>触发模式</label>
                  <select
                    value={editing.source.mode}
                    onChange={(e) => setEditing({
                      ...editing,
                      source: { ...editing.source, mode: e.target.value }
                    })}
                  >
                    <option value="press">按下</option>
                    <option value="release">释放</option>
                    <option value="hold">长按</option>
                    <option value="tap">短按</option>
                  </select>
                </div>
              </div>
              <div className="form-group">
                <label>源按键 {isGamepad && "(支持组合键：先按住一个键不放，再按另一个键)"}</label>
                <KeyCapture
                  value={{ primary_key: editing.source.primary_key, modifiers: editing.source.modifiers, combo_keys: editing.source.combo_keys || [] }}
                  onChange={(keys) => setEditing({
                    ...editing,
                    source: { ...editing.source, primary_key: keys.primary_key, modifiers: keys.modifiers, combo_keys: keys.combo_keys || [] }
                  })}
                  device={editing.source.device}
                  allowCombo={isGamepad}
                />
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>目标设备</label>
                  <select
                    value={editing.targets[0]?.output_device || "keyboard"}
                    onChange={(e) => {
                      localStorage.setItem("km_last_target_device", e.target.value);
                      const targets = editing.targets.length > 0
                        ? [{ ...editing.targets[0], output_device: e.target.value, output_key: 0, output_modifiers: [] }]
                        : [{ action_type: "key_click", output_device: e.target.value, output_key: 0, output_modifiers: [], duration_ms: 0 }];
                      setEditing({ ...editing, targets });
                    }}
                  >
                    <option value="keyboard">键盘</option>
                    <option value="mouse">鼠标</option>
                    <option value="xbox_gamepad">Xbox 手柄</option>
                    <option value="ps5_gamepad">PS5 手柄</option>
                  </select>
                </div>
              </div>
              <div className="form-group">
                <label>目标按键（支持组合键：如 Ctrl+A、Alt+F1）</label>
                <KeyCapture
                  value={{ primary_key: editing.targets[0]?.output_key || 0, modifiers: editing.targets[0]?.output_modifiers || [] }}
                  onChange={(keys) => {
                    const targets = editing.targets.length > 0
                      ? [{ ...editing.targets[0], output_key: keys.primary_key, output_modifiers: keys.modifiers }]
                      : [{ action_type: "key_click", output_device: "keyboard", output_key: keys.primary_key, output_modifiers: keys.modifiers, duration_ms: 0 }];
                    setEditing({ ...editing, targets });
                  }}
                  device={editing.targets[0]?.output_device || "keyboard"}
                />
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>触发前延迟 (ms)</label>
                  <input
                    type="number"
                    value={editing.advanced.delay_before_ms}
                    onChange={(e) => setEditing({
                      ...editing,
                      advanced: { ...editing.advanced, delay_before_ms: parseInt(e.target.value) || 0 }
                    })}
                  />
                </div>
                <div className="form-group">
                  <label>消费原始输入</label>
                  <button
                    className={`toggle ${editing.advanced.consume_input ? "on" : ""}`}
                    onClick={() => setEditing({
                      ...editing,
                      advanced: { ...editing.advanced, consume_input: !editing.advanced.consume_input }
                    })}
                  />
                </div>
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>🔊 声音提示</label>
                  <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                    <button
                      className={`toggle ${editing.sound_feedback ? "on" : ""}`}
                      onClick={() => setEditing({ ...editing, sound_feedback: !editing.sound_feedback })}
                    />
                    <button className="btn btn-sm" onClick={() => api.testSound()}>测试</button>
                  </div>
                </div>
                {isGamepad && (
                  <div className="form-group">
                    <label>📳 震动提示</label>
                    <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                      <button
                        className={`toggle ${editing.vibration_feedback ? "on" : ""}`}
                        onClick={() => setEditing({ ...editing, vibration_feedback: !editing.vibration_feedback })}
                      />
                      <button className="btn btn-sm" onClick={() => api.testVibration(editing.vibration_intensity || 128, editing.vibration_duration_ms || 200)}>测试</button>
                    </div>
                  </div>
                )}
              </div>
              {editing.vibration_feedback && isGamepad && (
                <div className="form-row">
                  <div className="form-group">
                    <label>震动强度 ({editing.vibration_intensity})</label>
                    <input
                      type="range" min="1" max="255"
                      value={editing.vibration_intensity}
                      onChange={(e) => setEditing({ ...editing, vibration_intensity: parseInt(e.target.value) || 128 })}
                    />
                  </div>
                  <div className="form-group">
                    <label>震动时长 (ms)</label>
                    <input
                      type="number" min="50" max="2000"
                      value={editing.vibration_duration_ms}
                      onChange={(e) => setEditing({ ...editing, vibration_duration_ms: parseInt(e.target.value) || 200 })}
                    />
                  </div>
                </div>
              )}
            </div>
            <div className="modal-footer">
              <button className="btn" onClick={() => setEditing(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleSave}>
                {isAdding ? "添加" : "保存"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
