import { useState } from 'react';

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
  onDropApk?: (id: string, apkPath: string) => void;
}

export default function DeviceCard({ device, onStart, onStop, onDelete, onClone, onDropApk }: Props) {
  const isRunning = device.status === 'running';
  const [dragOver, setDragOver] = useState(false);

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer.types.includes('Files')) {
      e.dataTransfer.dropEffect = 'copy';
    }
  };

  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer.types.includes('Files')) {
      setDragOver(true);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    // Only set false if we're leaving the card itself, not a child
    if ((e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) return;
    setDragOver(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);

    const files = e.dataTransfer.files;
    if (files.length > 0 && onDropApk) {
      const file = files[0];
      // Tauri webview adds a `path` property to File objects
      const apkPath = (file as any).path || file.name;
      onDropApk(device.id, apkPath);
    }
  };

  const cardClass = [
    'device-card',
    isRunning ? 'status-running' : 'status-stopped',
    dragOver ? 'drag-over' : '',
  ].filter(Boolean).join(' ');

  return (
    <div
      className={cardClass}
      onDragOver={handleDragOver}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className="device-info">
        <div className="device-name">
          <span className="status-dot" />
          {device.display_name}
        </div>
        <div className="device-meta">
          {device.profile || 'custom'} · API {device.api_level} · Port {device.port || '-'}
          {device.root_enabled && ' · rooted'}
        </div>
        {dragOver && <div className="drop-hint">Drop APK to install</div>}
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
