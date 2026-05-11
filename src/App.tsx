import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import DeviceCard, { Device } from './components/DeviceCard';
import CreateWizard from './components/CreateWizard';

export default function App() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [wizardOpen, setWizardOpen] = useState(false);

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
            />
          ))
        )}
      </main>

      <CreateWizard
        isOpen={wizardOpen}
        onClose={() => setWizardOpen(false)}
        onCreated={handleCreate}
      />
    </div>
  );
}
