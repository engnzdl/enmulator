import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import DeviceCard, { Device } from './components/DeviceCard';
import CreateWizard from './components/CreateWizard';
import QuickActions from './components/QuickActions';
import FileExplorer from './components/FileExplorer';

export default function App() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [fileExplorerDeviceId, setFileExplorerDeviceId] = useState<string | null>(null);

  const loadDevices = useCallback(async () => {
    try {
      const list = await invoke<Device[]>('list_devices');
      setDevices(list);
    } catch (e) {
      console.error('Failed to list devices:', e);
    }
  }, []);

  useEffect(() => { loadDevices(); }, [loadDevices]);

  const handleStart = async (id: string) => {
    await invoke('start_device', { id, headless: false });
    await loadDevices();
  };

  const handleStop = async (id: string) => {
    await invoke('stop_device', { id });
    await loadDevices();
  };

  const handleDelete = async (id: string) => {
    await invoke('delete_device', { id });
    await loadDevices();
  };

  const handleClone = async (id: string) => {
    const name = prompt('Clone name?');
    if (name) {
      await invoke('clone_device', { sourceId: id, targetName: name });
      await loadDevices();
    }
  };

  const handleDropApk = async (id: string, apkPath: string) => {
    try {
      console.log(`Installing APK: ${apkPath} on device ${id}`);
      await invoke('install_apk', { id, apkPath });
      await loadDevices();
    } catch (e) {
      console.error('Failed to install APK:', e);
    }
  };

  const handleCreate = async () => {
    setWizardOpen(false);
    await loadDevices();
  };

  return (
    <div className="app">
      <header>
        <h1>Enmulator</h1>
        <button className="btn-primary" onClick={() => setWizardOpen(true)}>
          + New Device
        </button>
      </header>

      <main>
        {devices.length === 0 ? (
          <p className="empty-state">No devices yet. Click "+ New Device" to create one.</p>
        ) : (
          devices.map((d) => (
            <DeviceCard
              key={d.id}
              device={d}
              onStart={handleStart}
              onStop={handleStop}
              onDelete={handleDelete}
              onClone={handleClone}
              onDropApk={handleDropApk}
            />
          ))
        )}
      </main>

      {devices.length > 0 && (
        <footer className="toolbar">
          <h2 className="toolbar-title">Quick Actions</h2>
          {devices.map((d) => (
            <QuickActions
              key={d.id}
              device_id={d.id}
              onOpenFiles={() => setFileExplorerDeviceId(d.id)}
            />
          ))}
        </footer>
      )}

      <CreateWizard
        isOpen={wizardOpen}
        onClose={() => setWizardOpen(false)}
        onCreated={handleCreate}
      />

      {fileExplorerDeviceId && (
        <FileExplorer
          device_id={fileExplorerDeviceId}
          onClose={() => setFileExplorerDeviceId(null)}
        />
      )}
    </div>
  );
}
