import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  device_id: string;
  device_port?: number;
  onOpenFiles?: () => void;
  rootEnabled?: boolean;
  writableSystem?: boolean;
  onRootToggle?: () => void;
  onWritableToggle?: () => void;
}

type ActionState = 'idle' | 'loading' | 'done' | 'error';

export default function QuickActions({
  device_id,
  device_port,
  onOpenFiles,
  rootEnabled,
  writableSystem,
  onRootToggle,
  onWritableToggle,
}: Props) {
  const [states, setStates] = useState<Record<string, ActionState>>({});
  const [stateMsg, setStateMsg] = useState<Record<string, string>>({});
  const [gpsOpen, setGpsOpen] = useState(false);
  const [gpsLat, setGpsLat] = useState('41.0082');
  const [gpsLon, setGpsLon] = useState('28.9784');
  const [magiskResult, setMagiskResult] = useState<string | null>(null);

  const mark = (key: string, s: ActionState, msg = '') => {
    setStates((prev) => ({ ...prev, [key]: s }));
    setStateMsg((prev) => ({ ...prev, [key]: msg }));
    if (s === 'done') setTimeout(() => mark(key, 'idle'), 2000);
    if (s === 'error') setTimeout(() => mark(key, 'idle'), 4000);
  };

  const act = async (key: string, cmd: string, args?: Record<string, unknown>) => {
    mark(key, 'loading');
    try {
      const result = await invoke<string>(cmd, { id: device_id, ...args });
      mark(key, 'done', typeof result === 'string' ? result : '');
    } catch (e: any) {
      mark(key, 'error', e?.message ?? String(e));
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
      mark('gps', 'error');
    }
  };

  const handleMagiskRoot = async () => {
    mark('magisk', 'loading');
    try {
      const result = await invoke<string>('root_with_magisk', { id: device_id });
      setMagiskResult(result);
      mark('magisk', 'done');
    } catch (e: any) {
      setMagiskResult(`Error: ${e?.message ?? String(e)}`);
      mark('magisk', 'error');
    }
  };

  const btnClass = (key: string, activeClass = '') => {
    const s = states[key] || 'idle';
    return [
      'qa-btn',
      s === 'loading' ? 'loading' : '',
      s === 'done' ? 'done' : '',
      s === 'error' ? 'recording' : '',
      activeClass,
    ].filter(Boolean).join(' ');
  };

  const adbSerial = device_port ? `emulator-${device_port}` : null;

  return (
    <div>
      <div className="quick-actions">

        {/* GPS */}
        <button className={btnClass('gps')} onClick={() => setGpsOpen(true)} title="Mock GPS location">
          <span className="qa-btn-icon">📍</span> GPS
        </button>

        {/* File Explorer */}
        <button className="qa-btn" onClick={onOpenFiles} title="File Explorer">
          <span className="qa-btn-icon">📁</span> Files
        </button>

        {/* Root (adb root) */}
        {onRootToggle !== undefined && (
          <button
            className={btnClass('root', rootEnabled ? 'recording' : '')}
            onClick={() => {
              mark('root', 'loading');
              onRootToggle();
              setTimeout(() => mark('root', 'done'), 2500);
            }}
            title={rootEnabled
              ? 'Unroot (adb unroot) — removes root from ADB daemon'
              : 'Root via adb root — requires google_apis (non-Play Store) image'}
          >
            <span className="qa-btn-icon">{rootEnabled ? '🔓' : '🔒'}</span>
            {rootEnabled ? 'Unroot' : 'ADB Root'}
          </button>
        )}

        {/* Writable System */}
        {onWritableToggle !== undefined && (
          <button
            className={btnClass('writable', writableSystem ? 'recording' : '')}
            onClick={() => {
              mark('writable', 'loading');
              onWritableToggle();
              setTimeout(() => mark('writable', 'done'), 3000);
            }}
            title={writableSystem
              ? 'Switch system disk back to read-only'
              : 'Make /system writable (adb root + adb remount). Runtime only — resets on reboot.'}
          >
            <span className="qa-btn-icon">{writableSystem ? '💾' : '🔐'}</span>
            {writableSystem ? 'System RW' : 'System RO'}
          </button>
        )}

        {/* Magisk Root */}
        <button
          className={btnClass('magisk')}
          onClick={handleMagiskRoot}
          title="Root system image with Magisk (rootAVD). Device must be stopped. One-time operation per system image."
        >
          <span className="qa-btn-icon">🧲</span> Magisk Root
        </button>

        {/* Install Magisk App */}
        <button
          className={btnClass('magisk_apk')}
          onClick={() => act('magisk_apk', 'install_magisk_apk')}
          title="Install Magisk Manager APK on the running device (after Magisk Root)"
        >
          <span className="qa-btn-icon">📦</span> Install Magisk App
        </button>

        {/* Emulator Detection Bypass */}
        <button
          className={btnClass('bypass')}
          onClick={() => act('bypass', 'bypass_detection')}
          title="Set build props to look like a production device"
        >
          <span className="qa-btn-icon">🛡️</span> Bypass Detection
        </button>

      </div>

      {/* ADB Connection Info */}
      {adbSerial && (
        <div className="adb-info">
          <span className="adb-info-label">ADB Serial</span>
          <code className="adb-info-serial">{adbSerial}</code>
          <span className="adb-info-hint">→ use with <code>adb -s {adbSerial} shell</code></span>
        </div>
      )}

      {/* Action feedback messages */}
      {Object.entries(stateMsg).map(([key, msg]) =>
        msg && states[key] !== 'idle' ? (
          <div key={key} className={`qa-msg ${states[key] === 'error' ? 'qa-msg-error' : 'qa-msg-ok'}`}>
            {msg}
          </div>
        ) : null
      )}

      {/* Magisk result modal */}
      {magiskResult && (
        <div className="modal-overlay" onClick={() => setMagiskResult(null)}>
          <div className="modal" onClick={e => e.stopPropagation()} style={{ maxWidth: 480 }}>
            <h2>Magisk Root</h2>
            <pre style={{ whiteSpace: 'pre-wrap', fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: 16 }}>
              {magiskResult}
            </pre>
            <div className="modal-actions">
              <button className="btn-primary" onClick={() => setMagiskResult(null)}>OK</button>
            </div>
          </div>
        </div>
      )}

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
