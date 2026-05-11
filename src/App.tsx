import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import DeviceCard, { Device } from './components/DeviceCard';
import CreateWizard from './components/CreateWizard';
import QuickActions from './components/QuickActions';
import ProxyCard from './components/ProxyCard';
import IdentityCard from './components/IdentityCard';
import FileExplorer from './components/FileExplorer';
import SettingsPanel from './components/SettingsPanel';
interface BatchResult {
  success: string[];
  failed: { id: string; error: string }[];
}

export default function App() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [fileExplorerDeviceId, setFileExplorerDeviceId] = useState<string | null>(null);
  const [selectMode, setSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [settingsOpen, setSettingsOpen] = useState(false);

  const loadDevices = useCallback(async () => {
    try {
      const list = await invoke<Device[]>('list_devices');
      setDevices(list);
    } catch (e) {
      console.error('Failed to list devices:', e);
    }
  }, []);

  useEffect(() => { loadDevices(); }, [loadDevices]);

  // Poll running devices to detect manual emulator closes
  useEffect(() => {
    const interval = setInterval(() => {
      devices.forEach(async (d) => {
        if (d.status === 'running') {
          try {
            const alive = await invoke<boolean>('check_device_alive', { id: d.id });
            if (!alive) loadDevices();
          } catch { /* ignore */ }
        }
      });
    }, 5000);
    return () => clearInterval(interval);
  }, [devices, loadDevices]);

  // Auto-select first device if nothing selected and devices exist
  useEffect(() => {
    if (devices.length > 0 && !selectedDeviceId) {
      // Check if current selection still exists
      const exists = devices.find(d => d.id === selectedDeviceId);
      if (!exists) {
        setSelectedDeviceId(devices[0].id);
      }
    }
    if (devices.length === 0) {
      setSelectedDeviceId(null);
    }
  }, [devices, selectedDeviceId]);

  const selectedDevice = devices.find(d => d.id === selectedDeviceId) ?? null;

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
    if (selectedDeviceId === id) setSelectedDeviceId(null);
    await loadDevices();
  };

  const handleClone = async (id: string) => {
    const dev = devices.find(d => d.id === id);
    const name = dev ? `${dev.display_name} (clone)` : `Clone of ${id}`;
    try {
      await invoke('clone_device', { sourceId: id, targetName: name });
      await loadDevices();
    } catch (e) {
      console.error('clone failed:', e);
    }
  };

  const handleRootToggle = async (id: string) => {
    try {
      const result = await invoke<string>('toggle_root', { id });
      console.log('Root toggle:', result);
    } catch (e) {
      console.error('Root toggle failed:', e);
    }
    await loadDevices();
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
      {/* ── Glass Header ── */}
      <header className="app-header">
        <div className="app-header-brand">
          <div className="app-header-logo">E</div>
          <h1>Enmulator</h1>
        </div>
        <div className="app-header-actions">
          <button className="btn-icon" onClick={() => setSettingsOpen(true)} title="Settings">
            ⚙
          </button>
          <button
            className={selectMode ? 'btn-primary' : 'btn-ghost'}
            onClick={toggleSelectMode}
          >
            {selectMode ? 'Cancel' : 'Select'}
          </button>
          <button className="btn-primary" onClick={() => setWizardOpen(true)}>
            + New Device
          </button>
        </div>
      </header>

      {/* ── Content: Sidebar + Panel ── */}
      <div className="app-content">
        {/* ── Left Sidebar ── */}
        <aside className="app-sidebar">
          <div className="sidebar-header">
            <h2>Devices</h2>
            <span className="sidebar-count">{devices.length}</span>
          </div>

          <div className="sidebar-device-list">
            {devices.length === 0 ? (
              <div className="sidebar-empty">
                <div className="sidebar-empty-icon">📱</div>
                <p>No devices yet.<br />Click "+ New Device" to create one.</p>
              </div>
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
                  isActive={d.id === selectedDeviceId}
                  onSelect={toggleSelect}
                  onClick={() => { if (!selectMode) setSelectedDeviceId(d.id); }}
                />
              ))
            )}
          </div>

          {/* Batch toolbar */}
          {selectedList.length > 0 && (
            <div className="sidebar-batch-toolbar">
              <span className="batch-count-badge">{selectedList.length} selected</span>
              <button className="btn-batch" onClick={() => batch('batch_start')}>Start</button>
              <button className="btn-batch" onClick={() => batch('batch_stop')}>Stop</button>
              <button className="btn-batch" onClick={batchInstallApk}>Install APK</button>
              <button className="btn-batch btn-batch-danger" onClick={() => batch('batch_delete')}>Delete</button>
            </div>
          )}
        </aside>

        {/* ── Right Panel ── */}
        <main className="app-panel">
          {!selectedDevice ? (
            <div className="panel-empty animate-in">
              <div className="panel-empty-icon">📱</div>
              <h3>Select a device</h3>
              <p>Choose a device from the sidebar to view details, manage snapshots, browse files, and control the emulator.</p>
            </div>
          ) : (
            <div className="panel-device animate-in" key={selectedDevice.id}>
              {/* Device Hero */}
              <div className="panel-device-hero">
                <div className="panel-device-icon">📱</div>
                <div className="panel-device-info">
                  <div className="panel-device-name">
                    {selectedDevice.display_name}
                    <span className={`status-badge ${selectedDevice.status === 'running' ? 'running' : 'stopped'}`}>
                      <span className="status-badge-dot" />
                      {selectedDevice.status}
                    </span>
                  </div>
                  <div className="panel-device-meta">
                    <span>📱 {selectedDevice.fingerprint_profile || selectedDevice.profile || 'custom'}</span>
                    <span>🔧 Android {(() => {
                      const v: Record<number, string> = {35:'15',34:'14',33:'13',32:'12L',31:'12',30:'11',29:'10',28:'9'};
                      const ver = v[selectedDevice.api_level];
                      return ver ? `${ver} (API ${selectedDevice.api_level})` : `API ${selectedDevice.api_level}`;
                    })()}</span>
                    <span>🔌 Port {selectedDevice.port || '-'}</span>
                    {selectedDevice.root_enabled && <span>🔓 rooted</span>}
                  </div>
                </div>
                <div className="panel-device-actions">
                  {selectedDevice.status === 'running' ? (
                    <button className="btn-secondary" onClick={() => handleStop(selectedDevice.id)}>Stop</button>
                  ) : (
                    <button className="btn-primary" onClick={() => handleStart(selectedDevice.id)}>Start</button>
                  )}
                  <button className="btn-secondary" onClick={() => handleClone(selectedDevice.id)}>Clone</button>
                  <button className="btn-danger" onClick={() => handleDelete(selectedDevice.id)}>Delete</button>
                </div>
              </div>

              {/* Quick Actions Section */}
              <div className="panel-section">
                <div className="panel-section-title">Quick Actions</div>
                <QuickActions
                  device_id={selectedDevice.id}
                  onOpenFiles={() => setFileExplorerDeviceId(selectedDevice.id)}
                  rootEnabled={selectedDevice.root_enabled}
                  onRootToggle={() => handleRootToggle(selectedDevice.id)}
                />
              </div>

              {/* Identity Section */}
              <div className="panel-section">
                <div className="panel-section-title">Device Identity</div>
                <IdentityCard
                  device_id={selectedDevice.id}
                  deviceStatus={selectedDevice.status}
                  profileName={selectedDevice.fingerprint_profile ?? null}
                />
              </div>

              {/* Proxy Section */}
              <div className="panel-section">
                <div className="panel-section-title">Proxy</div>
                <ProxyCard
                  device_id={selectedDevice.id}
                  rootEnabled={selectedDevice.root_enabled}
                />
              </div>
            </div>
          )}
        </main>
      </div>

      {/* ── Modals ── */}
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

      {settingsOpen && (
        <SettingsPanel onClose={() => setSettingsOpen(false)} />
      )}

    </div>
  );
}
