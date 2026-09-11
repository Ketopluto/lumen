import React, { useEffect, useRef, useState } from 'react';
import { ask } from '@tauri-apps/plugin-dialog';
import { api, fromHistory, SOURCE_LABELS, type HistoryEntry } from '../../api';
import { useInView, useThumbSrc } from '../../hooks';
import { useAppStore } from '../../store/appStore';
import './HistoryPage.css';

const HistoryThumb: React.FC<{ entry: HistoryEntry }> = ({ entry }) => {
  const ref = useRef<HTMLDivElement>(null);
  const visible = useInView(ref);
  const { src, video } = useThumbSrc(fromHistory(entry), visible);
  return (
    <div className="history-item__thumb" ref={ref}>
      {src && <img src={src} alt="" decoding="async" />}
      {video && <video src={`${video}#t=1`} muted preload="metadata" />}
    </div>
  );
};

const formatDate = (iso: string) => {
  const d = new Date(iso.includes('T') ? iso : `${iso.replace(' ', 'T')}Z`);
  return isNaN(d.getTime())
    ? iso
    : d.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
};

export const HistoryPage: React.FC = () => {
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const apply = useAppStore((s) => s.apply);
  const busy = useAppStore((s) => s.busy);

  const load = () =>
    api
      .getHistory()
      .then(setHistory)
      .catch(console.error)
      .finally(() => setLoading(false));

  useEffect(() => {
    load();
  }, []);

  const clearAll = async () => {
    if (await ask('Clear your whole wallpaper history?', { title: 'Clear history', kind: 'warning' })) {
      await api.clearHistory().catch(console.error);
      setHistory([]);
    }
  };

  return (
    <div className="history-page page-enter">
      <div className="page-header">
        <div>
          <h1 className="page-title">
            <span className="gradient-text">History</span>
          </h1>
          <p className="page-subtitle">Click any entry to use it again</p>
        </div>
        {history.length > 0 && (
          <button className="btn btn--glass" onClick={clearAll}>
            Clear History
          </button>
        )}
      </div>

      {loading ? (
        <div className="history-list">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="history-item glass-panel">
              <div className="skeleton" style={{ width: 120, height: 75, borderRadius: 8 }} />
              <div style={{ flex: 1 }}>
                <div className="skeleton" style={{ width: '60%', height: 14, marginBottom: 8 }} />
                <div className="skeleton" style={{ width: '30%', height: 12 }} />
              </div>
            </div>
          ))}
        </div>
      ) : history.length === 0 ? (
        <div className="wp-grid__empty">
          <div className="wp-grid__empty-icon">🕐</div>
          <p className="wp-grid__empty-text">No history yet</p>
        </div>
      ) : (
        <div className="history-list">
          {history.map((item, i) => {
            const wp = fromHistory(item);
            const isBusy = wp.id in busy;
            return (
              <div
                key={item.id}
                className="history-item glass-panel"
                style={{ animationDelay: `${Math.min(i, 15) * 30}ms` }}
                onClick={() => apply(wp)}
              >
                <HistoryThumb entry={item} />
                <div className="history-item__info">
                  <p className="history-item__title">{item.title || 'Untitled'}</p>
                  <p className="history-item__meta">
                    {formatDate(item.set_at)} · {SOURCE_LABELS[item.source] ?? item.source}
                    {item.media_type === 'video' ? ' · Live' : ''}
                  </p>
                  {item.width && item.height && (
                    <p className="history-item__res">
                      {item.width}×{item.height}
                    </p>
                  )}
                </div>
                <button className="btn btn--glass history-item__revert" disabled={isBusy}>
                  {isBusy ? 'Applying…' : 'Use again'}
                </button>
                <button
                  className="history-item__remove"
                  title="Remove from history"
                  onClick={async (e) => {
                    e.stopPropagation();
                    await api.removeHistoryEntry(item.id).catch(console.error);
                    setHistory((prev) => prev.filter((h) => h.id !== item.id));
                  }}
                >
                  ✕
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
