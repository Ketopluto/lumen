import React, { useCallback, useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { WallpaperGrid } from '../../components/WallpaperGrid/WallpaperGrid';
import { ImagePreview } from '../../components/ImagePreview/ImagePreview';
import { api, errorText, fromLocal, type Wallpaper, type WatchedFolder } from '../../api';
import { useUIStore } from '../../store/uiStore';
import './LocalPage.css';

type Kind = 'all' | 'image' | 'video';

const folderName = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;

export const LocalPage: React.FC = () => {
  const addToast = useUIStore((s) => s.addToast);
  const [folders, setFolders] = useState<WatchedFolder[]>([]);
  const [downloadsDir, setDownloadsDir] = useState('');
  const [selected, setSelected] = useState('');
  const [items, setItems] = useState<Wallpaper[]>([]);
  const [kind, setKind] = useState<Kind>('all');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewIndex, setPreviewIndex] = useState<number | null>(null);

  const scan = useCallback(async (path: string) => {
    setSelected(path);
    setLoading(true);
    setError(null);
    setItems([]);
    try {
      const images = await api.getLocalImages(path);
      setItems(images.map(fromLocal));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    Promise.all([api.getWatchedFolders(), api.getSettings()])
      .then(([f, s]) => {
        setFolders(f);
        setDownloadsDir(s.download_dir);
        scan(s.download_dir);
      })
      .catch((e) => setError(errorText(e)));
  }, [scan]);

  const addFolder = async () => {
    try {
      const picked = await open({ directory: true, multiple: false, title: 'Choose a wallpaper folder' });
      if (typeof picked !== 'string') return;
      const folder = await api.addWatchedFolder(picked);
      setFolders((prev) => (prev.some((f) => f.path === folder.path) ? prev : [...prev, folder]));
      scan(folder.path);
    } catch (e) {
      addToast({ type: 'error', title: "Couldn't add folder", message: errorText(e) });
    }
  };

  const removeFolder = async (path: string) => {
    try {
      await api.removeWatchedFolder(path);
      setFolders((prev) => prev.filter((f) => f.path !== path));
      if (selected === path) scan(downloadsDir);
    } catch (e) {
      addToast({ type: 'error', title: "Couldn't remove folder", message: errorText(e) });
    }
  };

  const visible = kind === 'all' ? items : items.filter((w) => (kind === 'video' ? w.media_type === 'video' : w.media_type !== 'video'));

  return (
    <div className="local-page page-enter">
      <div className="page-header">
        <div>
          <h1 className="page-title">
            <span className="gradient-text">My files</span>
          </h1>
          <p className="page-subtitle truncate">{selected || 'Your images and videos'}</p>
        </div>
        <div className="page-actions">
          {selected && (
            <button className="btn btn--glass" onClick={() => scan(selected)}>
              Refresh
            </button>
          )}
          <button className="btn btn--primary" onClick={addFolder}>
            + Add Folder
          </button>
        </div>
      </div>

      <div className="local-page__folders">
        {downloadsDir && (
          <button
            className={`folder-item ${selected === downloadsDir ? 'folder-item--active' : ''}`}
            onClick={() => scan(downloadsDir)}
          >
            ⬇ Downloads
          </button>
        )}
        {folders.map((f) => (
          <span key={f.id} className={`folder-item ${selected === f.path ? 'folder-item--active' : ''}`}>
            <button className="folder-item__name" onClick={() => scan(f.path)} title={f.path}>
              📁 {folderName(f.path)}
            </button>
            <button className="folder-item__remove" onClick={() => removeFolder(f.path)} title="Remove folder">
              ✕
            </button>
          </span>
        ))}
      </div>

      <div className="filter-chips" style={{ marginBottom: 'var(--space-4)' }}>
        {(['all', 'image', 'video'] as Kind[]).map((k) => (
          <button key={k} className={`filter-chip ${kind === k ? 'filter-chip--active' : ''}`} onClick={() => setKind(k)}>
            {k === 'all' ? `All (${items.length})` : k === 'image' ? 'Images' : 'Videos'}
          </button>
        ))}
      </div>

      {error && <div className="notice notice--error">{error}</div>}

      <WallpaperGrid
        wallpapers={visible}
        loading={loading}
        onOpen={(_, i) => setPreviewIndex(i)}
        emptyMessage={
          selected === downloadsDir
            ? 'Wallpapers you set or save from Browse land here'
            : 'No images or videos in this folder'
        }
      />

      {previewIndex !== null && visible[previewIndex] && (
        <ImagePreview
          items={visible}
          index={previewIndex}
          onIndexChange={setPreviewIndex}
          onClose={() => setPreviewIndex(null)}
        />
      )}
    </div>
  );
};
