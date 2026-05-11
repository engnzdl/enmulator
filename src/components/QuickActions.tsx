import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  device_id: string;
  onOpenFiles?: () => void;
  onOpenSnapshots?: () => void;
  rootEnabled?: boolean;
  onRootToggle?: () => void;
}

type ActionState = 'idle' | 'loading' | 'done';

export default function QuickActions({ device_id, onOpenFiles, onOpenSnapshots, rootEnabled, onRootToggle }: Props) {
  const [states, setStates] = useState<Record<string, ActionState>>({});
  const [recording, setRecording] = useState(false);
  const [proxyHost, setProxyHost] = useState('10.0.2.2');
  const [proxyPort, setProxyPort] = useState('8080');
  const [proxyEnabled, setProxyEnabled] = useState(false);

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

  const stateClass = (key: string) => {
    const s = states[key] || 'idle';
    return `qa-btn${s === 'loading' ? ' loading' : ''}${s === 'done' ? ' done' : ''}`;
  };

  const handleRecord = async () => {
    const key = 'record';
    mark(key, 'loading');
    try {
      if (recording) {
        await invoke('stop_screen_record', { id: device_id });
      } else {
        await invoke('start_screen_record', { id: device_id });
      }
      setRecording(!recording);
      mark(key, 'done');
    } catch (e) {
      console.error('record toggle failed:', e);
      mark(key, 'idle');
    }
  };

  const handleProxyToggle = async () => {
    const key = 'proxy';
    const enabled = !proxyEnabled;
    const portNum = parseInt(proxyPort, 10) || 8080;
    mark(key, 'loading');
    try {
      await invoke('set_device_proxy', {
        id: device_id,
        host: proxyHost || '10.0.2.2',
        port: portNum,
        enabled,
      });
      setProxyEnabled(enabled);
      mark(key, 'done');
    } catch (e) {
      console.error('proxy toggle failed:', e);
      mark(key, 'idle');
    }
  };

  return (
    <div>
      <div className="quick-actions">
        <button
          className={stateClass('shell')}
          onClick={() => act('shell', 'adb_shell', { cmd: 'id' })}
          title="ADB Shell (id)"
        >
          <span className="qa-btn-icon">🔧</span> Shell
        </button>
        <button
          className={stateClass('profile')}
          onClick={() => act('profile', 'apply_profile', { profile_name: 'pixel_5' })}
          title="Apply Profile"
        >
          <span className="qa-btn-icon">📱</span> Profile
        </button>
        <button
          className={`qa-btn${recording ? ' recording' : ''}${states['record'] === 'loading' ? ' loading' : ''}${states['record'] === 'done' ? ' done' : ''}`}
          onClick={handleRecord}
          title={recording ? 'Stop Recording' : 'Start Recording'}
        >
          <span className="qa-btn-icon">{recording ? '⏹' : '⏺'}</span> {recording ? 'Stop' : 'Record'}
        </button>
        <button
          className={stateClass('gps')}
          onClick={() => act('gps', 'gps_set', { lat: 37.7749, lon: -122.4194 })}
          title="Set GPS (San Francisco)"
        >
          <span className="qa-btn-icon">📍</span> GPS
        </button>
        <button
          className={stateClass('log')}
          onClick={() => act('log', 'logcat_start')}
          title="Start Logcat"
        >
          <span className="qa-btn-icon">📋</span> Log
        </button>
        <button
          className={stateClass('clipboard')}
          onClick={() => act('clipboard', 'clipboard_sync', { direction: 'get' })}
          title="Get Clipboard"
        >
          <span className="qa-btn-icon">📋</span> Clip
        </button>

        <button
          className="qa-btn"
          onClick={onOpenFiles}
          title="File Explorer"
        >
          <span className="qa-btn-icon">📁</span> Files
        </button>

        <button
          className="qa-btn"
          onClick={onOpenSnapshots}
          title="Snapshots"
        >
          <span className="qa-btn-icon">📸</span> Snapshots
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
          className={`qa-btn${states['bypass'] === 'loading' ? ' loading' : ''}${states['bypass'] === 'done' ? ' done' : ''}`}
          onClick={() => act('bypass', 'bypass_detection', { id: device_id })}
          title="Hide emulator detection (build.props)"
        >
          <span className="qa-btn-icon">🛡️</span> Bypass
        </button>
      </div>

      {/* Proxy inline inputs + toggle */}
      <div className="proxy-controls">
        <input
          type="text"
          value={proxyHost}
          onChange={(e) => setProxyHost(e.target.value)}
          placeholder="host"
          disabled={proxyEnabled}
        />
        <span>:</span>
        <input
          type="number"
          value={proxyPort}
          onChange={(e) => setProxyPort(e.target.value)}
          placeholder="port"
          disabled={proxyEnabled}
        />
        <button
          className={stateClass('proxy')}
          onClick={handleProxyToggle}
          title={proxyEnabled ? 'Disable Proxy' : 'Enable Proxy'}
          style={{ fontSize: 11, padding: '3px 8px' }}
        >
          {proxyEnabled ? 'Disable' : 'Enable'}
        </button>
      </div>
    </div>
  );
}
