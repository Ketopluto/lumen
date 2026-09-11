import React, { memo, useRef, useState } from 'react';
import { isLive, type Wallpaper } from '../../api';
import { useInView, useThumbSrc } from '../../hooks';
import { useAppStore } from '../../store/appStore';
import './WallpaperCard.css';

interface WallpaperCardProps {
  wallpaper: Wallpaper;
  onOpen?: (wp: Wallpaper) => void;
}

export const WallpaperCard: React.FC<WallpaperCardProps> = memo(({ wallpaper, onOpen }) => {
  const ref = useRef<HTMLDivElement>(null);
  const visible = useInView(ref);
  const { src, video } = useThumbSrc(wallpaper, visible);
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);

  const favorited = useAppStore((s) => !!(s.favIds[wallpaper.id] ?? s.favUrls[wallpaper.url]));
  const progress = useAppStore((s) => s.busy[wallpaper.id]);
  const busy = progress !== undefined;
  const apply = useAppStore((s) => s.apply);
  const toggleFavorite = useAppStore((s) => s.toggleFavorite);

  const badge = wallpaper.media_type === 'video' ? 'LIVE' : wallpaper.media_type === 'gif' ? 'GIF' : null;
  const label =
    wallpaper.width && wallpaper.height ? `${wallpaper.width}×${wallpaper.height}` : wallpaper.title ?? '';

  return (
    <div
      className="wp-card glass-panel"
      ref={ref}
      onClick={() => onOpen?.(wallpaper)}
      title={wallpaper.title ?? undefined}
    >
      <div className="wp-card__image-wrapper">
        {!loaded && !failed && <div className="wp-card__skeleton skeleton" />}
        {failed && <div className="wp-card__error">Preview unavailable</div>}
        {src && !failed && (
          <img
            className={`wp-card__image ${loaded ? 'wp-card__image--loaded' : ''}`}
            src={src}
            alt=""
            decoding="async"
            referrerPolicy="no-referrer"
            onLoad={() => setLoaded(true)}
            onError={() => setFailed(true)}
          />
        )}
        {video && !failed && (
          <video
            className={`wp-card__image ${loaded ? 'wp-card__image--loaded' : ''}`}
            src={`${video}#t=1`}
            muted
            preload="metadata"
            onLoadedData={() => setLoaded(true)}
            onError={() => setFailed(true)}
          />
        )}
        {badge && <span className={`wp-card__badge ${isLive(wallpaper) ? 'wp-card__badge--live' : ''}`}>{badge}</span>}
        {busy && (
          <div className="wp-card__busy">
            <span className="spinner" />
            <span>{progress != null ? `${Math.round(progress * 100)}%` : 'Working…'}</span>
          </div>
        )}
        <div className="wp-card__overlay">
          <div className="wp-card__actions">
            <button
              className="wp-card__action-btn wp-card__action-btn--set"
              title="Set as wallpaper"
              disabled={busy}
              onClick={(e) => {
                e.stopPropagation();
                apply(wallpaper);
              }}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="2" y="2" width="12" height="12" rx="2" stroke="currentColor" strokeWidth="1.5" />
                <path d="M2 11l3.5-3.5 2.5 2.5 2-2 4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              </svg>
            </button>
            <button
              className={`wp-card__action-btn ${favorited ? 'wp-card__action-btn--favorited' : ''}`}
              title={favorited ? 'Remove from favorites' : 'Add to favorites'}
              onClick={(e) => {
                e.stopPropagation();
                toggleFavorite(wallpaper);
              }}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill={favorited ? 'currentColor' : 'none'}>
                <path d="M8 13.5l-5.5-5A3.2 3.2 0 018 3.5a3.2 3.2 0 015.5 5L8 13.5z" stroke="currentColor" strokeWidth="1.3" />
              </svg>
            </button>
          </div>
        </div>
      </div>
      <div className="wp-card__meta">
        <span className="wp-card__resolution truncate">{label}</span>
        {wallpaper.colors && wallpaper.colors.length > 0 && (
          <div className="wp-card__colors">
            {wallpaper.colors.slice(0, 4).map((color, i) => (
              <span key={i} className="wp-card__color-dot" style={{ backgroundColor: color }} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
});
