import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { SearchBar } from '../../components/SearchBar/SearchBar';
import { WallpaperGrid } from '../../components/WallpaperGrid/WallpaperGrid';
import { ImagePreview } from '../../components/ImagePreview/ImagePreview';
import { api, errorText, type Wallpaper } from '../../api';
import './BrowsePage.css';

interface SourceDef {
  id: string;
  label: string;
  note: string;
  search?: boolean;
  suggestions?: string[];
}

const SOURCES: SourceDef[] = [
  { id: 'wallhaven', label: 'Wallhaven', note: '600k+ wallpapers', search: true,
    suggestions: ['nature', 'space', 'anime', 'city', 'minimalist', 'cyberpunk', 'mountains', 'cars', 'ocean'] },
  { id: 'anime', label: 'Anime', note: '110k+ anime wallpapers', search: true,
    suggestions: ['lookism', 'solo leveling', 'jujutsu kaisen', 'demon slayer', 'one piece', 'chainsaw man', 'naruto', 'attack on titan', 'dragon ball'] },
  { id: 'konachan', label: 'Konachan', note: 'Anime art board', search: true,
    suggestions: ['scenic', 'sky', 'night', 'city', 'original', 'genshin impact', 'touhou'] },
  { id: 'live', label: 'Live', note: 'Video wallpapers', search: true,
    suggestions: ['timelapse', 'ocean waves', 'clouds', 'aurora', 'rain', 'waterfall', 'stars', 'city night', 'fire'] },
  { id: 'commons', label: 'Wikimedia', note: 'Award-winning photos', search: true,
    suggestions: ['landscape', 'mountain', 'sunset', 'forest', 'beach', 'lake', 'architecture', 'wildlife'] },
  { id: 'nasa', label: 'NASA', note: 'Space imagery', search: true,
    suggestions: ['galaxy', 'nebula', 'earth', 'mars', 'jupiter', 'hubble', 'webb', 'aurora'] },
  { id: 'bing', label: 'Bing Daily', note: 'Last 16 days' },
  { id: 'unsplash', label: 'Unsplash', note: 'Needs free key', search: true,
    suggestions: ['nature', 'dark', 'abstract', 'architecture', 'mountains'] },
  { id: 'pexels', label: 'Pexels', note: 'Needs free key', search: true,
    suggestions: ['nature', 'abstract', 'city', 'ocean', 'forest'] },
  { id: 'pexels_video', label: 'Pexels Video', note: 'Needs free key', search: true,
    suggestions: ['nature', 'ocean', 'abstract', 'city', 'space', 'rain'] },
];

const SORTS = [
  { id: 'toplist', label: 'Top' },
  { id: 'hot', label: 'Hot' },
  { id: 'date_added', label: 'Latest' },
  { id: 'random', label: 'Random' },
  { id: 'favorites', label: 'Most liked' },
];
const RANGES = [
  { id: '1w', label: 'Week' },
  { id: '1M', label: 'Month' },
  { id: '1y', label: 'Year' },
];
const RESOLUTIONS = ['', '1920x1080', '2560x1440', '3840x2160'];
const CATEGORY_LABELS = ['General', 'Anime', 'People'];

interface Filters {
  source: string;
  query: string;
  sorting: string;
  topRange: string;
  categories: string;
  resolution: string;
}

interface BrowseState {
  items: Wallpaper[];
  page: number;
  hasMore: boolean;
  seed: string | null;
  error: string | null;
}

const EMPTY: BrowseState = { items: [], page: 0, hasMore: true, seed: null, error: null };

// Survives navigating to other pages and back, so browsing doesn't restart.
let cache: { filters: Filters; state: BrowseState } | null = null;

export const BrowsePage: React.FC = () => {
  const navigate = useNavigate();
  const [filters, setFilters] = useState<Filters>(
    () =>
      cache?.filters ?? { source: 'wallhaven', query: '', sorting: 'toplist', topRange: '1M', categories: '111', resolution: '' },
  );
  const [input, setInput] = useState(filters.query);
  const [state, setState] = useState<BrowseState>(() => cache?.state ?? EMPTY);
  const [loading, setLoading] = useState(false);
  const [previewIndex, setPreviewIndex] = useState<number | null>(null);

  const requestId = useRef(0);
  const stateRef = useRef(state);
  stateRef.current = state;
  const loadingRef = useRef(false);

  const source = SOURCES.find((s) => s.id === filters.source) ?? SOURCES[0];

  const fetchPage = useCallback(
    async (reset: boolean) => {
      const id = ++requestId.current;
      const prev = reset ? EMPTY : stateRef.current;
      const page = prev.page + 1;
      loadingRef.current = true;
      setLoading(true);
      if (reset) setState(EMPTY);
      try {
        const res = await api.search({
          source: filters.source,
          query: filters.query || undefined,
          page,
          sorting: filters.source === 'wallhaven' || filters.source === 'anime' ? filters.sorting : undefined,
          top_range: filters.topRange,
          categories: filters.categories,
          resolution: filters.resolution || undefined,
          seed: prev.seed,
        });
        if (id !== requestId.current) return;
        const seen = new Set(prev.items.map((w) => w.id));
        const fresh = res.wallpapers.filter((w) => !seen.has(w.id) && seen.add(w.id));
        setState({
          items: [...prev.items, ...fresh],
          page: res.page,
          hasMore: res.has_more && res.wallpapers.length > 0,
          seed: res.seed ?? prev.seed,
          error: null,
        });
      } catch (e) {
        if (id !== requestId.current) return;
        setState({ ...prev, hasMore: false, error: errorText(e) });
      } finally {
        if (id === requestId.current) {
          loadingRef.current = false;
          setLoading(false);
        }
      }
    },
    [filters],
  );

  // Restart the listing whenever the filters change (but reuse the cache on first mount).
  const firstRun = useRef(true);
  useEffect(() => {
    if (firstRun.current && cache && cache.state.items.length > 0) {
      firstRun.current = false;
      return;
    }
    firstRun.current = false;
    fetchPage(true);
  }, [fetchPage]);

  useEffect(() => {
    cache = { filters, state };
  }, [filters, state]);

  const loadMore = useCallback(() => {
    if (!loadingRef.current && stateRef.current.hasMore) fetchPage(false);
  }, [fetchPage]);

  const update = (patch: Partial<Filters>) => setFilters((f) => ({ ...f, ...patch }));
  const search = (q: string) => {
    setInput(q);
    update({ query: q.trim() });
  };

  const categoryMask = filters.categories.split('');
  const toggleCategory = (i: number) => {
    const next = [...categoryMask];
    next[i] = next[i] === '1' ? '0' : '1';
    if (next.join('') !== '000') update({ categories: next.join('') });
  };

  const needsKey = useMemo(() => state.error?.includes('API key') ?? false, [state.error]);

  return (
    <div className="browse-page page-enter">
      <div className="page-header">
        <div>
          <h1 className="page-title">
            <span className="gradient-text">Browse</span> Wallpapers
          </h1>
          <p className="page-subtitle">{source.note}</p>
        </div>
        {source.search && (
          <SearchBar value={input} onChange={setInput} onSearch={search} placeholder={`Search ${source.label}…`} />
        )}
      </div>

      <div className="browse-page__sources">
        {SOURCES.map((src) => (
          <button
            key={src.id}
            className={`source-tab ${filters.source === src.id ? 'source-tab--active' : ''}`}
            onClick={() => {
              if (src.id === filters.source) return;
              setInput('');
              update({ source: src.id, query: '' });
            }}
          >
            {src.label}
          </button>
        ))}
      </div>

      {(filters.source === 'wallhaven' || filters.source === 'anime') && (
        <div className="filter-panel">
          <div className="filter-group">
            <span className="filter-label">Sort</span>
            <div className="filter-chips">
              {SORTS.map((s) => (
                <button
                  key={s.id}
                  className={`filter-chip ${filters.sorting === s.id ? 'filter-chip--active' : ''}`}
                  onClick={() => update({ sorting: s.id })}
                >
                  {s.label}
                </button>
              ))}
            </div>
          </div>
          {filters.sorting === 'toplist' && (
            <div className="filter-group">
              <span className="filter-label">Period</span>
              <div className="filter-chips">
                {RANGES.map((r) => (
                  <button
                    key={r.id}
                    className={`filter-chip ${filters.topRange === r.id ? 'filter-chip--active' : ''}`}
                    onClick={() => update({ topRange: r.id })}
                  >
                    {r.label}
                  </button>
                ))}
              </div>
            </div>
          )}
          {filters.source === 'wallhaven' && (
            <div className="filter-group">
              <span className="filter-label">Categories</span>
              <div className="filter-chips">
                {CATEGORY_LABELS.map((c, i) => (
                  <button
                    key={c}
                    className={`filter-chip ${categoryMask[i] === '1' ? 'filter-chip--active' : ''}`}
                    onClick={() => toggleCategory(i)}
                  >
                    {c}
                  </button>
                ))}
              </div>
            </div>
          )}
          <div className="filter-group">
            <span className="filter-label">Min resolution</span>
            <div className="filter-chips">
              {RESOLUTIONS.map((r) => (
                <button
                  key={r || 'any'}
                  className={`filter-chip ${filters.resolution === r ? 'filter-chip--active' : ''}`}
                  onClick={() => update({ resolution: r })}
                >
                  {r ? r.replace('1920x1080', 'HD').replace('2560x1440', '2K').replace('3840x2160', '4K') : 'Any'}
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {source.suggestions && (
        <div className="browse-page__suggestions">
          {source.suggestions.map((s) => (
            <button
              key={s}
              className={`filter-chip ${filters.query === s ? 'filter-chip--active' : ''}`}
              onClick={() => search(filters.query === s ? '' : s)}
            >
              {s}
            </button>
          ))}
        </div>
      )}

      {state.error && (
        <div className="notice notice--error">
          <span>{state.error}</span>
          {needsKey ? (
            <button className="btn btn--glass" onClick={() => navigate('/settings')}>
              Open Settings
            </button>
          ) : (
            <button className="btn btn--glass" onClick={() => fetchPage(state.items.length === 0)}>
              Retry
            </button>
          )}
        </div>
      )}

      <WallpaperGrid
        wallpapers={state.items}
        loading={loading}
        hasMore={state.hasMore && !state.error}
        onLoadMore={loadMore}
        onOpen={(_, i) => setPreviewIndex(i)}
        emptyMessage={state.error ? 'Nothing to show' : 'No wallpapers matched — try another search'}
      />

      {previewIndex !== null && state.items[previewIndex] && (
        <ImagePreview
          items={state.items}
          index={previewIndex}
          onIndexChange={(i) => {
            setPreviewIndex(i);
            if (i >= state.items.length - 3) loadMore();
          }}
          onClose={() => setPreviewIndex(null)}
        />
      )}
    </div>
  );
};
