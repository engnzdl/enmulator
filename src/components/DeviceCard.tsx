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
  selectMode?: boolean;
  selected?: boolean;
  isActive?: boolean;
  onSelect?: (id: string, selected: boolean) => void;
  onClick?: () => void;
}

export default function DeviceCard({
  device,
  onStart,
  onStop,
  onDelete,
  onClone,
  onDropApk,
  selectMode,
  selected,
  isActive,
  onSelect,
  onClick,
}: Props) {
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
      const apkPath = (file as any).path || file.name;
      onDropApk(device.id, apkPath);
    }
  };

  const cardClass = [
    'device-card',
    isRunning ? 'status-running' : 'status-stopped',
    dragOver ? 'drag-over' : '',
    selectMode ? 'select-mode' : '',
    selected ? 'device-card-selected' : '',
    isActive && !selectMode && !selected ? 'device-card-selected' : '',
  ].filter(Boolean).join(' ');

  return (
    <div
      className={cardClass}
      onDragOver={handleDragOver}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      onClick={onClick}
    >
      {selectMode && (
        <input
          type="checkbox"
          className="device-checkbox"
          checked={selected || false}
          onChange={(e) => onSelect?.(device.id, e.target.checked)}
          onClick={(e) => e.stopPropagation()}
        />
      )}

      {/* Device Icon */}
      <div className="device-icon">
        📱
      </div>

      {/* Device Info */}
      <div className="device-info">
        <div className="device-name">
          <span className="status-dot" />
          {device.display_name}
        </div>
        <div className="device-meta">
          {device.profile || 'custom'} · API {device.api_level}
          {dragOver && <span className="drop-hint"> — Drop APK to install</span>}
        </div>
      </div>

      {/* Mini action buttons (visible on hover) */}
      <div className="device-actions">
        {isRunning ? (
          <button
            className="device-action-btn stop-btn"
            onClick={(e) => { e.stopPropagation(); onStop(device.id); }}
            title="Stop"
          >
            ⏹
          </button>
        ) : (
          <button
            className="device-action-btn start-btn"
            onClick={(e) => { e.stopPropagation(); onStart(device.id); }}
            title="Start"
          >
            ▶
          </button>
        )}
        <button
          className="device-action-btn"
          onClick={(e) => { e.stopPropagation(); onClone(device.id); }}
          title="Clone"
        >
          ⧉
        </button>
        <button
          className="device-action-btn delete-btn"
          onClick={(e) => { e.stopPropagation(); onDelete(device.id); }}
          title="Delete"
        >
          🗑
        </button>
      </div>
    </div>
  );
}
