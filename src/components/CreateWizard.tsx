import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface CreateWizardProps {
  isOpen: boolean;
  onClose: () => void;
  onCreated: () => void;
}

const PROFILES = ['pixel_8_us', 'samsung_s24_tr', 'Custom'];
const API_LEVELS = [34, 33, 31];

export default function CreateWizard({ isOpen, onClose, onCreated }: CreateWizardProps) {
  const [name, setName] = useState('');
  const [profile, setProfile] = useState(PROFILES[0]);
  const [apiLevel, setApiLevel] = useState(API_LEVELS[0]);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState('');

  if (!isOpen) return null;

  const handleCreate = async () => {
    if (!name.trim()) {
      setError('Device name is required');
      return;
    }
    setError('');
    setCreating(true);
    try {
      await invoke('create_device', { name: name.trim(), profile, apiLevel });
      setName('');
      setProfile(PROFILES[0]);
      setApiLevel(API_LEVELS[0]);
      onCreated();
      onClose();
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setCreating(false);
    }
  };

  const overlayClick = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains('modal-overlay')) {
      onClose();
    }
  };

  return (
    <div className="modal-overlay" onClick={overlayClick}>
      <div className="modal">
        <h2>New Device</h2>

        <label>Name</label>
        <input
          type="text"
          placeholder="My Device"
          value={name}
          onChange={(e) => setName(e.target.value)}
          autoFocus
        />

        <label>Profile</label>
        <select value={profile} onChange={(e) => setProfile(e.target.value)}>
          {PROFILES.map((p) => (
            <option key={p} value={p}>{p}</option>
          ))}
        </select>

        <label>API Level</label>
        <select value={apiLevel} onChange={(e) => setApiLevel(Number(e.target.value))}>
          {API_LEVELS.map((lvl) => (
            <option key={lvl} value={lvl}>API {lvl}</option>
          ))}
        </select>

        {error && <p className="error-msg">{error}</p>}

        <div className="modal-actions">
          <button onClick={onClose} disabled={creating}>Cancel</button>
          <button className="btn-primary" onClick={handleCreate} disabled={creating}>
            {creating ? 'Creating...' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  );
}
