import React, { useEffect, useRef } from 'react';
import type { Wallpaper } from '../../api';
import { WallpaperCard } from '../WallpaperCard/WallpaperCard';
import './WallpaperGrid.css';

interface WallpaperGridProps {
  wallpapers: Wallpaper[];
  loading?: boolean;
  onOpen?: (wp: Wallpaper, index: number) => void;
  hasMore?: boolean;
  onLoadMore?: () => void;
  emptyMessage?: string;
}

export const WallpaperGrid: React.FC<WallpaperGridProps> = ({
  wallpapers,
  loading = false,
  onOpen,
  hasMore = false,
  onLoadMore,
  emptyMessage = 'No wallpapers found',
}) => {
  const sentinel = useRef<HTMLDivElement>(null);

  // Infinite scroll: load the next page well before the user reaches the bottom.
  useEffect(() => {
    const el = sentinel.current;
    if (!el || !hasMore || !onLoadMore || loading) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) onLoadMore();
      },
      { root: el.closest('.layout__content'), rootMargin: '1200px' },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [hasMore, onLoadMore, loading, wallpapers.length]);

  if (!loading && wallpapers.length === 0) {
    return (
      <div className="wp-grid__empty">
        <svg
          className="wp-grid__empty-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <rect x="3" y="5" width="18" height="14" rx="2" />
          <path d="M3 15l5-5 4 4 3-3 6 6" />
        </svg>
        <p className="wp-grid__empty-text">{emptyMessage}</p>
      </div>
    );
  }

  return (
    <>
      <div className="wp-grid">
        {wallpapers.map((wp, i) => (
          <div key={wp.id} className="wp-grid__item" style={{ animationDelay: `${(i % 24) * 25}ms` }}>
            <WallpaperCard wallpaper={wp} onOpen={onOpen ? () => onOpen(wp, i) : undefined} />
          </div>
        ))}
        {loading &&
          Array.from({ length: wallpapers.length === 0 ? 12 : 4 }).map((_, i) => (
            <div key={`skel-${i}`} className="wp-grid__item">
              <div className="wp-card" aria-hidden="true">
                <div className="wp-card__skeleton skeleton" />
              </div>
            </div>
          ))}
      </div>
      <div ref={sentinel} className="wp-grid__sentinel" />
      {!loading && !hasMore && wallpapers.length > 0 && onLoadMore && (
        <p className="wp-grid__end">That's everything — {wallpapers.length} wallpapers</p>
      )}
    </>
  );
};
