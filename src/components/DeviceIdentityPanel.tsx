import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';

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
  imei?: string;
  imei2?: string;
  meid?: string;
  phone_number?: string;
  sim_operator?: string;
  sim_operator_name?: string;
  sim_country?: string;
  sim_serial?: string;
}

interface Props {
  device_id: string;
  device_status: string;
}

// Luhn algorithm — generates a valid 15-digit IMEI
function generateIMEI(): string {
  const digits: number[] = [];
  for (let i = 0; i < 14; i++) {
    digits.push(Math.floor(Math.random() * 10));
  }
  // Luhn check digit
  let sum = 0;
  for (let i = 0; i < 14; i++) {
    let d = digits[i];
    if (i % 2 === 1) {
      d *= 2;
      if (d > 9) d -= 9;
    }
    sum += d;
  }
  const check = (10 - (sum % 10)) % 10;
  return [...digits, check].join('');
}

type ApplyState = 'idle' | 'loading' | 'done' | 'error';

export default function DeviceIdentityPanel({ device_id, device_status }: Props) {
  const [profiles, setProfiles] = useState<FingerprintProfile[]>([]);
  const [mode, setMode] = useState<'preset' | 'custom'>('preset');

  // Preset picker state
  const [selectedBrand, setSelectedBrand] = useState('');
  const [selectedProfileName, setSelectedProfileName] = useState('');

  // Editable identity fields
  const [imei, setImei] = useState('');
  const [imei2, setImei2] = useState('');
  const [phoneNumber, setPhoneNumber] = useState('');
  const [simOperator, setSimOperator] = useState('');
  const [simOperatorName, setSimOperatorName] = useState('');
  const [simCountry, setSimCountry] = useState('');
  const [simSerial, setSimSerial] = useState('');

  const [applyState, setApplyState] = useState<ApplyState>('idle');
  const [applyMsg, setApplyMsg] = useState('');

  useEffect(() => {
    invoke<FingerprintProfile[]>('list_profiles').then(setProfiles).catch(() => {});
  }, []);

  // Unique brands from profiles
  const brands = useMemo(() => {
    const set = new Set(profiles.map(p => p.brand));
    return [...set].sort();
  }, [profiles]);

  // Models for selected brand
  const brandProfiles = useMemo(
    () => profiles.filter(p => p.brand === selectedBrand),
    [profiles, selectedBrand]
  );

  // Auto-fill from selected profile
  const selectedProfile = useMemo(
    () => profiles.find(p => p.name === selectedProfileName),
    [profiles, selectedProfileName]
  );

  useEffect(() => {
    if (!selectedProfile) return;
    setImei(selectedProfile.imei ?? '');
    setImei2(selectedProfile.imei2 ?? '');
    setPhoneNumber(selectedProfile.phone_number ?? '');
    setSimOperator(selectedProfile.sim_operator ?? '');
    setSimOperatorName(selectedProfile.sim_operator_name ?? '');
    setSimCountry(selectedProfile.sim_country ?? '');
    setSimSerial(selectedProfile.sim_serial ?? '');
  }, [selectedProfile]);

  const handleBrandChange = (brand: string) => {
    setSelectedBrand(brand);
    setSelectedProfileName('');
  };

  const handleApply = async () => {
    if (device_status !== 'running') {
      setApplyMsg('Device must be running to apply identity');
      setApplyState('error');
      setTimeout(() => setApplyState('idle'), 3000);
      return;
    }
    setApplyState('loading');
    setApplyMsg('');
    try {
      await invoke('set_device_identity', {
        deviceId: device_id,
        imei: imei || null,
        imei2: imei2 || null,
        meid: null,
        phoneNumber: phoneNumber || null,
        simOperator: simOperator || null,
        simOperatorName: simOperatorName || null,
        simCountry: simCountry || null,
        simSerial: simSerial || null,
      });
      setApplyState('done');
      setApplyMsg('Identity applied successfully');
      setTimeout(() => setApplyState('idle'), 3000);
    } catch (e: any) {
      setApplyState('error');
      setApplyMsg(e?.message ?? String(e));
      setTimeout(() => setApplyState('idle'), 5000);
    }
  };

  const isRunning = device_status === 'running';

  return (
    <div className="identity-panel">
      {/* Mode toggle */}
      <div className="identity-mode-row">
        <span className="identity-mode-label">Device Model:</span>
        <div className="identity-mode-toggle">
          <button
            className={`identity-mode-btn ${mode === 'preset' ? 'active' : ''}`}
            onClick={() => setMode('preset')}
          >
            Preset
          </button>
          <button
            className={`identity-mode-btn ${mode === 'custom' ? 'active' : ''}`}
            onClick={() => setMode('custom')}
          >
            Custom
          </button>
        </div>
      </div>

      {/* Preset picker */}
      {mode === 'preset' && (
        <div className="identity-preset-box">
          <div className="identity-field-row">
            <label>Brand</label>
            <select
              value={selectedBrand}
              onChange={e => handleBrandChange(e.target.value)}
              className="identity-select"
            >
              <option value="">— select brand —</option>
              {brands.map(b => (
                <option key={b} value={b}>{b}</option>
              ))}
            </select>
          </div>
          <div className="identity-field-row">
            <label>Name</label>
            <select
              value={selectedProfileName}
              onChange={e => setSelectedProfileName(e.target.value)}
              className="identity-select"
              disabled={!selectedBrand}
            >
              <option value="">— select model —</option>
              {brandProfiles.map(p => (
                <option key={p.name} value={p.name}>{p.name}</option>
              ))}
            </select>
          </div>
          {selectedProfile && (
            <div className="identity-field-row">
              <label>Model</label>
              <span className="identity-model-code">{selectedProfile.model}</span>
            </div>
          )}
        </div>
      )}

      {/* Identity fields */}
      <div className="identity-fields">
        {/* IMEI */}
        <div className="identity-field-row">
          <label>IMEI</label>
          <div className="identity-input-with-btn">
            <input
              type="text"
              value={imei}
              onChange={e => setImei(e.target.value)}
              placeholder="15-digit IMEI"
              maxLength={15}
              className="identity-input"
            />
            <button
              className="identity-regen-btn"
              onClick={() => setImei(generateIMEI())}
              title="Generate random valid IMEI"
            >
              ↻
            </button>
          </div>
        </div>

        {/* IMEI2 */}
        <div className="identity-field-row">
          <label>IMEI2</label>
          <div className="identity-input-with-btn">
            <input
              type="text"
              value={imei2}
              onChange={e => setImei2(e.target.value)}
              placeholder="15-digit IMEI (SIM2)"
              maxLength={15}
              className="identity-input"
            />
            <button
              className="identity-regen-btn"
              onClick={() => setImei2(generateIMEI())}
              title="Generate random valid IMEI"
            >
              ↻
            </button>
          </div>
        </div>

        {/* Phone Number */}
        <div className="identity-field-row">
          <label>Phone Number</label>
          <input
            type="text"
            value={phoneNumber}
            onChange={e => setPhoneNumber(e.target.value)}
            placeholder="+1 555 000 0000"
            className="identity-input"
          />
        </div>

        {/* SIM Operator */}
        <div className="identity-field-row">
          <label>SIM Operator</label>
          <div style={{ display: 'flex', gap: 6 }}>
            <input
              type="text"
              value={simOperator}
              onChange={e => setSimOperator(e.target.value)}
              placeholder="MCC+MNC (e.g. 28602)"
              className="identity-input"
              style={{ flex: 1 }}
            />
            <input
              type="text"
              value={simOperatorName}
              onChange={e => setSimOperatorName(e.target.value)}
              placeholder="Name (e.g. Turkcell)"
              className="identity-input"
              style={{ flex: 1 }}
            />
          </div>
        </div>

        {/* SIM Country */}
        <div className="identity-field-row">
          <label>SIM Country</label>
          <input
            type="text"
            value={simCountry}
            onChange={e => setSimCountry(e.target.value)}
            placeholder="ISO code (e.g. tr, us)"
            className="identity-input"
            style={{ maxWidth: 120 }}
          />
        </div>
      </div>

      {/* Apply button */}
      <div className="identity-actions">
        {!isRunning && (
          <span className="identity-hint">Start the device to apply identity via ADB</span>
        )}
        {applyMsg && (
          <span className={`identity-msg ${applyState === 'error' ? 'identity-msg-error' : 'identity-msg-ok'}`}>
            {applyMsg}
          </span>
        )}
        <button
          className="btn-primary"
          onClick={handleApply}
          disabled={!isRunning || applyState === 'loading'}
          style={{ marginLeft: 'auto' }}
        >
          {applyState === 'loading' ? 'Applying...' : applyState === 'done' ? '✓ Applied' : 'Apply Identity'}
        </button>
      </div>
    </div>
  );
}
