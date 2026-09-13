import React, { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import Toggle from '../../components/common/Toggle';
import { api, applyTheme, errorText, type Settings, type Theme } from '../../api';
import { capabilities, osName, startupLabel, trayName, trayNote } from '../../platform';
import { useUIStore } from '../../store/uiStore';
import './SettingsPage.css';

const THEMES: { id: Theme; label: string }[] = [
  { id: 'dark', label: '🌙 Dark' },
  { id: 'light', label: '☀️ Light' },
  { id: 'system', label: '💻 System' },
];
// Only the fit modes this desktop can actually apply.
const FITS = capabilities().fit_modes;

export const SettingsPage: React.FC = () => {
  const addToast = useUIStore((s) => s.addToast);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [saved, setSaved] = useState<Settings | null>(null);
  const [liveActive, setLiveActive] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    api
      .getSettings()
      .then((s) => {
        setSettings(s);
        setSaved(s);
      })
      .catch((e) => addToast({ type: 'error', title: "Couldn't load settings", message: errorText(e) }));
    api.liveStatus().then((s) => setLiveActive(!!s.path)).catch(() => {});
  }, [addToast]);

  if (!settings) return null;

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) => setSettings({ ...settings, [key]: value });
  const dirty = JSON.stringify(settings) !== JSON.stringify(saved);

  const save = async () => {
    setSaving(true);
    try {
      const next = await api.updateSettings({
        ...settings,
        wallhaven_api_key: settings.wallhaven_api_key?.trim() || null,
        unsplash_access_key: settings.unsplash_access_key?.trim() || null,
        pexels_api_key: settings.pexels_api_key?.trim() || null,
      });
      setSettings(next);
      setSaved(next);
      applyTheme(next.theme);
      addToast({ type: 'success', title: 'Settings saved', duration: 2000 });
    } catch (e) {
      addToast({ type: 'error', title: "Couldn't save settings", message: errorText(e) });
    } finally {
      setSaving(false);
    }
  };

  const pickDownloadDir = async () => {
    const picked = await open({ directory: true, multiple: false, title: 'Download folder', defaultPath: settings.download_dir });
    if (typeof picked === 'string') set('download_dir', picked);
  };

  const stopLive = async () => {
    await api.stopLive().catch(console.error);
    setLiveActive(false);
    addToast({ type: 'info', title: 'Live wallpaper stopped', duration: 2000 });
  };

  const row = (label: string, desc: string, control: React.ReactNode) => (
    <div className="settings-row">
      <div className="settings-row__info">
        <span className="settings-row__label">{label}</span>
        <span className="settings-row__desc">{desc}</span>
      </div>
      {control}
    </div>
  );

  return (
    <div className="settings-page page-enter">
      <div className="page-header">
        <h1 className="page-title">
          <span className="gradient-text">Settings</span>
        </h1>
      </div>

      <div className="settings-section glass-panel">
        <h2 className="settings-section__title">Appearance</h2>
        {row(
          'Theme',
          `Light, dark, or follow ${osName()}`,
          <div className="filter-chips">
            {THEMES.map((t) => (
              <button
                key={t.id}
                className={`filter-chip ${settings.theme === t.id ? 'filter-chip--active' : ''}`}
                onClick={() => {
                  set('theme', t.id);
                  applyTheme(t.id);
                }}
              >
                {t.label}
              </button>
            ))}
          </div>,
        )}
      </div>

      <div className="settings-section glass-panel">
        <h2 className="settings-section__title">Wallpaper</h2>
        {row(
          'Image fit',
          'How still images fill the screen',
          <div className="filter-chips">
            {FITS.map((f) => (
              <button
                key={f}
                className={`filter-chip ${settings.fit_mode === f ? 'filter-chip--active' : ''}`}
                onClick={() => set('fit_mode', f)}
              >
                {f[0].toUpperCase() + f.slice(1)}
              </button>
            ))}
          </div>,
        )}
        {row(
          'Live wallpaper volume',
          settings.live_wallpaper_volume === 0 ? 'Muted' : `${settings.live_wallpaper_volume}%`,
          <input
            type="range"
            className="settings-range"
            min={0}
            max={100}
            step={5}
            value={settings.live_wallpaper_volume}
            onChange={(e) => set('live_wallpaper_volume', Number(e.target.value))}
          />,
        )}
        {capabilities().pause_on_fullscreen
          ? row(
              'Pause when a game or video is fullscreen',
              'Frees your GPU for the fullscreen app',
              <Toggle checked={settings.pause_on_fullscreen} onChange={(v) => set('pause_on_fullscreen', v)} />,
            )
          : row(
              'Pause when a game or video is fullscreen',
              capabilities().session === 'wayland'
                ? 'Wayland does not let apps see what other windows are doing'
                : 'Not available on this system yet',
              <Toggle checked={false} onChange={() => {}} disabled />,
            )}
        {row(
          'Pause on battery',
          'Saves power when unplugged',
          <Toggle checked={settings.pause_on_battery} onChange={(v) => set('pause_on_battery', v)} />,
        )}
        {capabilities().live_wallpaper
          ? row(
              'Live wallpaper',
              liveActive ? 'A live wallpaper is running' : 'No live wallpaper running',
              <button className="btn btn--glass" onClick={stopLive} disabled={!liveActive}>
                Stop
              </button>,
            )
          : row('Live wallpaper', capabilities().live_unsupported_reason ?? 'Not available on this desktop', <span />)}
      </div>

      <div className="settings-section glass-panel">
        <h2 className="settings-section__title">Behavior</h2>
        {row(
          startupLabel(),
          `Launches quietly in the ${trayName()} and restores your wallpaper`,
          <Toggle checked={settings.start_on_boot} onChange={(v) => set('start_on_boot', v)} />,
        )}
        {row(
          `Keep running in the ${trayName()}`,
          `The close button leaves Lumen running instead of quitting${trayNote()}`,
          <Toggle checked={settings.minimize_to_tray} onChange={(v) => set('minimize_to_tray', v)} />,
        )}
        {row(
          'Download folder',
          settings.download_dir,
          <button className="btn btn--glass" onClick={pickDownloadDir}>
            Change…
          </button>,
        )}
      </div>

      <div className="settings-section glass-panel">
        <h2 className="settings-section__title">API keys</h2>
        <p className="settings-section__desc">
          Optional. Wallhaven, Wikimedia, NASA and Bing work without keys. Free keys unlock Unsplash (unsplash.com/developers) and
          Pexels photos + videos (pexels.com/api).
        </p>
        <div className="settings-field">
          <label>Unsplash access key</label>
          <input
            type="password"
            className="settings-input"
            value={settings.unsplash_access_key ?? ''}
            onChange={(e) => set('unsplash_access_key', e.target.value)}
            placeholder="Paste access key…"
          />
        </div>
        <div className="settings-field">
          <label>Pexels API key</label>
          <input
            type="password"
            className="settings-input"
            value={settings.pexels_api_key ?? ''}
            onChange={(e) => set('pexels_api_key', e.target.value)}
            placeholder="Paste API key…"
          />
        </div>
        <div className="settings-field">
          <label>Wallhaven API key (optional)</label>
          <input
            type="password"
            className="settings-input"
            value={settings.wallhaven_api_key ?? ''}
            onChange={(e) => set('wallhaven_api_key', e.target.value)}
            placeholder="Only needed for your Wallhaven account settings"
          />
        </div>
      </div>

      <div className="settings-section glass-panel">
        <div className="settings-about">
          <p className="settings-about__name gradient-text">Lumen</p>
          <p className="settings-about__version">Version 3.0.1</p>
          <p className="settings-about__desc">A lightweight live wallpaper manager.</p>
        </div>
      </div>

      <div className="settings-savebar">
        <button className="btn btn--primary" onClick={save} disabled={!dirty || saving}>
          {saving ? 'Saving…' : dirty ? 'Save changes' : 'Saved'}
        </button>
      </div>
    </div>
  );
};
