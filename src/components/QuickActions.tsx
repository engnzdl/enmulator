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

  const mark = (key: string, s: ActionState) => {
    setStates((prev) => ({ ...prev, [key]: s }));
    if (s === 'done') setTimeout(() => mark(key, 'idle'), 1200);
  };

  const act = async (key: string, cmd: string, args?: Record<string, unknown>) => {
    mark(key, 'loading');
    try {
      const result = await invoke(cmd, { id: device_id, ...args });
      console.log(`[${cmd}]`, result);
      mark(key, 'done');
    } catch (e: any) {
      console.error(`[${cmd}] failed:`, e?.message ?? e);
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

  return (
    <div>
      <div className="quick-actions">
        <button
          className={stateClass('shell')}
          onClick={() => {
            console.log('Shell clicked, invoking adb_shell...');
            mark('shell', 'loading');
            invoke('adb_shell', { id: device_id, cmd: 'echo "Root: $(id -u)"' })
              .then(() => mark('shell', 'done'))
              .catch(() => mark('shell', 'idle'));
          }}
          title="ADB Shell test"
        >
          <span className="qa-btn-icon">🔧</span> Shell
        </button>
        <button
          className={stateClass('profile')}
          onClick={() => act('profile', 'list_profiles')}
          title="List Profiles"
        >
          <span className="qa-btn-icon">📱</span> Profiles
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
          <span className="qa-btn-icon">🛡️</span> Emu Detection Bypass
        </button>
      </div>
    </div>
  );
}
