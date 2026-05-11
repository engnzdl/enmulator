import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  device_id: string;
}

type ActionState = 'idle' | 'loading' | 'done';

interface ProxyConfig {
  host: string;
  port: number;
  enabled: boolean;
}

export default function QuickActions({ device_id }: Props) {
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
    <div className="quick-actions">
      <button
        className={stateClass('shell')}
        onClick={() => act('shell', 'adb_shell', { cmd: 'id' })}
        title="ADB Shell (id)"
      >
        🔧 Shell
      </button>
      <button
        className={stateClass('profile')}
        onClick={() => act('profile', 'apply_profile', { profile_name: 'pixel_5' })}
        title="Apply Profile"
      >
        📱 Profile
      </button>
      <button
        className={`qa-btn${recording ? ' recording' : ''}${states['record'] === 'loading' ? ' loading' : ''}${states['record'] === 'done' ? ' done' : ''}`}
        onClick={handleRecord}
        title={recording ? 'Stop Recording' : 'Start Recording'}
      >
        {recording ? '⏹ Stop' : '⏺ Record'}
      </button>
      <button
        className={stateClass('gps')}
        onClick={() => act('gps', 'gps_set', { lat: 37.7749, lon: -122.4194 })}
        title="Set GPS (San Francisco)"
      >
        📍 GPS
      </button>
      <button
        className={stateClass('log')}
        onClick={() => act('log', 'logcat_start')}
        title="Start Logcat"
      >
        📋 Log
      </button>
      <button
        className={stateClass('clipboard')}
        onClick={() => act('clipboard', 'clipboard_sync', { direction: 'get' })}
        title="Get Clipboard"
      >
        📋 Clip
      </button>

      {/* Proxy inline inputs + toggle */}
      <div className="proxy-controls" style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
        <input
          type="text"
          value={proxyHost}
          onChange={(e) => setProxyHost(e.target.value)}
          placeholder="host"
          style={{ width: 90, fontSize: 11, padding: '2px 4px' }}
          disabled={proxyEnabled}
        />
        <span>:</span>
        <input
          type="number"
          value={proxyPort}
          onChange={(e) => setProxyPort(e.target.value)}
          placeholder="port"
          style={{ width: 55, fontSize: 11, padding: '2px 4px' }}
          disabled={proxyEnabled}
        />
        <button
          className={stateClass('proxy')}
          onClick={handleProxyToggle}
          title={proxyEnabled ? 'Disable Proxy' : 'Enable Proxy'}
          style={{ fontSize: 11, padding: '2px 6px' }}
        >
          {proxyEnabled ? 'Disable' : 'Enable'}
        </button>
      </div>
    </div>
  );
}
