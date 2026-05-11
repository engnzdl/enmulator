import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

interface Config {
  sdk_path: string | null;
  devices_dir: string;
  api_server_port: number;
  default_headless: boolean;
  auto_start_api: boolean;
  default_api_level: number;
  default_abi: string;
  default_tag: string;
}

interface Props {
  onClose: () => void;
}

const API_LEVELS = [
  { value: 35, label: 'Android 15 (API 35)' },
  { value: 34, label: 'Android 14 (API 34)' },
  { value: 33, label: 'Android 13 (API 33)' },
  { value: 32, label: 'Android 12L (API 32)' },
  { value: 31, label: 'Android 12 (API 31)' },
  { value: 30, label: 'Android 11 (API 30)' },
];

const ABI_OPTIONS = ['x86_64', 'arm64-v8a', 'x86', 'armeabi-v7a'];
const TAG_OPTIONS = ['google_apis', 'google_apis_playstore', 'google_atd', 'android-tv', 'default'];

export default function SettingsPanel({ onClose }: Props) {
  const [config, setConfig] = useState<Config | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<Config>('get_config').then(setConfig).catch(console.error);
  }, []);

  const handleSave = async () => {
    if (!config) return;
    setSaving(true);
    try {
      await invoke('update_config', {
        sdkPath: config.sdk_path,
        devicesDir: config.devices_dir,
        apiServerPort: config.api_server_port,
        defaultHeadless: config.default_headless,
        autoStartApi: config.auto_start_api,
        defaultApiLevel: config.default_api_level,
        defaultAbi: config.default_abi,
        defaultTag: config.default_tag,
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error('Failed to save config:', e);
    }
    setSaving(false);
  };

  const handleBrowseSdk = async () => {
    try {
      const path = await open({ directory: true, title: 'Select Android SDK Folder' });
      if (path && config) {
        setConfig({ ...config, sdk_path: path as string });
      }
    } catch (e) {
      console.error('Dialog error:', e);
    }
  };

  if (!config) return <div className="settings-loading">Loading...</div>;

  return (
    <div className="settings-overlay">
      <div className="settings-panel animate-in">
        <div className="settings-header">
          <h2>⚙ Settings</h2>
          <button className="settings-close-btn" onClick={onClose}>✕</button>
        </div>

        <div className="settings-body">
          {/* SDK Path */}
          <div className="settings-group">
            <div className="settings-group-title">Android SDK</div>
            <div className="settings-field">
              <label>SDK Path</label>
              <div className="settings-field-row">
                <input
                  type="text"
                  value={config.sdk_path || ''}
                  onChange={(e) => setConfig({ ...config, sdk_path: e.target.value || null })}
                  placeholder="Auto-detected: ~/Library/Android/sdk"
                  className="settings-input"
                />
                <button className="settings-browse-btn" onClick={handleBrowseSdk}>Browse</button>
              </div>
              <span className="settings-hint">Leave empty for auto-detection</span>
            </div>
          </div>

          {/* Default Device Settings */}
          <div className="settings-group">
            <div className="settings-group-title">Default Device Settings</div>
            <div className="settings-field">
              <label>API Level</label>
              <select
                value={config.default_api_level}
                onChange={(e) => setConfig({ ...config, default_api_level: parseInt(e.target.value) })}
                className="settings-select"
              >
                {API_LEVELS.map((l) => (
                  <option key={l.value} value={l.value}>{l.label}</option>
                ))}
              </select>
            </div>
            <div className="settings-field">
              <label>ABI</label>
              <select
                value={config.default_abi}
                onChange={(e) => setConfig({ ...config, default_abi: e.target.value })}
                className="settings-select"
              >
                {ABI_OPTIONS.map((a) => (<option key={a} value={a}>{a}</option>))}
              </select>
            </div>
            <div className="settings-field">
              <label>Tag</label>
              <select
                value={config.default_tag}
                onChange={(e) => setConfig({ ...config, default_tag: e.target.value })}
                className="settings-select"
              >
                {TAG_OPTIONS.map((t) => (<option key={t} value={t}>{t.replace(/_/g, ' ')}</option>))}
              </select>
            </div>
          </div>

          {/* Behavior */}
          <div className="settings-group">
            <div className="settings-group-title">Behavior</div>
            <div className="settings-field settings-field-toggle">
              <label>Start devices headless by default</label>
              <button
                className={`settings-toggle ${config.default_headless ? 'on' : 'off'}`}
                onClick={() => setConfig({ ...config, default_headless: !config.default_headless })}
              >
                {config.default_headless ? 'ON' : 'OFF'}
              </button>
            </div>
            <div className="settings-field settings-field-toggle">
              <label>Auto-start API server</label>
              <button
                className={`settings-toggle ${config.auto_start_api ? 'on' : 'off'}`}
                onClick={() => setConfig({ ...config, auto_start_api: !config.auto_start_api })}
              >
                {config.auto_start_api ? 'ON' : 'OFF'}
              </button>
            </div>
          </div>

          {/* Advanced */}
          <div className="settings-group">
            <div className="settings-group-title">Advanced</div>
            <div className="settings-field">
              <label>Devices Directory</label>
              <input
                type="text"
                value={config.devices_dir}
                onChange={(e) => setConfig({ ...config, devices_dir: e.target.value })}
                className="settings-input"
              />
              <span className="settings-hint">Where AVD data is stored (requires restart)</span>
            </div>
            <div className="settings-field">
              <label>API Server Port</label>
              <input
                type="number"
                value={config.api_server_port}
                onChange={(e) => setConfig({ ...config, api_server_port: parseInt(e.target.value) || 8080 })}
                className="settings-input settings-input-sm"
              />
            </div>
          </div>
        </div>

        <div className="settings-footer">
          {saved && <span className="settings-saved">✓ Saved</span>}
          <button className="btn-primary" onClick={handleSave} disabled={saving}>
            {saving ? 'Saving...' : 'Save Settings'}
          </button>
        </div>
      </div>
    </div>
  );
}
