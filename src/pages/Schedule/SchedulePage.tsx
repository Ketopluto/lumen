import React, { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import Toggle from '../../components/common/Toggle';
import { api, errorText, type Collection, type FitMode, type SlideshowConfig, type SlideshowSource } from '../../api';
import { useUIStore } from '../../store/uiStore';
import './SchedulePage.css';

const INTERVALS = [
  { value: 60, label: '1 min' },
  { value: 300, label: '5 min' },
  { value: 900, label: '15 min' },
  { value: 1800, label: '30 min' },
  { value: 3600, label: '1 hour' },
  { value: 10800, label: '3 hours' },
  { value: 21600, label: '6 hours' },
  { value: 86400, label: 'Daily' },
];
const FITS: FitMode[] = ['fill', 'fit', 'stretch', 'center', 'span'];

type SourceKind = SlideshowSource['type'];

export const SchedulePage: React.FC = () => {
  const addToast = useUIStore((s) => s.addToast);
  const [running, setRunning] = useState<SlideshowConfig | null>(null);
  const [kind, setKind] = useState<SourceKind>('Favorites');
  const [collectionId, setCollectionId] = useState('');
  const [folder, setFolder] = useState('');
  const [collections, setCollections] = useState<Collection[]>([]);
  const [interval, setIntervalSecs] = useState(1800);
  const [shuffle, setShuffle] = useState(true);
  const [fit, setFit] = useState<FitMode>('fill');
  const [working, setWorking] = useState(false);

  useEffect(() => {
    api.getCollections().then(setCollections).catch(console.error);
    api
      .slideshowStatus()
      .then((cfg) => {
        setRunning(cfg);
        if (!cfg) return;
        setKind(cfg.source.type);
        if (cfg.source.type === 'Collection') setCollectionId(cfg.source.value);
        if (cfg.source.type === 'Folder') setFolder(cfg.source.value);
        setIntervalSecs(cfg.interval_secs);
        setShuffle(cfg.shuffle);
        setFit(cfg.fit_mode);
      })
      .catch(console.error);
  }, []);

  const pickFolder = async () => {
    const picked = await open({ directory: true, multiple: false, title: 'Slideshow folder' });
    if (typeof picked === 'string') setFolder(picked);
  };

  const buildSource = (): SlideshowSource | null => {
    if (kind === 'Collection') return collectionId ? { type: 'Collection', value: collectionId } : null;
    if (kind === 'Folder') return folder ? { type: 'Folder', value: folder } : null;
    return { type: 'Favorites' };
  };

  const start = async () => {
    const source = buildSource();
    if (!source) {
      addToast({ type: 'warning', title: kind === 'Folder' ? 'Choose a folder first' : 'Choose a collection first' });
      return;
    }
    const config: SlideshowConfig = { source, interval_secs: interval, shuffle, fit_mode: fit };
    setWorking(true);
    try {
      await api.startSlideshow(config);
      setRunning(config);
      addToast({ type: 'success', title: 'Slideshow started', duration: 2500 });
    } catch (e) {
      addToast({ type: 'error', title: "Couldn't start slideshow", message: errorText(e) });
    } finally {
      setWorking(false);
    }
  };

  const stop = async () => {
    await api.stopSlideshow().catch(console.error);
    setRunning(null);
    addToast({ type: 'info', title: 'Slideshow stopped', duration: 2000 });
  };

  return (
    <div className="schedule-page page-enter">
      <div className="page-header">
        <div>
          <h1 className="page-title">
            <span className="gradient-text">Slideshow</span>
          </h1>
          <p className="page-subtitle">Rotate wallpapers automatically — keeps running from the tray</p>
        </div>
      </div>

      <div className="schedule-card glass-panel">
        <div className="schedule-card__header">
          <h2>Settings</h2>
          <div className={`schedule-status ${running ? 'schedule-status--running' : ''}`}>
            {running ? '● Running' : '○ Stopped'}
          </div>
        </div>

        <div className="schedule-section">
          <span className="schedule-label">Source</span>
          <div className="schedule-options">
            {(['Favorites', 'Collection', 'Folder'] as SourceKind[]).map((k) => (
              <button key={k} className={`filter-chip ${kind === k ? 'filter-chip--active' : ''}`} onClick={() => setKind(k)}>
                {k === 'Favorites' ? '⭐ Favorites' : k === 'Collection' ? '📂 Collection' : '📁 Folder'}
              </button>
            ))}
          </div>
          {kind === 'Collection' && (
            <select className="select schedule-sub" value={collectionId} onChange={(e) => setCollectionId(e.target.value)}>
              <option value="">{collections.length ? 'Choose a collection…' : 'No collections yet'}</option>
              {collections.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name} ({c.count})
                </option>
              ))}
            </select>
          )}
          {kind === 'Folder' && (
            <div className="schedule-sub schedule-folder">
              <span className="truncate">{folder || 'No folder chosen'}</span>
              <button className="btn btn--glass" onClick={pickFolder}>
                Choose…
              </button>
            </div>
          )}
        </div>

        <div className="schedule-section">
          <span className="schedule-label">Change every</span>
          <div className="schedule-options">
            {INTERVALS.map((i) => (
              <button
                key={i.value}
                className={`filter-chip ${interval === i.value ? 'filter-chip--active' : ''}`}
                onClick={() => setIntervalSecs(i.value)}
              >
                {i.label}
              </button>
            ))}
          </div>
        </div>

        <div className="schedule-section">
          <span className="schedule-label">Image fit</span>
          <div className="schedule-options">
            {FITS.map((f) => (
              <button key={f} className={`filter-chip ${fit === f ? 'filter-chip--active' : ''}`} onClick={() => setFit(f)}>
                {f[0].toUpperCase() + f.slice(1)}
              </button>
            ))}
          </div>
        </div>

        <div className="schedule-section">
          <Toggle checked={shuffle} onChange={setShuffle} label="Shuffle order" />
        </div>

        <div className="schedule-actions">
          <button className="btn btn--primary" onClick={start} disabled={working}>
            {working ? 'Starting…' : running ? '↻ Restart with these settings' : '▶ Start Slideshow'}
          </button>
          {running && (
            <button className="btn btn--glass" onClick={stop}>
              ■ Stop
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
