import React, { useEffect, useState } from 'react';
import { fileSrc, isRemote, SOURCE_LABELS, type Wallpaper } from '../../api';
import { useThumbSrc } from '../../hooks';
import { useAppStore } from '../../store/appStore';
import './ImagePreview.css';

interface ImagePreviewProps {
  items: Wallpaper[];
  index: number;
  onIndexChange: (index: number) => void;
  onClose: () => void;
  /** Extra buttons/controls for the current wallpaper (e.g. collection actions). */
  extraActions?: (wp: Wallpaper) => React.ReactNode;
}

export const ImagePreview: React.FC<ImagePreviewProps> = ({ items, index, onIndexChange, onClose, extraActions }) => {
  const wallpaper = items[index];
  const { src: thumb } = useThumbSrc(wallpaper, true);
  const [fullLoaded, setFullLoaded] = useState(false);
  const [fullFailed, setFullFailed] = useState(false);

  const favorited = useAppStore((s) => !!(s.favIds[wallpaper.id] ?? s.favUrls[wallpaper.url]));
  const progress = useAppStore((s) => s.busy[wallpaper.id]);
  const busy = progress !== undefined;
  const { apply, toggleFavorite, download } = useAppStore.getState();

  const localPath = wallpaper.local_path ?? (isRemote(wallpaper.url) ? null : wallpaper.url);
  const full = localPath ? fileSrc(localPath) : wallpaper.url;
  const isVideo = wallpaper.media_type === 'video';

  useEffect(() => {
    setFullLoaded(false);
    setFullFailed(false);
  }, [wallpaper.id]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
      else if (e.key === 'ArrowRight' && index < items.length - 1) onIndexChange(index + 1);
      else if (e.key === 'ArrowLeft' && index > 0) onIndexChange(index - 1);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [index, items.length, onClose, onIndexChange]);

  return (
    <div className="preview-overlay" onClick={onClose}>
      <div className="preview-content" onClick={(e) => e.stopPropagation()}>
        <button className="preview-close" onClick={onClose} aria-label="Close">
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path d="M4 4l12 12M16 4L4 16" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          </svg>
        </button>

        <div className="preview-stage">
          {/* Thumbnail shows instantly; the full-size file fades in on top once downloaded. */}
          {isVideo ? (
            <video
              key={full}
              className="preview-media"
              src={full}
              poster={thumb}
              autoPlay
              muted
              loop
              playsInline
              onLoadedData={() => setFullLoaded(true)}
              onError={() => setFullFailed(true)}
            />
          ) : (
            <>
              {thumb && <img className="preview-media" src={thumb} alt="" referrerPolicy="no-referrer" />}
              {!fullFailed && (
                <img
                  key={full}
                  className={`preview-media preview-media--full ${fullLoaded ? 'is-loaded' : ''}`}
                  src={full}
                  alt={wallpaper.title ?? 'Preview'}
                  referrerPolicy="no-referrer"
                  onLoad={() => setFullLoaded(true)}
                  onError={() => setFullFailed(true)}
                />
              )}
            </>
          )}
          {!fullLoaded && !fullFailed && (
            <div className="preview-loading">
              <span className="spinner" /> Loading full {isVideo ? 'video' : 'resolution'}…
            </div>
          )}
          {fullFailed && <div className="preview-loading">Full preview unavailable — you can still set it</div>}
          {index > 0 && (
            <button className="preview-nav preview-nav--prev" onClick={() => onIndexChange(index - 1)} aria-label="Previous">
              ‹
            </button>
          )}
          {index < items.length - 1 && (
            <button className="preview-nav preview-nav--next" onClick={() => onIndexChange(index + 1)} aria-label="Next">
              ›
            </button>
          )}
        </div>

        <div className="preview-info glass-panel">
          <div className="preview-meta">
            <h3 className="preview-title truncate">{wallpaper.title || 'Untitled'}</h3>
            <p className="preview-resolution">
              {wallpaper.width && wallpaper.height ? `${wallpaper.width}×${wallpaper.height} · ` : ''}
              {SOURCE_LABELS[wallpaper.source] ?? wallpaper.source}
              {isVideo ? ' · Live video' : ''}
            </p>
            {wallpaper.author && <p className="preview-source truncate">{wallpaper.author}</p>}
          </div>
          <div className="preview-actions">
            {extraActions?.(wallpaper)}
            {isRemote(wallpaper.url) && (
              <button className="btn btn--glass" disabled={busy} onClick={() => download(wallpaper)}>
                Save
              </button>
            )}
            <button className="btn btn--glass" onClick={() => toggleFavorite(wallpaper)}>
              {favorited ? '♥ Favorited' : '♡ Favorite'}
            </button>
            <button className="btn btn--primary" disabled={busy} onClick={() => apply(wallpaper)}>
              {busy ? (progress != null ? `Downloading ${Math.round(progress * 100)}%` : 'Applying…') : 'Set as wallpaper'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
