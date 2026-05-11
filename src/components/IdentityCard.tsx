import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Profile {
  name: string;
  brand: string;
  model: string;
  manufacturer: string;
  device: string;
  fingerprint: string;
  dpi: number;
  resolution_w: number;
  resolution_h: number;
  imei: string;
  imei2: string;
  meid: string;
  phone_number: string;
  sim_operator: string;
  sim_operator_name: string;
  sim_country: string;
  sim_serial: string;
}

export interface DeviceIdentity {
  imei: string;
  imei2: string;
  meid: string;
  phone_number: string;
  sim_operator: string;
  sim_operator_name: string;
  sim_country: string;
  sim_serial: string;
}

interface Props {
  device_id: string;
  deviceStatus: string;
  profileName: string | null;
}

export default function IdentityCard({ device_id, deviceStatus, profileName }: Props) {
  const [identity, setIdentity] = useState<DeviceIdentity>({
    imei: '', imei2: '', meid: '', phone_number: '',
    sim_operator: '', sim_operator_name: '', sim_country: '', sim_serial: '',
  });
  const [applying, setApplying] = useState(false);
  const [applied, setApplied] = useState(false);

  // Load identity from the device's fingerprint profile on mount
  useEffect(() => {
    if (!profileName) return;
    invoke<Profile[]>('list_profiles').then((profiles) => {
      const match = profiles.find((p) => p.name === profileName);
      if (match) {
        setIdentity({
          imei: match.imei || '',
          imei2: match.imei2 || '',
          meid: match.meid || '',
          phone_number: match.phone_number || '',
          sim_operator: match.sim_operator || '',
          sim_operator_name: match.sim_operator_name || '',
          sim_country: match.sim_country || '',
          sim_serial: match.sim_serial || '',
        });
      }
    }).catch(console.error);
  }, [profileName]);

  const handleApply = async () => {
    if (deviceStatus !== 'running') return;
    setApplying(true);
    try {
      await invoke('set_device_identity', {
        deviceId: device_id,
        imei: identity.imei || null,
        imei2: identity.imei2 || null,
        meid: identity.meid || null,
        phoneNumber: identity.phone_number || null,
        simOperator: identity.sim_operator || null,
        simOperatorName: identity.sim_operator_name || null,
        simCountry: identity.sim_country || null,
        simSerial: identity.sim_serial || null,
      });
      setApplied(true);
      setTimeout(() => setApplied(false), 2000);
    } catch (e) {
      console.error('Failed to apply identity:', e);
    }
    setApplying(false);
  };

  const setField = (field: keyof DeviceIdentity, value: string) => {
    setIdentity((prev) => ({ ...prev, [field]: value }));
  };

  const Field = ({ label, field, placeholder }: { label: string; field: keyof DeviceIdentity; placeholder?: string }) => (
    <div className="identity-field">
      <label>{label}</label>
      <input
        type="text"
        value={identity[field]}
        onChange={(e) => setField(field, e.target.value)}
        placeholder={placeholder}
        className="identity-input"
      />
    </div>
  );

  return (
    <div className="identity-card">
      <div className="identity-body">
        <div className="identity-grid">
          <Field label="IMEI 1" field="imei" placeholder="15-digit IMEI" />
          <Field label="IMEI 2" field="imei2" placeholder="15-digit IMEI" />
          <Field label="MEID" field="meid" placeholder="14-digit MEID" />
          <Field label="Phone Number" field="phone_number" placeholder="+905XXXXXXXXX" />
        </div>
        <div className="identity-grid">
          <Field label="Operator (MCC+MNC)" field="sim_operator" placeholder="28601" />
          <Field label="Operator Name" field="sim_operator_name" placeholder="Turkcell" />
          <Field label="ISO Country" field="sim_country" placeholder="tr" />
          <Field label="SIM Serial (ICCID)" field="sim_serial" placeholder="89..." />
        </div>
      </div>
      <div className="identity-footer">
        {applied && <span className="identity-saved">✓ Applied</span>}
        <button
          className="btn-primary btn-sm"
          onClick={handleApply}
          disabled={applying || deviceStatus !== 'running'}
        >
          {applying ? 'Applying...' : 'Apply to Device'}
        </button>
      </div>
    </div>
  );
}
