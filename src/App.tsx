import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import DeviceCard, { Device } from './components/DeviceCard';
import CreateWizard from './components/CreateWizard';
import QuickActions from './components/QuickActions';
import FileExplorer from './components/FileExplorer';
import SnapshotPanel from './components/SnapshotPanel';

interface BatchResult {
  success: string[];
  failed: { id: string; error: string }[];
}

export default function App() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [fileExplorerDeviceId, setFileExplorerDeviceId] = useState<string | null>(null);
  const [snapshotDeviceId, setSnapshotDeviceId] = useState<string | null>(null);
  const [selectMode, setSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

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

  const toggleSelect = (id: string, checked: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (checked) next.add(id);
      else next.delete(id);
      return next;
    });
  };

  const toggleSelectMode = () => {
    setSelectMode((prev) => !prev);
    setSelectedIds(new Set());
  };

  const selectedList = [...selectedIds];

  const batch = async (cmd: string) => {
    if (selectedList.length === 0) return;
    try {
      const res = await invoke<BatchResult>(cmd, { ids: selectedList });
      console.log(`Batch ${cmd}:`, res);
    } catch (e) {
      console.error(`Batch ${cmd} failed:`, e);
    }
    setSelectedIds(new Set());
    setSelectMode(false);
    await loadDevices();
  };

  const batchInstallApk = async () => {
    // Use Tauri dialog to pick an APK file
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const path = await open({
        filters: [{ name: 'APK', extensions: ['apk'] }],
        multiple: false,
      });
      if (path) {
        for (const id of selectedList) {
          try {
            await invoke('install_apk', { id, apkPath: path });
          } catch (e) {
            console.error(`Failed to install APK on ${id}:`, e);
          }
        }
      }
    } catch (e) {
      console.error('Batch install APK failed:', e);
    }
    setSelectedIds(new Set());
    setSelectMode(false);
    await loadDevices();
  };

  return (
    <div className="app">
      <header>
        <h1>Enmulator</h1>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <button
            className={selectMode ? 'btn-primary' : ''}
            onClick={toggleSelectMode}
          >
            {selectMode ? '✕ Cancel' : '☐ Select'}
          </button>
          <button className="btn-primary" onClick={() => setWizardOpen(true)}>
            + New Device
          </button>
        </div>
      </header>

      <main>
        {selectedList.length > 0 && (
          <div className="batch-toolbar">
            <span className="batch-count">{selectedList.length} selected</span>
            <button className="btn-batch" onClick={() => batch('batch_start')}>▶ Start All</button>
            <button className="btn-batch" onClick={() => batch('batch_stop')}>⏹ Stop All</button>
            <button className="btn-batch" onClick={batchInstallApk}>📦 Install APK on All</button>
            <button className="btn-batch btn-batch-danger" onClick={() => batch('batch_delete')}>🗑 Delete All</button>
          </div>
        )}
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
              selectMode={selectMode}
              selected={selectedIds.has(d.id)}
              onSelect={toggleSelect}
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
              onOpenSnapshots={() => setSnapshotDeviceId(d.id)}
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

      {snapshotDeviceId && (
        <SnapshotPanel
          device_id={snapshotDeviceId}
          onClose={() => setSnapshotDeviceId(null)}
        />
      )}
    </div>
  );
}
