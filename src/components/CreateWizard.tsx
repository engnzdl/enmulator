import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface SystemImage {
  api_level: number;
  abi: string;
  tag: string;
  description: string;
}

interface CreateWizardProps {
  isOpen: boolean;
  onClose: () => void;
  onCreated: () => void;
}

const PROFILES = ['pixel_8_us', 'samsung_s24_tr', 'Custom'];

export default function CreateWizard({ isOpen, onClose, onCreated }: CreateWizardProps) {
  const [name, setName] = useState('');
  const [profile, setProfile] = useState(PROFILES[0]);
  const [apiLevel, setApiLevel] = useState<number | null>(null);
  const [abi, setAbi] = useState('');
  const [tag, setTag] = useState('');
  const [images, setImages] = useState<SystemImage[]>([]);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState('');
  const [statusMsg, setStatusMsg] = useState('');

  // Fetch available system images when wizard opens
  useEffect(() => {
    if (!isOpen) return;
    setError('');
    setStatusMsg('');
    setLoading(true);
    invoke<SystemImage[]>('list_available_images_cmd')
      .then((imgs) => {
        setImages(imgs);
        if (imgs.length > 0) {
          const first = imgs[0];
          setApiLevel(first.api_level);
          setAbi(first.abi);
          setTag(first.tag);
        }
      })
      .catch((e: any) => setError(e?.message ?? String(e)))
      .finally(() => setLoading(false));
  }, [isOpen]);

  // Unique API levels from images, sorted descending
  const apiLevels = useMemo(() => {
    const levels = [...new Set(images.map((img) => img.api_level))];
    levels.sort((a, b) => b - a);
    return levels;
  }, [images]);

  // Available ABIs for selected API level
  const abis = useMemo(() => {
    if (apiLevel === null) return [];
    const abis = [
      ...new Set(
        images
          .filter((img) => img.api_level === apiLevel)
          .map((img) => img.abi)
      ),
    ];
    return abis;
  }, [images, apiLevel]);

  // Available tags for selected API level + ABI
  const tags = useMemo(() => {
    if (apiLevel === null || !abi) return [];
    const tags = [
      ...new Set(
        images
          .filter((img) => img.api_level === apiLevel && img.abi === abi)
          .map((img) => img.tag)
      ),
    ];
    return tags;
  }, [images, apiLevel, abi]);

  // Current image description
  const currentDescription = useMemo(() => {
    if (apiLevel === null || !abi || !tag) return '';
    const img = images.find(
      (i) => i.api_level === apiLevel && i.abi === abi && i.tag === tag
    );
    return img?.description ?? '';
  }, [images, apiLevel, abi, tag]);

  // When API level changes, reset ABI and tag to first available
  const handleApiChange = (level: number) => {
    setApiLevel(level);
    const matching = images.filter((img) => img.api_level === level);
    if (matching.length > 0) {
      setAbi(matching[0].abi);
      const tagMatches = matching.filter((img) => img.abi === matching[0].abi);
      setTag(tagMatches.length > 0 ? tagMatches[0].tag : '');
    } else {
      setAbi('');
      setTag('');
    }
  };

  // When ABI changes, reset tag to first available
  const handleAbiChange = (newAbi: string) => {
    setAbi(newAbi);
    const matching = images.filter(
      (img) => img.api_level === apiLevel && img.abi === newAbi
    );
    setTag(matching.length > 0 ? matching[0].tag : '');
  };

  const handleCreate = async () => {
    if (!name.trim()) {
      setError('Device name is required');
      return;
    }
    if (apiLevel === null || !abi || !tag) {
      setError('Please select a complete system image (API, ABI, and variant)');
      return;
    }
    setError('');
    setStatusMsg('');

    const pkg = `system-images;android-${apiLevel};${tag};${abi}`;

    // First, try to install the system image (sdkmanager skips if already installed)
    setInstalling(true);
    setStatusMsg(`Checking system image: ${pkg}...`);
    try {
      await invoke('install_system_image_cmd', { package: pkg });
      setStatusMsg('System image ready.');
    } catch (e: any) {
      setInstalling(false);
      setError(`Failed to download system image: ${e?.message ?? String(e)}`);
      return;
    }
    setInstalling(false);

    // Now create the AVD
    setCreating(true);
    setStatusMsg('Creating device...');
    try {
      await invoke('create_device', {
        name: name.trim(),
        profile,
        apiLevel,
        abi,
        tag,
      });
      setName('');
      setProfile(PROFILES[0]);
      setStatusMsg('');
      onCreated();
      onClose();
    } catch (e: any) {
      setError(e?.message ?? String(e));
      setStatusMsg('');
    } finally {
      setCreating(false);
    }
  };

  const overlayClick = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains('modal-overlay')) {
      onClose();
    }
  };

  if (!isOpen) return null;

  const busy = loading || creating || installing;

  return (
    <div className="modal-overlay" onClick={overlayClick}>
      <div className="modal">
        <h2>New Device</h2>

        {loading && <p className="status-msg">Loading available system images...</p>}

        <label>Name</label>
        <input
          type="text"
          placeholder="My Device"
          value={name}
          onChange={(e) => setName(e.target.value)}
          autoFocus
          disabled={busy}
        />

        <label>Profile</label>
        <select value={profile} onChange={(e) => setProfile(e.target.value)} disabled={busy}>
          {PROFILES.map((p) => (
            <option key={p} value={p}>{p}</option>
          ))}
        </select>

        <label>API Level</label>
        <select
          value={apiLevel ?? ''}
          onChange={(e) => handleApiChange(Number(e.target.value))}
          disabled={busy || apiLevels.length === 0}
        >
          {apiLevels.length === 0 && <option value="">-- None available --</option>}
          {apiLevels.map((lvl) => (
            <option key={lvl} value={lvl}>API {lvl}</option>
          ))}
        </select>

        <label>ABI (Architecture)</label>
        <select
          value={abi}
          onChange={(e) => handleAbiChange(e.target.value)}
          disabled={busy || abis.length === 0}
        >
          {abis.length === 0 && <option value="">-- Select API first --</option>}
          {abis.map((a) => (
            <option key={a} value={a}>{a}</option>
          ))}
        </select>

        <label>Variant (Tag)</label>
        <select
          value={tag}
          onChange={(e) => setTag(e.target.value)}
          disabled={busy || tags.length === 0}
        >
          {tags.length === 0 && <option value="">-- Select ABI first --</option>}
          {tags.map((t) => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>

        {currentDescription && (
          <p className="image-desc">{currentDescription}</p>
        )}

        {statusMsg && <p className="status-msg">{statusMsg}</p>}
        {error && <p className="error-msg">{error}</p>}

        <div className="modal-actions">
          <button onClick={onClose} disabled={creating || installing}>Cancel</button>
          <button className="btn-primary" onClick={handleCreate} disabled={busy || apiLevel === null}>
            {installing ? 'Downloading image...' : creating ? 'Creating...' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  );
}
