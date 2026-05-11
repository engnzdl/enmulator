import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  device_id: string;
  onOpenFiles?: () => void;
  rootEnabled?: boolean;
  onRootToggle?: () => void;
}

type ActionState = 'idle' | 'loading' | 'done';

export default function QuickActions({ device_id, onOpenFiles, rootEnabled, onRootToggle }: Props) {
  const [states, setStates] = useState<Record<string, ActionState>>({});
  const [gpsOpen, setGpsOpen] = useState(false);
  const [gpsLat, setGpsLat] = useState('41.0082');
  const [gpsLon, setGpsLon] = useState('28.9784');

  const mark = (key: string, s: ActionState) => {
    setStates((prev) => ({ ...prev, [key]: s }));
    if (s === 'done') setTimeout(() => mark(key, 'idle'), 1200);
  };

  const act = async (key: string, cmd: string, args?: Record<string, unknown>) => {
    mark(key, 'loading');
    try {
      await invoke(cmd, { id: device_id, ...args });
      mark(key, 'done');
    } catch (e) {
      console.error(`${cmd} failed:`, e);
      mark(key, 'idle');
    }
  };

  const handleGps = async () => {
    const lat = parseFloat(gpsLat);
    const lon = parseFloat(gpsLon);
    if (isNaN(lat) || isNaN(lon)) return;
    mark('gps', 'loading');
    try {
      await invoke('gps_set', { id: device_id, lat, lon });
      mark('gps', 'done');
      setGpsOpen(false);
    } catch {
      mark('gps', 'idle');
    }
  };

  const stateClass = (key: string) => {
    const s = states[key] || 'idle';
    return `qa-btn${s === 'loading' ? ' loading' : ''}${s === 'done' ? ' done' : ''}`;
  };

  return (
    <div>
      <div className="quick-actions">
        <button
          className={stateClass('gps')}
          onClick={() => setGpsOpen(true)}
          title="Set GPS Location"
        >
          <span className="qa-btn-icon">📍</span> GPS
        </button>

        <button className="qa-btn" onClick={onOpenFiles} title="File Explorer">
          <span className="qa-btn-icon">📁</span> Files
        </button>

        {onRootToggle !== undefined && (
          <button
            className={`qa-btn${rootEnabled ? ' recording' : ''}${states['root'] === 'loading' ? ' loading' : ''}${states['root'] === 'done' ? ' done' : ''}`}
            onClick={() => {
              mark('root', 'loading');
              onRootToggle();
              setTimeout(() => mark('root', 'done'), 2000);
            }}
            title={rootEnabled ? 'Unroot device' : 'Root device (adb root)'}
          >
            <span className="qa-btn-icon">{rootEnabled ? '🔓' : '🔒'}</span> {rootEnabled ? 'Unroot' : 'Root'}
          </button>
        )}

        <button
          className={stateClass('bypass')}
          onClick={() => act('bypass', 'bypass_detection', { id: device_id })}
          title="Hide emulator detection"
        >
          <span className="qa-btn-icon">🛡️</span> Emu Detection Bypass
        </button>
      </div>

      {/* GPS Popup */}
      {gpsOpen && (
        <div className="modal-overlay" onClick={() => setGpsOpen(false)}>
          <div className="modal" onClick={e => e.stopPropagation()} style={{ minWidth: 320 }}>
            <h2>Set GPS Location</h2>
            <label>Latitude</label>
            <input type="text" value={gpsLat} onChange={e => setGpsLat(e.target.value)} placeholder="41.0082" />
            <label>Longitude</label>
            <input type="text" value={gpsLon} onChange={e => setGpsLon(e.target.value)} placeholder="28.9784" />
            <div className="modal-actions" style={{ marginTop: 16 }}>
              <button className="btn-secondary" onClick={() => setGpsOpen(false)}>Cancel</button>
              <button className="btn-primary" onClick={handleGps} disabled={states['gps'] === 'loading'}>
                {states['gps'] === 'loading' ? 'Setting...' : 'Set Location'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
