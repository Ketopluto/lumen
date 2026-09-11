import React, { useCallback, useEffect, useState } from 'react';
import { ask } from '@tauri-apps/plugin-dialog';
import { WallpaperGrid } from '../../components/WallpaperGrid/WallpaperGrid';
import { ImagePreview } from '../../components/ImagePreview/ImagePreview';
import { api, errorText, type Collection, type Favorite, type Wallpaper } from '../../api';
import { useAppStore } from '../../store/appStore';
import { useUIStore } from '../../store/uiStore';
import './FavoritesPage.css';

export const FavoritesPage: React.FC = () => {
  const addToast = useUIStore((s) => s.addToast);
  const favVersion = useAppStore((s) => s.favVersion);
  const [favorites, setFavorites] = useState<Favorite[]>([]);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const [previewIndex, setPreviewIndex] = useState<number | null>(null);

  const loadCollections = useCallback(() => api.getCollections().then(setCollections).catch(console.error), []);

  useEffect(() => {
    loadCollections();
  }, [loadCollections, favVersion]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    api
      .getFavorites(selected)
      .then((f) => !cancelled && setFavorites(f))
      .catch((e) => addToast({ type: 'error', title: "Couldn't load favorites", message: errorText(e) }))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [selected, favVersion, addToast]);

  const createCollection = async () => {
    try {
      const c = await api.createCollection(newName);
      setCreating(false);
      setNewName('');
      await loadCollections();
      setSelected(c.id);
    } catch (e) {
      addToast({ type: 'error', title: "Couldn't create collection", message: errorText(e) });
    }
  };

  const deleteCollection = async (c: Collection) => {
    const ok = await ask(`Delete the collection "${c.name}"? Your favorites stay.`, { title: 'Delete collection', kind: 'warning' });
    if (!ok) return;
    await api.deleteCollection(c.id).catch(console.error);
    setSelected(null);
    loadCollections();
  };

  const collectionActions = (wp: Wallpaper) => {
    const favId = wp.id;
    if (selected) {
      return (
        <button
          className="btn btn--glass"
          onClick={async () => {
            await api.removeFromCollection(selected, favId).catch(console.error);
            setPreviewIndex(null);
            setFavorites((prev) => prev.filter((f) => f.id !== favId));
            loadCollections();
          }}
        >
          Remove from collection
        </button>
      );
    }
    if (collections.length === 0) return null;
    return (
      <select
        className="select"
        value=""
        onChange={async (e) => {
          const cid = e.target.value;
          if (!cid) return;
          try {
            await api.addToCollection(cid, favId);
            addToast({ type: 'success', title: `Added to ${collections.find((c) => c.id === cid)?.name}`, duration: 2000 });
            loadCollections();
          } catch (err) {
            addToast({ type: 'error', title: "Couldn't add to collection", message: errorText(err) });
          }
        }}
      >
        <option value="">Add to collection…</option>
        {collections.map((c) => (
          <option key={c.id} value={c.id}>
            {c.name}
          </option>
        ))}
      </select>
    );
  };

  const current = collections.find((c) => c.id === selected);

  return (
    <div className="favorites-page page-enter">
      <div className="page-header">
        <div>
          <h1 className="page-title">
            <span className="gradient-text">Favorites</span>
          </h1>
          <p className="page-subtitle">{current ? current.name : `${favorites.length} saved`}</p>
        </div>
        <div className="page-actions">
          {current && (
            <button className="btn btn--danger" onClick={() => deleteCollection(current)}>
              Delete collection
            </button>
          )}
          {creating ? (
            <form
              className="inline-form"
              onSubmit={(e) => {
                e.preventDefault();
                createCollection();
              }}
            >
              <input
                autoFocus
                className="settings-input"
                placeholder="Collection name"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => e.key === 'Escape' && setCreating(false)}
              />
              <button className="btn btn--primary" type="submit" disabled={!newName.trim()}>
                Create
              </button>
            </form>
          ) : (
            <button className="btn btn--glass" onClick={() => setCreating(true)}>
              + New Collection
            </button>
          )}
        </div>
      </div>

      <div className="favorites-page__collections">
        <button className={`folder-item ${!selected ? 'folder-item--active' : ''}`} onClick={() => setSelected(null)}>
          ⭐ All Favorites
        </button>
        {collections.map((c) => (
          <button
            key={c.id}
            className={`folder-item ${selected === c.id ? 'folder-item--active' : ''}`}
            onClick={() => setSelected(c.id)}
          >
            📂 {c.name} ({c.count})
          </button>
        ))}
      </div>

      {!selected && collections.length > 0 && favorites.length > 0 && (
        <p className="page-hint">Tip: open a favorite to add it to a collection.</p>
      )}

      <WallpaperGrid
        wallpapers={favorites}
        loading={loading}
        onOpen={(_, i) => setPreviewIndex(i)}
        emptyMessage={selected ? 'This collection is empty — open a favorite to add it here' : 'No favorites yet. Tap ♡ on any wallpaper.'}
      />

      {previewIndex !== null && favorites[previewIndex] && (
        <ImagePreview
          items={favorites}
          index={previewIndex}
          onIndexChange={setPreviewIndex}
          onClose={() => setPreviewIndex(null)}
          extraActions={collectionActions}
        />
      )}
    </div>
  );
};
