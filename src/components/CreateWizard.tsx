import { useState, useEffect, useMemo, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

interface SystemImage {
  api_level: number;
  abi: string;
  tag: string;
  description: string;
}

interface FingerprintProfile {
  name: string;
  brand: string;
  model: string;
  manufacturer: string;
  device: string;
  fingerprint: string;
  dpi: number;
  resolution_w: number;
  resolution_h: number;
}

interface CreateWizardProps {
  isOpen: boolean;
  onClose: () => void;
  onCreated: () => void;
}

const STEPS = ['Device Profile', 'System Image'];

export default function CreateWizard({ isOpen, onClose, onCreated }: CreateWizardProps) {
  // ── Step tracking ──
  const [step, setStep] = useState(0);

  // ── Step 0: Name + Fingerprint Profile (merged) ──
  const [name, setName] = useState('');
  const [profiles, setProfiles] = useState<FingerprintProfile[]>([]);
  const [selectedProfile, setSelectedProfile] = useState('');
  const [showNewProfileForm, setShowNewProfileForm] = useState(false);
  // New profile form fields
  const [newProfile, setNewProfile] = useState<FingerprintProfile>({
    name: '',
    brand: '',
    model: '',
    manufacturer: '',
    device: '',
    fingerprint: '',
    dpi: 420,
    resolution_w: 1080,
    resolution_h: 1920,
  });

  // ── Step 1: System Image ──
  const [images, setImages] = useState<SystemImage[]>([]);
  const [apiLevel, setApiLevel] = useState<number | null>(null);
  const [abi, setAbi] = useState('');
  const [tag, setTag] = useState('');

  // ── UI state ──
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState('');
  const [statusMsg, setStatusMsg] = useState('');

  // ── Download progress ──
  const [downloadLines, setDownloadLines] = useState<string[]>([]);
  const [downloadPercent, setDownloadPercent] = useState<number | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  // ── Fetch data when wizard opens ──
  useEffect(() => {
    if (!isOpen) return;
    setError('');
    setStatusMsg('');
    setStep(0);
    setShowNewProfileForm(false);
    setSelectedProfile('');
    setLoading(true);
    setDownloadLines([]);
    setDownloadPercent(null);
    // Clean up any lingering event listener
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }

    // Generate random device name
    const adjectives = ['Alpha', 'Beta', 'Delta', 'Gamma', 'Neo', 'Optima', 'Prime', 'Quantum', 'Swift', 'Ultra'];
    const nouns = ['Device', 'Emu', 'Instance', 'Lab', 'Node', 'Runner', 'Sandbox', 'Station', 'Test', 'VM'];
    const rand = (arr: string[]) => arr[Math.floor(Math.random() * arr.length)];
    setName(`${rand(adjectives)} ${rand(nouns)}`);

    Promise.all([
      invoke<SystemImage[]>('list_available_images_cmd'),
      invoke<FingerprintProfile[]>('list_profiles').catch(() => [] as FingerprintProfile[]),
    ])
      .then(([imgs, fpProfiles]) => {
        setImages(imgs);
        setProfiles(fpProfiles || []);
        if (imgs.length > 0) {
          // Prefer google_apis + x86_64, highest API first
          const preferred = imgs.find(i => i.tag === 'google_apis' && i.abi === 'x86_64')
            || imgs.find(i => i.tag === 'google_apis')
            || imgs.find(i => i.abi === 'x86_64')
            || imgs[0];
          setApiLevel(preferred.api_level);
          setAbi(preferred.abi);
          setTag(preferred.tag);
        }
      })
      .catch((e: any) => setError(e?.message ?? String(e)))
      .finally(() => setLoading(false));
  }, [isOpen]);

  // ── Derived: unique API levels, ABIs, tags ──
  const apiLevels = useMemo(() => {
    const levels = [...new Set(images.map((img) => img.api_level))];
    levels.sort((a, b) => b - a);
    return levels;
  }, [images]);

  const abis = useMemo(() => {
    if (apiLevel === null) return [];
    const list = [...new Set(
      images.filter((img) => img.api_level === apiLevel).map((img) => img.abi)
    )];
    // x86_64 first
    list.sort((a, b) => {
      if (a === 'x86_64') return -1;
      if (b === 'x86_64') return 1;
      return a.localeCompare(b);
    });
    return list;
  }, [images, apiLevel]);

  const tags = useMemo(() => {
    if (apiLevel === null || !abi) return [];
    const list = [...new Set(
      images.filter((img) => img.api_level === apiLevel && img.abi === abi).map((img) => img.tag)
    )];
    // google_apis first
    list.sort((a, b) => {
      if (a === 'google_apis') return -1;
      if (b === 'google_apis') return 1;
      return a.localeCompare(b);
    });
    return list;
  }, [images, apiLevel, abi]);

  const currentDescription = useMemo(() => {
    if (apiLevel === null || !abi || !tag) return '';
    const img = images.find(
      (i) => i.api_level === apiLevel && i.abi === abi && i.tag === tag
    );
    return img?.description ?? '';
  }, [images, apiLevel, abi, tag]);

  // ── Handlers ──
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

  const handleAbiChange = (newAbi: string) => {
    setAbi(newAbi);
    const matching = images.filter(
      (img) => img.api_level === apiLevel && img.abi === newAbi
    );
    setTag(matching.length > 0 ? matching[0].tag : '');
  };

  const handleNewProfileChange = (field: keyof FingerprintProfile, value: string | number) => {
    setNewProfile((prev) => ({ ...prev, [field]: value }));
  };

  const handleSaveNewProfile = async () => {
    if (!newProfile.name.trim() || !newProfile.brand.trim() || !newProfile.model.trim()) {
      setError('Profile name, brand, and model are required');
      return;
    }
    setError('');
    try {
      const saved = await invoke<FingerprintProfile>('create_profile', { profile: newProfile });
      setProfiles((prev) => [...prev, saved]);
      setSelectedProfile(saved.name);
      setShowNewProfileForm(false);
      // Reset form
      setNewProfile({
        name: '',
        brand: '',
        model: '',
        manufacturer: '',
        device: '',
        fingerprint: '',
        dpi: 420,
        resolution_w: 1080,
        resolution_h: 1920,
      });
    } catch (e: any) {
      setError(e?.message ?? String(e));
    }
  };

  const handleDeleteProfile = async (profileName: string) => {
    try {
      await invoke('delete_profile', { name: profileName });
      setProfiles((prev) => prev.filter((p) => p.name !== profileName));
      if (selectedProfile === profileName) {
        setSelectedProfile('');
      }
    } catch (e: any) {
      setError(e?.message ?? String(e));
    }
  };

  // ── Final create ──
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
    setDownloadLines([]);
    setDownloadPercent(null);

    const pkg = `system-images;android-${apiLevel};${tag};${abi}`;

    // Set up progress listener before starting install
    const unlisten = await listen<{ package: string; line: string }>('download-progress', (event) => {
      setDownloadLines((prev) => {
        const next = [...prev, event.payload.line];
        // Try to detect a percentage from the line
        const pctMatch = event.payload.line.match(/(\d+)%/);
        if (pctMatch) {
          setDownloadPercent(parseInt(pctMatch[1], 10));
        }
        return next;
      });
    });
    unlistenRef.current = unlisten;

    // Install system image
    setInstalling(true);
    setStatusMsg(`Downloading system image: ${pkg}...`);
    try {
      await invoke('install_system_image_cmd', { package: pkg });
      setStatusMsg('System image ready.');
      setDownloadPercent(100);
    } catch (e: any) {
      setInstalling(false);
      // Clean up listener on error
      unlisten();
      unlistenRef.current = null;
      setError(`Failed to download system image: ${e?.message ?? String(e)}`);
      return;
    }
    // Clean up listener after success
    unlisten();
    unlistenRef.current = null;
    setInstalling(false);

    // Create AVD with fingerprint profile (resolution/DPI set from profile)
    setCreating(true);
    setStatusMsg('Creating device...');
    try {
      await invoke('create_device', {
        name: name.trim(),
        apiLevel,
        abi,
        tag,
        fingerprintProfile: selectedProfile || null,
      });
      setName('');
      setSelectedProfile('');
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

  const nextStep = () => setStep((s) => Math.min(s + 1, STEPS.length - 1));
  const prevStep = () => setStep((s) => Math.max(s - 1, 0));

  // ── Selected profile details for display ──
  const selectedProfileObj = profiles.find((p) => p.name === selectedProfile);

  if (!isOpen) return null;

  const busy = loading || creating || installing;

  return (
    <div className="modal-overlay" onClick={overlayClick}>
      <div className="modal" style={{ width: '520px' }}>
        <h2>New Device</h2>

        {/* ── Step Indicators ── */}
        <div className="wizard-steps">
          {STEPS.map((label, i) => (
            <div
              key={label}
              className={`wizard-step ${i === step ? 'active' : ''} ${i < step ? 'done' : ''}`}
              onClick={() => { if (i < step) setStep(i); }}
            >
              <span className="wizard-step-num">{i < step ? '✓' : i + 1}</span>
              <span className="wizard-step-label">{label}</span>
            </div>
          ))}
        </div>

        {loading && (
          <p className="status-msg">
            <span className="status-spinner" /> Loading...
          </p>
        )}

        {/* ═══════════════ STEP 0: Device Name + Fingerprint Profile ═══════════════ */}
        {step === 0 && (
          <div className="wizard-step-body">
            <label>Device Name</label>
            <input
              type="text"
              placeholder="My Device"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
              disabled={busy}
            />

            <label>Device Profile (sets resolution &amp; DPI)</label>

            {/* Existing profiles list */}
            {profiles.length > 0 ? (
              <div className="profile-list">
                {profiles.map((p) => (
                  <div
                    key={p.name}
                    className={`profile-card ${selectedProfile === p.name ? 'profile-card-selected' : ''}`}
                    onClick={() => { setSelectedProfile(p.name); setShowNewProfileForm(false); }}
                  >
                    <div className="profile-card-main">
                      <span className="profile-card-name">{p.name}</span>
                      <span className="profile-card-meta">
                        {p.brand} — {p.model} — {p.resolution_w}×{p.resolution_h} @ {p.dpi}dpi
                      </span>
                    </div>
                    <div className="profile-card-actions">
                      {selectedProfile === p.name && <span className="profile-check">✓</span>}
                      <button
                        className="btn-ghost profile-delete-btn"
                        onClick={(e) => { e.stopPropagation(); handleDeleteProfile(p.name); }}
                        title="Delete profile"
                      >
                        ✕
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="image-desc">No device profiles yet. Create one below.</div>
            )}

            {/* Selected profile details */}
            {selectedProfileObj && (
              <div className="image-desc profile-details">
                <strong>{selectedProfileObj.name}</strong>
                <div className="profile-details-grid">
                  <span>Brand: {selectedProfileObj.brand}</span>
                  <span>Model: {selectedProfileObj.model}</span>
                  <span>Manufacturer: {selectedProfileObj.manufacturer}</span>
                  <span>Device: {selectedProfileObj.device}</span>
                  <span>DPI: {selectedProfileObj.dpi}</span>
                  <span>Resolution: {selectedProfileObj.resolution_w}×{selectedProfileObj.resolution_h}</span>
                </div>
                <div className="profile-fp">Fingerprint: {selectedProfileObj.fingerprint}</div>
              </div>
            )}

            {/* Toggle New Profile Form */}
            {!showNewProfileForm ? (
              <button
                className="btn-secondary"
                style={{ marginTop: '8px', width: '100%' }}
                onClick={() => setShowNewProfileForm(true)}
                disabled={busy}
              >
                + Create New Profile
              </button>
            ) : (
              <div className="new-profile-form">
                <h4 style={{ marginBottom: '12px', fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-secondary)' }}>
                  New Device Profile
                </h4>
                <label>Profile Name</label>
                <input
                  type="text"
                  placeholder="e.g. Galaxy S24"
                  value={newProfile.name}
                  onChange={(e) => handleNewProfileChange('name', e.target.value)}
                />
                <div className="profile-form-row">
                  <div>
                    <label>Brand</label>
                    <input
                      type="text"
                      placeholder="samsung"
                      value={newProfile.brand}
                      onChange={(e) => handleNewProfileChange('brand', e.target.value)}
                    />
                  </div>
                  <div>
                    <label>Model</label>
                    <input
                      type="text"
                      placeholder="SM-S928B"
                      value={newProfile.model}
                      onChange={(e) => handleNewProfileChange('model', e.target.value)}
                    />
                  </div>
                </div>
                <div className="profile-form-row">
                  <div>
                    <label>Manufacturer</label>
                    <input
                      type="text"
                      placeholder="samsung"
                      value={newProfile.manufacturer}
                      onChange={(e) => handleNewProfileChange('manufacturer', e.target.value)}
                    />
                  </div>
                  <div>
                    <label>Device Code</label>
                    <input
                      type="text"
                      placeholder="e3q"
                      value={newProfile.device}
                      onChange={(e) => handleNewProfileChange('device', e.target.value)}
                    />
                  </div>
                </div>
                <label>Fingerprint</label>
                <input
                  type="text"
                  placeholder="samsung/e3qxxx/e3q:14/UP1A..."
                  value={newProfile.fingerprint}
                  onChange={(e) => handleNewProfileChange('fingerprint', e.target.value)}
                />
                <div className="profile-form-row">
                  <div>
                    <label>DPI</label>
                    <input
                      type="number"
                      value={newProfile.dpi}
                      onChange={(e) => handleNewProfileChange('dpi', parseInt(e.target.value) || 0)}
                    />
                  </div>
                  <div>
                    <label>Resolution (W×H)</label>
                    <div style={{ display: 'flex', gap: '6px' }}>
                      <input
                        type="number"
                        placeholder="1080"
                        value={newProfile.resolution_w}
                        onChange={(e) => handleNewProfileChange('resolution_w', parseInt(e.target.value) || 0)}
                        style={{ flex: 1 }}
                      />
                      <span style={{ color: 'var(--text-muted)', alignSelf: 'center' }}>×</span>
                      <input
                        type="number"
                        placeholder="1920"
                        value={newProfile.resolution_h}
                        onChange={(e) => handleNewProfileChange('resolution_h', parseInt(e.target.value) || 0)}
                        style={{ flex: 1 }}
                      />
                    </div>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: '8px', marginTop: '12px' }}>
                  <button className="btn-primary" onClick={handleSaveNewProfile} disabled={busy}>
                    Save Profile
                  </button>
                  <button className="btn-secondary" onClick={() => setShowNewProfileForm(false)}>
                    Cancel
                  </button>
                </div>
              </div>
            )}
          </div>
        )}

        {/* ═══════════════ STEP 1: System Image ═══════════════ */}
        {step === 1 && (
          <div className="wizard-step-body">
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
              <div className="image-desc">{currentDescription}</div>
            )}
          </div>
        )}

        {/* ── Messages ── */}
        {statusMsg && <p className="status-msg"><span className="status-spinner" /> {statusMsg}</p>}
        {error && <p className="error-msg">{error}</p>}

        {/* ── Download Progress Bar ── */}
        {installing && (
          <div className="download-progress">
            {downloadPercent !== null ? (
              <div className="progress-bar-container">
                <div className="progress-bar-fill" style={{ width: `${downloadPercent}%` }} />
                <span className="progress-bar-label">{downloadPercent}%</span>
              </div>
            ) : (
              <div className="progress-bar-container">
                <div className="progress-bar-indeterminate" />
                <span className="progress-bar-label">Downloading...</span>
              </div>
            )}
            {downloadLines.length > 0 && (
              <div className="download-lines">
                {downloadLines.slice(-8).map((line, i) => (
                  <div key={i} className="download-line">{line}</div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* ── Navigation Buttons ── */}
        <div className="modal-actions">
          <button className="btn-secondary" onClick={onClose} disabled={creating || installing}>
            Cancel
          </button>
          <div style={{ display: 'flex', gap: '8px', marginLeft: 'auto' }}>
            {step > 0 && (
              <button className="btn-secondary" onClick={prevStep} disabled={busy}>
                Back
              </button>
            )}
            {step < STEPS.length - 1 ? (
              <button className="btn-primary" onClick={nextStep} disabled={busy}>
                Next
              </button>
            ) : (
              <button
                className="btn-primary"
                onClick={handleCreate}
                disabled={busy || apiLevel === null}
              >
                {installing ? 'Downloading...' : creating ? 'Creating...' : 'Create Device'}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
