import { useEffect, useState } from "react";
import { api, type MappingRule } from "../api";
import { v4 as uuidv4 } from "uuid";
import KeyCapture, { vkToName, buttonToName } from "../components/KeyCapture";

export default function Mappings() {
  const [mappings, setMappings] = useState<MappingRule[]>([]);
  const [editing, setEditing] = useState<MappingRule | null>(null);
  const [isAdding, setIsAdding] = useState(false);

  useEffect(() => { loadMappings(); }, []);

  const loadMappings = async () => {
    const rules = await api.getMappings();
    setMappings(rules);
  };

  const handleAdd = () => {
    setEditing({
      id: uuidv4(),
      name: "",
      is_enabled: true,
      priority: mappings.length,
      source: { device: "keyboard", primary_key: 0, modifiers: [], mode: "press" },
      targets: [{ action_type: "key_click", output_device: "keyboard", output_key: 0, output_modifiers: [], duration_ms: 0 }],
      conditions: [],
      advanced: { delay_before_ms: 0, delay_between_ms: 0, repeat_count: 0, repeat_interval_ms: 0, consume_input: true },
    });
    setIsAdding(true);
  };

  const handleSave = async () => {
    if (!editing) return;
    const rule = { ...editing };
    if (!rule.name.trim()) {
      const src = rule.source.modifiers.length > 0
        ? [...rule.source.modifiers, rule.source.primary_key].map(vkToName).join("+")
        : vkToName(rule.source.primary_key);
      const tgt = rule.targets.map((t) => vkToName(t.output_key)).join(", ");
      rule.name = `${src} → ${tgt}`;
    }
    if (isAdding) {
      await api.addMapping(rule);
    } else {
      await api.updateMapping(rule);
    }
    setEditing(null);
    setIsAdding(false);
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

  return (
    <div className="page">
      <div className="page-header">
        <h2>映射规则</h2>
        <div className="actions">
          <button className="btn btn-primary" onClick={handleAdd}>+ 添加规则</button>
        </div>
      </div>
      <div className="page-body">
        {mappings.length === 0 ? (
          <div className="empty-state">
            <div className="icon">⌨</div>
            <p>暂无映射规则，点击上方按钮添加</p>
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
                <th style={{ width: 80 }}>操作</th>
              </tr>
            </thead>
            <tbody>
              {mappings.map((m) => (
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
                        {" + "}<span className="key-badge">{vkToName(mod)}</span>
                      </span>
                    ))}
                  </td>
                  <td>
                    {m.targets.map((t, i) => (
                      <span key={i}>
                        {i > 0 && " → "}
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
                    <button className="btn btn-sm" onClick={() => { setEditing(m); setIsAdding(false); }}>编辑</button>
                    <button className="btn btn-sm btn-danger" onClick={() => handleDelete(m.id)}>删除</button>
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
                <label>规则名称（可选，留空自动生成）</label>
                <input
                  type="text"
                  value={editing.name}
                  onChange={(e) => setEditing({ ...editing, name: e.target.value })}
                  placeholder="例如：跳跃、瞄准"
                />
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>源设备</label>
                  <select
                    value={editing.source.device}
                    onChange={(e) => setEditing({
                      ...editing,
                      source: { ...editing.source, device: e.target.value, primary_key: 0, modifiers: [] }
                    })}
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
                <label>源按键</label>
                <KeyCapture
                  value={{ primary_key: editing.source.primary_key, modifiers: editing.source.modifiers }}
                  onChange={(keys) => setEditing({
                    ...editing,
                    source: { ...editing.source, primary_key: keys.primary_key, modifiers: keys.modifiers }
                  })}
                  device={editing.source.device}
                />
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>目标设备</label>
                  <select
                    value={editing.targets[0]?.output_device || "keyboard"}
                    onChange={(e) => {
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
                <label>目标按键</label>
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
