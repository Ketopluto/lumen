import React, { memo, useRef, useState } from 'react';
import { displayTitle, type Wallpaper } from '../../api';
import { useInView, useThumbSrc } from '../../hooks';
import { useAppStore } from '../../store/appStore';
import './WallpaperCard.css';

interface WallpaperCardProps {
  wallpaper: Wallpaper;
  onOpen?: (wp: Wallpaper) => void;
}

const Heart: React.FC<{ filled: boolean }> = ({ filled }) => (
  <svg viewBox="0 0 24 24" fill={filled ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth="2" strokeLinejoin="round" aria-hidden="true">
    <path d="M12 20s-7.5-4.6-7.5-10.2A4.2 4.2 0 0 1 12 7.3a4.2 4.2 0 0 1 7.5 2.5C19.5 15.4 12 20 12 20z" />
  </svg>
);

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
  const size = wallpaper.width && wallpaper.height ? `${wallpaper.width}×${wallpaper.height}` : null;
  const name = displayTitle(wallpaper) || 'wallpaper';

  return (
    <div
      ref={ref}
      className="wp-card"
      role="button"
      tabIndex={0}
      aria-label={`Preview ${name}`}
      title={wallpaper.title ?? undefined}
      onClick={() => onOpen?.(wallpaper)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' && e.target === e.currentTarget) onOpen?.(wallpaper);
      }}
    >
      {!loaded && !failed && <div className="wp-card__skeleton skeleton" />}
      {failed && <div className="wp-card__error">No preview</div>}
      {src && !failed && (
        <img
          className={`wp-card__media ${loaded ? 'is-loaded' : ''}`}
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
          className={`wp-card__media ${loaded ? 'is-loaded' : ''}`}
          src={`${video}#t=1`}
          muted
          preload="metadata"
          onLoadedData={() => setLoaded(true)}
          onError={() => setFailed(true)}
        />
      )}

      {badge && <span className="wp-card__badge">{badge}</span>}
      {favorited && (
        <span className="wp-card__fav" aria-label="In favorites">
          <Heart filled />
        </span>
      )}

      {busy ? (
        <div className="wp-card__busy">
          <span className="spinner" />
          <span>{progress != null ? `Downloading ${Math.round(progress * 100)}%` : 'Setting…'}</span>
        </div>
      ) : (
        <div className="wp-card__overlay">
          {(size || wallpaper.source === 'local') && (
            <span className="wp-card__info">{size ?? name}</span>
          )}
          <div className="wp-card__actions">
            <button
              className="wp-card__set"
              onClick={(e) => {
                e.stopPropagation();
                apply(wallpaper);
              }}
            >
              Set as wallpaper
            </button>
            <button
              className={`wp-card__icon ${favorited ? 'is-on' : ''}`}
              aria-label={favorited ? 'Remove from favorites' : 'Add to favorites'}
              title={favorited ? 'Remove from favorites' : 'Add to favorites'}
              onClick={(e) => {
                e.stopPropagation();
                toggleFavorite(wallpaper);
              }}
            >
              <Heart filled={favorited} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
});
