import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  device_id: string;
  rootEnabled?: boolean;
}

type ActionState = 'idle' | 'loading' | 'done';

export default function ProxyCard({ device_id, rootEnabled }: Props) {
  const [proxyHost, setProxyHost] = useState('10.0.2.2');
  const [proxyPort, setProxyPort] = useState('8080');
  const [proxyEnabled, setProxyEnabled] = useState(false);
  const [states, setStates] = useState<Record<string, ActionState>>({});

  const mark = (key: string, s: ActionState) => {
    setStates((prev) => ({ ...prev, [key]: s }));
    if (s === 'done') setTimeout(() => mark(key, 'idle'), 1200);
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

  const handleInstallCert = async () => {
    const key = 'cert';
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const certPath = await open({
        title: 'Select Certificate (.crt, .pem)',
        filters: [
          { name: 'Certificates', extensions: ['crt', 'pem', 'cer', 'der'] },
        ],
        multiple: false,
      });
      if (!certPath) return;

      mark(key, 'loading');
      const result = await invoke<string>('install_cert', {
        id: device_id,
        certPath: certPath as string,
      });
      console.log('cert installed:', result);
      mark(key, 'done');
    } catch (e) {
      console.error('install cert failed:', e);
      mark(key, 'idle');
    }
  };

  const stateClass = (key: string) => {
    const s = states[key] || 'idle';
    return `qa-btn${s === 'loading' ? ' loading' : ''}${s === 'done' ? ' done' : ''}`;
  };

  const needsRoot = !rootEnabled;

  return (
    <div className="proxy-card">
      <div className="proxy-card-row">
        <input
          type="text"
          value={proxyHost}
          onChange={(e) => setProxyHost(e.target.value)}
          placeholder="host"
          disabled={proxyEnabled}
          className="proxy-input"
        />
        <span>:</span>
        <input
          type="number"
          value={proxyPort}
          onChange={(e) => setProxyPort(e.target.value)}
          placeholder="port"
          disabled={proxyEnabled}
          className="proxy-input"
        />
        <button
          className={`qa-btn${proxyEnabled ? ' recording' : ''}${states['proxy'] === 'loading' ? ' loading' : ''}${states['proxy'] === 'done' ? ' done' : ''}`}
          onClick={handleProxyToggle}
          title={proxyEnabled ? 'Disable Proxy' : 'Enable Proxy'}
        >
          {proxyEnabled ? 'Disable' : 'Enable'}
        </button>
      </div>

      <div className="proxy-card-row">
        <button
          className={stateClass('cert')}
          onClick={handleInstallCert}
          title={needsRoot ? 'Device must be rooted to install system certificates' : 'Install CA certificate to /system/etc/security/cacerts/'}
          disabled={needsRoot}
        >
          <span className="qa-btn-icon">📜</span> Install Cert
        </button>
        {needsRoot && (
          <span className="proxy-hint">Root required</span>
        )}
      </div>
    </div>
  );
}
