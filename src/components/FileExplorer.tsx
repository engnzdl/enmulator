import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

interface FileEntry {
  name: string;
  is_dir: boolean;
  size: number;
  permissions: string;
}

interface Props {
  device_id: string;
  onClose: () => void;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

export default function FileExplorer({ device_id, onClose }: Props) {
  // Device file browser state
  const [devicePath, setDevicePath] = useState('/sdcard');
  const [deviceEntries, setDeviceEntries] = useState<FileEntry[]>([]);
  const [deviceLoading, setDeviceLoading] = useState(false);
  const [deviceError, setDeviceError] = useState('');

  // Host file browser state
  const [hostPath, setHostPath] = useState('');
  const [hostEntries, setHostEntries] = useState<FileEntry[]>([]);
  const [hostLoading, setHostLoading] = useState(false);

  // Selected entries
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  const [selectedHost, setSelectedHost] = useState<string | null>(null);

  // Status message
  const [status, setStatus] = useState('');

  const showStatus = (msg: string) => {
    setStatus(msg);
    setTimeout(() => setStatus(''), 3000);
  };

  // List device directory
  const listDevice = useCallback(async (path: string) => {
    setDeviceLoading(true);
    setDeviceError('');
    try {
      const entries = await invoke<FileEntry[]>('list_files', { id: device_id, path });
      setDeviceEntries(entries);
      setDevicePath(path);
      setSelectedDevice(null);
    } catch (e: any) {
      setDeviceError(String(e));
    } finally {
      setDeviceLoading(false);
    }
  }, [device_id]);

  // Initial load
  useEffect(() => {
    listDevice(devicePath);
  }, [listDevice]);

  // List host directory using a simple approach (no direct fs access from webview)
  // We'll use the Tauri path from dialog. For now, show a static hint.
  const listHost = async (path: string) => {
    setHostLoading(true);
    try {
      // We can't browse host filesystem directly from the webview without a backend command.
      // Instead, we use the Tauri dialog to select files/folders.
      // For browsing, we'd need a host_list_files command, but we'll keep it simple:
      // The host panel shows the current path and allows picking files via dialog.
      setHostPath(path);
      setHostEntries([]);
    } finally {
      setHostLoading(false);
    }
  };

  // Navigate device
  const deviceNavUp = () => {
    if (devicePath === '/') return;
    const parent = devicePath.substring(0, devicePath.lastIndexOf('/')) || '/';
    listDevice(parent);
  };

  const deviceNavInto = (entry: FileEntry) => {
    if (!entry.is_dir) return;
    const newPath = devicePath === '/' ? `/${entry.name}` : `${devicePath}/${entry.name}`;
    listDevice(newPath);
  };

  // Browse host folder via dialog
  const browseHost = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select a folder',
      });
      if (selected && typeof selected === 'string') {
        setHostPath(selected);
        setHostEntries([]);
      }
    } catch (e) {
      console.error('Dialog error:', e);
    }
  };

  // Pick host file for upload
  const pickAndUpload = async () => {
    try {
      const selected = await open({
        multiple: false,
        title: 'Select file to upload',
      });
      if (selected && typeof selected === 'string') {
        const fileName = selected.split('/').pop() || selected.split('\\').pop() || 'file';
        const remotePath = devicePath === '/' ? `/${fileName}` : `${devicePath}/${fileName}`;
        setStatus(`Uploading ${fileName}...`);
        await invoke('push_file', {
          id: device_id,
          localPath: selected,
          remotePath,
        });
        showStatus(`Uploaded ${fileName}`);
        listDevice(devicePath);
      }
    } catch (e: any) {
      showStatus(`Upload failed: ${e}`);
    }
  };

  // Download selected device file
  const downloadSelected = async () => {
    if (!selectedDevice) return;
    const entry = deviceEntries.find((e) => e.name === selectedDevice);
    if (!entry || entry.is_dir) {
      showStatus('Select a file to download');
      return;
    }
    try {
      // Pick save location
      const savePath = await open({
        directory: true,
        multiple: false,
        title: 'Select download folder',
      });
      if (!savePath || typeof savePath !== 'string') return;

      const remotePath = devicePath === '/' ? `/${entry.name}` : `${devicePath}/${entry.name}`;
      const localPath = `${savePath}/${entry.name}`;
      setStatus(`Downloading ${entry.name}...`);
      await invoke('pull_file', {
        id: device_id,
        remotePath,
        localPath,
      });
      showStatus(`Downloaded to ${localPath}`);
    } catch (e: any) {
      showStatus(`Download failed: ${e}`);
    }
  };

  // Upload a specific host file (already have path)
  const uploadHostFile = async () => {
    if (!selectedHost) return;
    try {
      const remotePath = devicePath === '/' ? `/${selectedHost}` : `${devicePath}/${selectedHost}`;
      const localPath = hostPath ? `${hostPath}/${selectedHost}` : selectedHost;
      setStatus(`Uploading ${selectedHost}...`);
      await invoke('push_file', {
        id: device_id,
        localPath,
        remotePath,
      });
      showStatus(`Uploaded ${selectedHost}`);
      listDevice(devicePath);
    } catch (e: any) {
      showStatus(`Upload failed: ${e}`);
    }
  };

  return (
    <div className="file-explorer-overlay" onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="file-explorer">
        <div className="fe-header">
          <h2>📁 File Explorer</h2>
          <button onClick={onClose}>✕</button>
        </div>

        {status && <div className="fe-status">{status}</div>}

        <div className="fe-panels">
          {/* Host Panel */}
          <div className="fe-panel fe-host">
            <div className="fe-panel-header">
              <span>💻 Host</span>
              <button className="fe-btn" onClick={browseHost} title="Browse folder">
                📂 Browse
              </button>
            </div>
            <div className="fe-path">{hostPath || 'No folder selected'}</div>
            <div className="fe-actions">
              <button
                className="fe-btn fe-btn-primary"
                onClick={pickAndUpload}
                disabled={deviceLoading}
              >
                ⬆ Upload File
              </button>
              {selectedHost && (
                <button className="fe-btn fe-btn-primary" onClick={uploadHostFile}>
                  ⬆ Upload Selected
                </button>
              )}
            </div>
            <ul className="fe-list">
              {hostEntries.length === 0 && (
                <li className="fe-hint">
                  {hostPath
                    ? 'No files shown. Use "Browse" to select a folder, or "Upload File" to pick a file.'
                    : 'Select a folder using "Browse" above.'}
                </li>
              )}
              {hostEntries.map((entry) => (
                <li
                  key={entry.name}
                  className={`fe-item ${selectedHost === entry.name ? 'fe-selected' : ''}`}
                  onClick={() => setSelectedHost(entry.name)}
                >
                  <span className="fe-icon">{entry.is_dir ? '📁' : '📄'}</span>
                  <span className="fe-name">{entry.name}</span>
                  <span className="fe-size">{entry.is_dir ? '' : formatSize(entry.size)}</span>
                </li>
              ))}
            </ul>
          </div>

          {/* Device Panel */}
          <div className="fe-panel fe-device">
            <div className="fe-panel-header">
              <span>📱 Device</span>
              <button className="fe-btn" onClick={deviceNavUp} disabled={deviceLoading || devicePath === '/'}>
                ⬆ Up
              </button>
            </div>
            <div className="fe-path">{devicePath}</div>
            <div className="fe-actions">
              <button
                className="fe-btn fe-btn-primary"
                onClick={downloadSelected}
                disabled={!selectedDevice || deviceLoading}
              >
                ⬇ Download
              </button>
            </div>
            {deviceError && <div className="fe-error">{deviceError}</div>}
            {deviceLoading ? (
              <div className="fe-loading">Loading...</div>
            ) : (
              <ul className="fe-list">
                {deviceEntries.length === 0 && !deviceError && (
                  <li className="fe-hint">Empty directory</li>
                )}
                {deviceEntries.map((entry) => (
                  <li
                    key={entry.name}
                    className={`fe-item ${entry.is_dir ? 'fe-dir' : ''} ${selectedDevice === entry.name ? 'fe-selected' : ''}`}
                    onClick={() => {
                      setSelectedDevice(entry.name);
                      if (entry.is_dir) deviceNavInto(entry);
                    }}
                    onDoubleClick={() => {
                      if (entry.is_dir) deviceNavInto(entry);
                    }}
                  >
                    <span className="fe-icon">{entry.is_dir ? '📁' : '📄'}</span>
                    <span className="fe-name">{entry.name}</span>
                    <span className="fe-perms">{entry.permissions}</span>
                    <span className="fe-size">{entry.is_dir ? '' : formatSize(entry.size)}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
