import React from 'react';
import { useAppStore } from '../../store/appStore';
import { useUIStore } from '../../store/uiStore';
import './MonitorPicker.css';

/**
 * Which monitor the next wallpaper lands on, drawn as the monitors themselves rather than a list
 * of numbers — on a dual-screen desktop the only question that matters is "which one is which",
 * and a scaled picture of the real layout answers it immediately.
 *
 * Hidden entirely on a single-monitor desktop, where the choice would be noise.
 */
export const MonitorPicker: React.FC = () => {
  const { monitors, targetMonitor, setTargetMonitor, liveByMonitor, clearMonitor } = useAppStore();
  const collapsed = useUIStore((s) => s.sidebarCollapsed);

  if (monitors.length < 2) return null;

  // Lay the monitors out in proportion to where they actually are.
  const left = Math.min(...monitors.map((m) => m.x));
  const top = Math.min(...monitors.map((m) => m.y));
  const right = Math.max(...monitors.map((m) => m.x + m.width));
  const bottom = Math.max(...monitors.map((m) => m.y + m.height));
  const span = Math.max(right - left, 1);
  const height = Math.max(bottom - top, 1);
  const selected = monitors.find((m) => m.index === targetMonitor);
  const playing = targetMonitor !== null && targetMonitor in liveByMonitor;

  const fileOn = (index: number) => {
    const path = liveByMonitor[index];
    return path ? path.split(/[\\/]/).pop() : undefined;
  };

  return (
    <div className="monitors" aria-label="Which monitor to set">
      {!collapsed && <span className="monitors__title">Set wallpaper on</span>}

      <div
        className="monitors__stage"
        style={{ aspectRatio: `${span} / ${height}` }}
        role="radiogroup"
        aria-label="Monitor"
      >
        {monitors.map((monitor) => {
          const live = fileOn(monitor.index);
          return (
            <button
              key={monitor.index}
              role="radio"
              aria-checked={targetMonitor === monitor.index}
              className={`monitors__screen ${targetMonitor === monitor.index ? 'monitors__screen--on' : ''}`}
              style={{
                left: `${((monitor.x - left) / span) * 100}%`,
                top: `${((monitor.y - top) / height) * 100}%`,
                width: `${(monitor.width / span) * 100}%`,
                height: `${(monitor.height / height) * 100}%`,
              }}
              onClick={() => setTargetMonitor(monitor.index)}
              title={[
                `${monitor.name} — ${monitor.width}x${monitor.height}`,
                monitor.primary ? 'primary' : null,
                live ? `playing ${live}` : 'no live wallpaper',
              ]
                .filter(Boolean)
                .join('\n')}
            >
              <span className="monitors__number">{monitor.index + 1}</span>
              {live && <span className="monitors__dot" aria-label="playing a live wallpaper" />}
            </button>
          );
        })}
      </div>

      <button
        className={`monitors__all ${targetMonitor === null ? 'monitors__all--on' : ''}`}
        onClick={() => setTargetMonitor(null)}
      >
        {collapsed ? 'All' : 'All monitors'}
      </button>

      {!collapsed && selected && (
        <span className="monitors__detail">
          {selected.name} · {selected.width}×{selected.height}
          {selected.primary ? ' · primary' : ''}
        </span>
      )}
      {!collapsed && playing && (
        <button className="monitors__stop" onClick={() => clearMonitor(targetMonitor)}>
          Stop live wallpaper here
        </button>
      )}
    </div>
  );
};
