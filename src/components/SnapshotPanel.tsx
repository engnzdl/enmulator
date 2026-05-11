import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  device_id: string;
  onClose: () => void;
}

type BtnState = 'idle' | 'loading' | 'done';

export default function SnapshotPanel({ device_id, onClose }: Props) {
  const [snapshots, setSnapshots] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [status, setStatus] = useState('');
  const [newName, setNewName] = useState('');
  const [btnStates, setBtnStates] = useState<Record<string, BtnState>>({});

  const markBtn = (key: string, s: BtnState) => {
    setBtnStates((prev) => ({ ...prev, [key]: s }));
    if (s === 'done') setTimeout(() => markBtn(key, 'idle'), 1200);
  };

  const btnClass = (key: string) => {
    const s = btnStates[key] || 'idle';
    return `fe-btn${s === 'loading' ? ' fe-btn-loading' : ''}${s === 'done' ? ' fe-btn-done' : ''}`;
  };

  const showStatus = (msg: string) => {
    setStatus(msg);
    setTimeout(() => setStatus(''), 3000);
  };

  const loadSnapshots = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const list = await invoke<string[]>('list_snapshots_cmd', { id: device_id });
      setSnapshots(list);
    } catch (e: any) {
      setError(String(e));
      setSnapshots([]);
    } finally {
      setLoading(false);
    }
  }, [device_id]);

  useEffect(() => {
    loadSnapshots();
  }, [loadSnapshots]);

  const handleSave = async () => {
    const name = newName.trim();
    if (!name) {
      showStatus('Enter a snapshot name');
      return;
    }
    markBtn('save', 'loading');
    try {
      await invoke('save_snapshot_cmd', { id: device_id, name });
      markBtn('save', 'done');
      showStatus(`Snapshot "${name}" saved`);
      setNewName('');
      await loadSnapshots();
    } catch (e: any) {
      showStatus(`Save failed: ${e}`);
      markBtn('save', 'idle');
    }
  };

  const handleLoad = async (name: string) => {
    markBtn(`load-${name}`, 'loading');
    try {
      await invoke('load_snapshot_cmd', { id: device_id, name });
      markBtn(`load-${name}`, 'done');
      showStatus(`Snapshot "${name}" loaded`);
    } catch (e: any) {
      showStatus(`Load failed: ${e}`);
      markBtn(`load-${name}`, 'idle');
    }
  };

  const handleDelete = async (name: string) => {
    if (!confirm(`Delete snapshot "${name}"?`)) return;
    markBtn(`del-${name}`, 'loading');
    try {
      await invoke('delete_snapshot_cmd', { id: device_id, name });
      markBtn(`del-${name}`, 'done');
      showStatus(`Snapshot "${name}" deleted`);
      await loadSnapshots();
    } catch (e: any) {
      showStatus(`Delete failed: ${e}`);
      markBtn(`del-${name}`, 'idle');
    }
  };

  return (
    <div className="file-explorer-overlay" onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="file-explorer" style={{ maxWidth: 520 }}>
        <div className="fe-header">
          <h2>📸 Snapshots</h2>
          <button onClick={onClose}>✕</button>
        </div>

        {status && <div className="fe-status">{status}</div>}
        {error && <div className="fe-error">{error}</div>}

        {/* New snapshot input */}
        <div style={{ display: 'flex', gap: 8, padding: '8px 12px', alignItems: 'center' }}>
          <input
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="Snapshot name..."
            style={{ flex: 1, padding: '4px 8px', fontSize: 13 }}
            onKeyDown={(e) => { if (e.key === 'Enter') handleSave(); }}
          />
          <button
            className={btnClass('save')}
            onClick={handleSave}
            disabled={btnStates['save'] === 'loading'}
          >
            💾 Save
          </button>
          <button
            className="fe-btn"
            onClick={loadSnapshots}
            disabled={loading}
            title="Refresh"
          >
            🔄
          </button>
        </div>

        {/* Snapshot list */}
        <div style={{ padding: '0 12px 12px' }}>
          {loading ? (
            <div className="fe-loading">Loading snapshots...</div>
          ) : snapshots.length === 0 ? (
            <div className="fe-hint">No snapshots. Save one above.</div>
          ) : (
            <ul className="fe-list" style={{ maxHeight: 300, overflowY: 'auto' }}>
              {snapshots.map((snap) => (
                <li key={snap} className="fe-item" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <span>
                    <span className="fe-icon">📸</span>
                    <span className="fe-name">{snap}</span>
                  </span>
                  <span style={{ display: 'flex', gap: 4 }}>
                    <button
                      className={btnClass(`load-${snap}`)}
                      onClick={() => handleLoad(snap)}
                      disabled={btnStates[`load-${snap}`] === 'loading'}
                      title="Load this snapshot"
                    >
                      ▶ Load
                    </button>
                    <button
                      className={btnClass(`del-${snap}`)}
                      onClick={() => handleDelete(snap)}
                      disabled={btnStates[`del-${snap}`] === 'loading'}
                      title="Delete this snapshot"
                      style={{ color: '#e74c3c' }}
                    >
                      🗑
                    </button>
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
