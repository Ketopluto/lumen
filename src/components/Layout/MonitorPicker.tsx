import React from 'react';
import { usePathThumb } from '../../hooks';
import { useAppStore } from '../../store/appStore';
import { useUIStore } from '../../store/uiStore';
import './MonitorPicker.css';

/**
 * Your desk, drawn to scale: the monitors where they really are, each showing what is really on
 * it. On a dual-screen desktop the only question that matters is "which one is which", and a
 * picture of the actual screens answers it before a label could.
 *
 * Hidden entirely on a single-monitor desktop, where the choice would be noise.
 */
export const MonitorPicker: React.FC = () => {
  const { monitors, targetMonitor, setTargetMonitor, liveByMonitor, stillByMonitor, clearMonitor } = useAppStore();
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

  return (
    <div className="monitors" aria-label="Which monitor to set">
      {!collapsed && <span className="monitors__title">Set wallpaper on</span>}

      <div
        className="monitors__stage"
        style={{ aspectRatio: `${span} / ${height}` }}
        role="radiogroup"
        aria-label="Monitor"
      >
        {monitors.map((monitor) => (
          <Screen
            key={monitor.index}
            label={monitor.index + 1}
            name={monitor.name}
            size={`${monitor.width}x${monitor.height}`}
            primary={monitor.primary}
            live={liveByMonitor[monitor.index]}
            still={stillByMonitor[monitor.index]}
            selected={targetMonitor === monitor.index}
            onSelect={() => setTargetMonitor(monitor.index)}
            box={{
              left: `${((monitor.x - left) / span) * 100}%`,
              top: `${((monitor.y - top) / height) * 100}%`,
              width: `${(monitor.width / span) * 100}%`,
              height: `${(monitor.height / height) * 100}%`,
            }}
          />
        ))}
      </div>

      <button
        className={`monitors__all ${targetMonitor === null ? 'monitors__all--on' : ''}`}
        onClick={() => setTargetMonitor(null)}
      >
        {collapsed ? 'All' : 'All monitors'}
      </button>

      {!collapsed && selected && (
        <span className="monitors__detail">
          {selected.name}
          <span className="monitors__size">
            {selected.width}×{selected.height}
            {selected.primary ? ' primary' : ''}
          </span>
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

interface ScreenProps {
  label: number;
  name: string;
  size: string;
  primary: boolean;
  /** The file playing here, if a live wallpaper is running on this screen. */
  live?: string;
  /** The still image the desktop reports for this screen. */
  still?: string;
  selected: boolean;
  onSelect: () => void;
  box: React.CSSProperties;
}

const Screen: React.FC<ScreenProps> = ({ label, name, size, primary, live, still, selected, onSelect, box }) => {
  // What is on the screen right now: the live wallpaper wins, because that is what is moving.
  const thumb = usePathThumb(live ?? still);
  const fileName = (path?: string) => path?.split(/[\\/]/).pop();

  return (
    <button
      role="radio"
      aria-checked={selected}
      className={`monitors__screen ${selected ? 'monitors__screen--on' : ''}`}
      style={box}
      onClick={onSelect}
      title={[
        `${name} — ${size}`,
        primary ? 'primary' : null,
        live ? `playing ${fileName(live)}` : fileName(still),
      ]
        .filter(Boolean)
        .join('\n')}
    >
      {thumb && <img className="monitors__shot" src={thumb} alt="" />}
      <span className="monitors__number">{label}</span>
      {live && <span className="monitors__dot" aria-label="playing a live wallpaper" />}
    </button>
  );
};
