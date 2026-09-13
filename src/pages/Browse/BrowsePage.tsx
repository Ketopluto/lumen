import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { SearchBar } from '../../components/SearchBar/SearchBar';
import { WallpaperGrid } from '../../components/WallpaperGrid/WallpaperGrid';
import { ImagePreview } from '../../components/ImagePreview/ImagePreview';
import { api, errorText, type Wallpaper } from '../../api';
import { mediaSupport } from '../../platform';
import './BrowsePage.css';

interface Provider {
  id: string;
  label: string;
  /** Search box placeholder; no search box when absent. */
  placeholder?: string;
  suggestions?: string[];
  /** 'full' = sort, period, size and categories; 'basic' = without categories. */
  filters?: 'full' | 'basic';
}

interface Category {
  id: string;
  label: string;
  note: string;
  providers: Provider[];
}

// Grouped by what people are looking for, not by which website it comes from.
const ALL_CATEGORIES: Category[] = [
  {
    id: 'discover',
    label: 'Discover',
    note: 'Popular wallpapers from Wallhaven',
    providers: [
      { id: 'wallhaven', label: 'Wallhaven', filters: 'full', placeholder: 'Search 600,000+ wallpapers',
        suggestions: ['nature', 'space', 'city', 'minimalist', 'cyberpunk', 'mountains', 'cars', 'ocean', 'dark'] },
    ],
  },
  {
    id: 'anime',
    label: 'Anime',
    note: 'Anime wallpapers',
    providers: [
      { id: 'anime', label: 'Wallhaven', filters: 'basic', placeholder: 'Search a series or character',
        suggestions: ['lookism', 'solo leveling', 'jujutsu kaisen', 'demon slayer', 'one piece', 'chainsaw man', 'naruto', 'attack on titan', 'dragon ball'] },
      { id: 'konachan', label: 'Konachan', placeholder: 'Search a tag, like scenic or sky',
        suggestions: ['scenic', 'sky', 'night', 'city', 'original', 'genshin impact', 'touhou'] },
    ],
  },
  {
    id: 'live',
    label: 'Live',
    note: 'Moving wallpapers that play behind your icons',
    providers: [
      { id: 'anime_live', label: 'Anime', placeholder: 'Search a series, character or tag',
        suggestions: ['scenery', 'pixel art', 'night', 'rain', 'city', 'sky', 'genshin impact', 'touhou', 'vocaloid'] },
      { id: 'konachan_live', label: 'Anime HD', placeholder: 'Search a tag, like night or city',
        suggestions: ['night', 'city', 'sky', 'original', 'scenic', 'pixel art'] },
      { id: 'live', label: 'Wikimedia', placeholder: 'Search time-lapse videos',
        suggestions: ['aurora', 'clouds', 'sunset', 'stars', 'city', 'storm', 'ocean', 'mountains'] },
      { id: 'pexels_video', label: 'Pexels', placeholder: 'Search Pexels videos',
        suggestions: ['nature', 'ocean', 'abstract', 'city', 'space', 'rain'] },
    ],
  },
  {
    id: 'photos',
    label: 'Photos',
    note: 'Award-winning photography',
    providers: [
      { id: 'commons', label: 'Wikimedia', placeholder: 'Search featured photos',
        suggestions: ['landscape', 'mountain', 'sunset', 'forest', 'beach', 'lake', 'architecture', 'wildlife'] },
      { id: 'unsplash', label: 'Unsplash', placeholder: 'Search Unsplash',
        suggestions: ['nature', 'dark', 'abstract', 'architecture', 'mountains'] },
      { id: 'pexels', label: 'Pexels', placeholder: 'Search Pexels',
        suggestions: ['nature', 'abstract', 'city', 'ocean', 'forest'] },
    ],
  },
  {
    id: 'space',
    label: 'Space',
    note: "NASA's image library",
    providers: [
      { id: 'nasa', label: 'NASA', placeholder: 'Search NASA images',
        suggestions: ['galaxy', 'nebula', 'earth', 'mars', 'jupiter', 'hubble', 'webb', 'aurora'] },
    ],
  },
  {
    id: 'daily',
    label: 'Daily',
    note: "Bing's photo of the day, from the last 16 days",
    providers: [{ id: 'bing', label: 'Bing' }],
  },
];

// Sources this system cannot play are hidden rather than left to fail at playback time.
const CATEGORIES: Category[] = ALL_CATEGORIES.map((category) => ({
  ...category,
  providers: category.providers.filter((provider) => {
    if (provider.id === 'live') return mediaSupport.webm;
    if (provider.id === 'pexels_video') return mediaSupport.mp4;
    return true;
  }),
})).filter((category) => category.providers.length > 0);

const SORTS = [
  { id: 'toplist', label: 'Top' },
  { id: 'hot', label: 'Trending' },
  { id: 'date_added', label: 'Newest' },
  { id: 'random', label: 'Random' },
  { id: 'favorites', label: 'Most favorited' },
];
const RANGES = [
  { id: '1w', label: 'Week' },
  { id: '1M', label: 'Month' },
  { id: '1y', label: 'Year' },
];
const RESOLUTIONS = [
  { id: '', label: 'Any' },
  { id: '1920x1080', label: 'Full HD or larger' },
  { id: '2560x1440', label: '2K or larger' },
  { id: '3840x2160', label: '4K' },
];
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
let cache: { category: string; providers: Record<string, string>; filters: Filters; state: BrowseState } | null = null;

const Select: React.FC<{
  label: string;
  value: string;
  options: { id: string; label: string }[];
  onChange: (value: string) => void;
}> = ({ label, value, options, onChange }) => (
  <label className="select-field">
    <span>{label}</span>
    <select className="select" value={value} onChange={(e) => onChange(e.target.value)}>
      {options.map((o) => (
        <option key={o.id} value={o.id}>
          {o.label}
        </option>
      ))}
    </select>
  </label>
);

export const BrowsePage: React.FC = () => {
  const navigate = useNavigate();
  const [category, setCategory] = useState(cache?.category ?? 'discover');
  const [providerByCategory, setProviderByCategory] = useState<Record<string, string>>(cache?.providers ?? {});
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

  const cat = CATEGORIES.find((c) => c.id === category) ?? CATEGORIES[0];
  const provider = cat.providers.find((p) => p.id === filters.source) ?? cat.providers[0];

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
    cache = { category, providers: providerByCategory, filters, state };
  }, [category, providerByCategory, filters, state]);

  const loadMore = useCallback(() => {
    if (!loadingRef.current && stateRef.current.hasMore) fetchPage(false);
  }, [fetchPage]);

  const update = (patch: Partial<Filters>) => setFilters((f) => ({ ...f, ...patch }));
  const search = (q: string) => {
    setInput(q);
    update({ query: q.trim() });
  };

  /** Switch category (remembering which source was last used in it) and/or source. */
  const choose = (categoryId: string, providerId?: string) => {
    const next = CATEGORIES.find((c) => c.id === categoryId) ?? CATEGORIES[0];
    const pid = providerId ?? providerByCategory[categoryId] ?? next.providers[0].id;
    setCategory(categoryId);
    setProviderByCategory((m) => ({ ...m, [categoryId]: pid }));
    if (pid !== filters.source) {
      setInput('');
      update({ source: pid, query: '' });
    }
  };

  const categoryMask = filters.categories.split('');
  const toggleCategory = (i: number) => {
    const next = [...categoryMask];
    next[i] = next[i] === '1' ? '0' : '1';
    if (next.join('') !== '000') update({ categories: next.join('') });
  };

  const needsKey = useMemo(() => state.error?.includes('API key') ?? false, [state.error]);
  const emptyMessage = state.error
    ? 'Nothing to show'
    : filters.query
      ? `No results for “${filters.query}”. Try a shorter search, or switch the source above.`
      : 'Nothing here yet';

  return (
    <div className="browse page-enter">
      <header className="browse__head">
        <nav className="browse__tabs" role="tablist" aria-label="Wallpaper type">
          {CATEGORIES.map((c) => (
            <button
              key={c.id}
              role="tab"
              aria-selected={c.id === category}
              className={`browse__tab ${c.id === category ? 'is-active' : ''}`}
              onClick={() => choose(c.id)}
            >
              {c.label}
            </button>
          ))}
        </nav>
        {provider.placeholder ? (
          <div className="browse__search">
            <SearchBar value={input} onChange={setInput} onSearch={search} placeholder={provider.placeholder} />
          </div>
        ) : (
          <p className="browse__note">{cat.note}</p>
        )}
      </header>

      {(cat.providers.length > 1 || provider.filters) && (
        <div className="browse__toolbar">
          {cat.providers.length > 1 && (
            <div className="segmented" role="radiogroup" aria-label="Source">
              <span className="segmented__label">From</span>
              {cat.providers.map((p) => (
                <button
                  key={p.id}
                  role="radio"
                  aria-checked={p.id === provider.id}
                  className={`segmented__item ${p.id === provider.id ? 'is-active' : ''}`}
                  onClick={() => choose(category, p.id)}
                >
                  {p.label}
                </button>
              ))}
            </div>
          )}
          {provider.filters && (
            <div className="browse__filters">
              <Select label="Sort" value={filters.sorting} options={SORTS} onChange={(v) => update({ sorting: v })} />
              {filters.sorting === 'toplist' && (
                <Select label="Past" value={filters.topRange} options={RANGES} onChange={(v) => update({ topRange: v })} />
              )}
              <Select label="Size" value={filters.resolution} options={RESOLUTIONS} onChange={(v) => update({ resolution: v })} />
              {provider.filters === 'full' && (
                <div className="browse__toggles" role="group" aria-label="Include">
                  {CATEGORY_LABELS.map((c, i) => (
                    <button
                      key={c}
                      aria-pressed={categoryMask[i] === '1'}
                      className={`filter-chip ${categoryMask[i] === '1' ? 'filter-chip--active' : ''}`}
                      onClick={() => toggleCategory(i)}
                    >
                      {c}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {provider.suggestions && (
        <div className="browse__suggestions" aria-label="Quick searches">
          {provider.suggestions.map((s) => (
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
              Try again
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
        emptyMessage={emptyMessage}
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
