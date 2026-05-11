import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface LogLine {
  serial: string;
  line: string;
}

interface Props {
  device_id: string;
  deviceStatus: string;
}

export default function LogcatPanel({ device_id, deviceStatus }: Props) {
  const [streaming, setStreaming] = useState(false);
  const [lines, setLines] = useState<string[]>([]);
  const [autoScroll, setAutoScroll] = useState(true);
  const bottomRef = useRef<HTMLDivElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const startStream = useCallback(async () => {
    try {
      setLines([]);
      const unlisten = await listen<LogLine>('logcat-line', (event) => {
        setLines((prev) => {
          const next = [...prev, event.payload.line];
          return next.length > 500 ? next.slice(next.length - 500) : next;
        });
      });
      unlistenRef.current = unlisten;
      await invoke('logcat_start', { id: device_id });
      setStreaming(true);
    } catch (e) {
      console.error('Logcat start failed:', e);
      setStreaming(false);
    }
  }, [device_id]);

  const stopStream = useCallback(() => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    setStreaming(false);
  }, []);

  // Cleanup on unmount or device_id change
  useEffect(() => {
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, [device_id]);

  // Auto-stop if device stops
  useEffect(() => {
    if (deviceStatus !== 'running' && streaming) {
      stopStream();
    }
  }, [deviceStatus, streaming, stopStream]);

  // Auto-scroll
  useEffect(() => {
    if (autoScroll && bottomRef.current) {
      bottomRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [lines, autoScroll]);

  const clearLog = () => setLines([]);

  return (
    <div className="logcat-panel">
      <div className="logcat-header">
        <span className="logcat-title">📜 Live Logcat</span>
        <div className="logcat-controls">
          {lines.length > 0 && (
            <span className="logcat-line-count">{lines.length} lines</span>
          )}
          <button
            className={`logcat-scroll-btn ${autoScroll ? 'active' : ''}`}
            onClick={() => setAutoScroll(!autoScroll)}
            title={autoScroll ? 'Auto-scroll ON' : 'Auto-scroll OFF'}
          >
            {autoScroll ? '⏬' : '⏸'}
          </button>
          <button className="logcat-clear-btn" onClick={clearLog} title="Clear">
            🗑
          </button>
          {streaming ? (
            <button className="logcat-btn stop" onClick={stopStream}>
              ⬛ Stop
            </button>
          ) : (
            <button
              className="logcat-btn start"
              onClick={startStream}
              disabled={deviceStatus !== 'running'}
            >
              ▶ Start
            </button>
          )}
        </div>
      </div>
      <div className="logcat-output">
        {lines.length === 0 ? (
          <div className="logcat-placeholder">
            {deviceStatus === 'running'
              ? 'Press ▶ Start to begin streaming logs.'
              : 'Start the device to stream logs.'}
          </div>
        ) : (
          lines.map((line, i) => (
            <div key={i} className="logcat-line">{line}</div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
