import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';

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

const STEPS = ['Device Template', 'Fingerprint Profile', 'System Image'];

export default function CreateWizard({ isOpen, onClose, onCreated }: CreateWizardProps) {
  // ── Step tracking ──
  const [step, setStep] = useState(0);

  // ── Step 1: Name + Template ──
  const [name, setName] = useState('');
  const [template, setTemplate] = useState('');

  // ── Step 2: Fingerprint Profile ──
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

  // ── Step 3: System Image ──
  const [images, setImages] = useState<SystemImage[]>([]);
  const [apiLevel, setApiLevel] = useState<number | null>(null);
  const [abi, setAbi] = useState('');
  const [tag, setTag] = useState('');

  // ── UI state ──
  const [deviceTemplates, setDeviceTemplates] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState('');
  const [statusMsg, setStatusMsg] = useState('');

  // ── Fetch data when wizard opens ──
  useEffect(() => {
    if (!isOpen) return;
    setError('');
    setStatusMsg('');
    setStep(0);
    setShowNewProfileForm(false);
    setSelectedProfile('');
    setLoading(true);

    Promise.all([
      invoke<SystemImage[]>('list_available_images_cmd'),
      invoke<string[]>('list_device_templates').catch(() => [] as string[]),
      invoke<FingerprintProfile[]>('list_profiles').catch(() => [] as FingerprintProfile[]),
    ])
      .then(([imgs, templates, fpProfiles]) => {
        setImages(imgs);
        setDeviceTemplates(templates || []);
        setProfiles(fpProfiles || []);
        if (imgs.length > 0) {
          const first = imgs[0];
          setApiLevel(first.api_level);
          setAbi(first.abi);
          setTag(first.tag);
        }
        if (templates && templates.length > 0) {
          setTemplate(templates[0]);
        } else {
          setTemplate('Custom');
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
    return [...new Set(
      images.filter((img) => img.api_level === apiLevel).map((img) => img.abi)
    )];
  }, [images, apiLevel]);

  const tags = useMemo(() => {
    if (apiLevel === null || !abi) return [];
    return [...new Set(
      images.filter((img) => img.api_level === apiLevel && img.abi === abi).map((img) => img.tag)
    )];
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
    if (!template) {
      setError('Please select a device template');
      return;
    }
    setError('');
    setStatusMsg('');

    const pkg = `system-images;android-${apiLevel};${tag};${abi}`;

    // Install system image
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

    // Create AVD with optional fingerprint profile
    setCreating(true);
    setStatusMsg('Creating device...');
    try {
      await invoke('create_device', {
        name: name.trim(),
        profile: template,
        apiLevel,
        abi,
        tag,
        fingerprintProfile: selectedProfile || null,
      });
      setName('');
      setTemplate(deviceTemplates.length > 0 ? deviceTemplates[0] : 'Custom');
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

        {/* ═══════════════ STEP 0: Name + Device Template ═══════════════ */}
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

            <label>Device Template</label>
            <select value={template} onChange={(e) => setTemplate(e.target.value)} disabled={busy}>
              {deviceTemplates.length === 0 && <option value="Custom">Custom</option>}
              {deviceTemplates.map((t) => (
                <option key={t} value={t}>{t}</option>
              ))}
              {deviceTemplates.length > 0 && <option value="Custom">Custom</option>}
            </select>
            {deviceTemplates.length > 0 && (
              <div className="image-desc">{deviceTemplates.length} device templates available</div>
            )}
          </div>
        )}

        {/* ═══════════════ STEP 1: Fingerprint Profile ═══════════════ */}
        {step === 1 && (
          <div className="wizard-step-body">
            <label>Fingerprint Profile (optional)</label>

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
                        {p.brand} — {p.model}
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
              <div className="image-desc">No fingerprint profiles yet. Create one below.</div>
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
                  New Fingerprint Profile
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

        {/* ═══════════════ STEP 2: System Image ═══════════════ */}
        {step === 2 && (
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
