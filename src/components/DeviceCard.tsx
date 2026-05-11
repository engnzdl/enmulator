export interface Device {
  id: string;
  display_name: string;
  profile: string | null;
  api_level: number;
  status: string;
  port: number;
  root_enabled: boolean;
}

interface Props {
  device: Device;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onDelete: (id: string) => void;
  onClone: (id: string) => void;
}

export default function DeviceCard({ device, onStart, onStop, onDelete, onClone }: Props) {
  const isRunning = device.status === 'running';

  return (
    <div className={`device-card ${isRunning ? 'status-running' : 'status-stopped'}`}>
      <div className="device-info">
        <div className="device-name">
          <span className="status-dot" />
          {device.display_name}
        </div>
        <div className="device-meta">
          {device.profile || 'custom'} · API {device.api_level} · Port {device.port || '-'}
          {device.root_enabled && ' · rooted'}
        </div>
      </div>
      <div className="device-actions">
        {isRunning ? (
          <button onClick={() => onStop(device.id)}>⏹ Stop</button>
        ) : (
          <button onClick={() => onStart(device.id)}>▶ Start</button>
        )}
        <button onClick={() => onClone(device.id)}>⧉ Clone</button>
        <button className="btn-danger" onClick={() => onDelete(device.id)}>🗑 Delete</button>
      </div>
    </div>
  );
}
