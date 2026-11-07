import React from 'react';
import { useAppStore } from '../../store/appStore';
import { useUIStore } from '../../store/uiStore';
import './MonitorPicker.css';

/**
 * Which monitor the next wallpaper lands on. Hidden entirely on a single-monitor desktop, where
 * the choice would be noise.
 */
export const MonitorPicker: React.FC = () => {
  const { monitors, targetMonitor, setTargetMonitor, liveOnMonitor, clearMonitor } = useAppStore();
  const collapsed = useUIStore((s) => s.sidebarCollapsed);

  if (monitors.length < 2) return null;

  const playing = targetMonitor !== null && liveOnMonitor(targetMonitor);
  const selected = monitors.find((m) => m.index === targetMonitor);

  return (
    <div className="monitors" aria-label="Wallpaper target">
      {!collapsed && <span className="monitors__title">Set wallpaper on</span>}
      <div className="monitors__row">
        <button
          className={`monitors__chip ${targetMonitor === null ? 'monitors__chip--on' : ''}`}
          onClick={() => setTargetMonitor(null)}
          title="Every monitor"
        >
          All
        </button>
        {monitors.map((monitor) => (
          <button
            key={monitor.index}
            className={`monitors__chip ${targetMonitor === monitor.index ? 'monitors__chip--on' : ''}`}
            onClick={() => setTargetMonitor(monitor.index)}
            title={`${monitor.name} — ${monitor.width}x${monitor.height}${monitor.primary ? ' (primary)' : ''}`}
          >
            {monitor.index + 1}
          </button>
        ))}
      </div>
      {!collapsed && selected && (
        <span className="monitors__detail">
          {selected.width}x{selected.height}
          {selected.primary ? ' · primary' : ''}
        </span>
      )}
      {!collapsed && playing && (
        <button className="monitors__stop" onClick={() => clearMonitor(targetMonitor)}>
          Stop this monitor
        </button>
      )}
    </div>
  );
};
