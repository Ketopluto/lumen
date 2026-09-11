import React from 'react';
import { NavLink } from 'react-router-dom';
import { useUIStore } from '../../store/uiStore';
import './Sidebar.css';

const ICONS: Record<string, React.ReactNode> = {
  discover: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M15.3 8.7l-1.9 4.7-4.7 1.9 1.9-4.7z" />
    </>
  ),
  files: <path d="M3.5 7.5a2 2 0 0 1 2-2h3.6l2 2h7.4a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" />,
  favorites: <path d="M12 19.5s-7-4.3-7-9.6A3.9 3.9 0 0 1 12 7.6a3.9 3.9 0 0 1 7 2.3c0 5.3-7 9.6-7 9.6z" />,
  history: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.5V12l3 2" />
    </>
  ),
  slideshow: (
    <>
      <rect x="3.5" y="5.5" width="13" height="10" rx="2" />
      <path d="M7.5 18.5h11a2 2 0 0 0 2-2v-8" />
      <path d="M8.8 8.3v4.4l3.6-2.2z" />
    </>
  ),
  settings: (
    <>
      <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
      <circle cx="15" cy="7" r="2" />
      <circle cx="9" cy="17" r="2" />
    </>
  ),
};

const NAV = [
  { path: '/browse', label: 'Discover', icon: 'discover' },
  { path: '/local', label: 'My files', icon: 'files' },
  { path: '/favorites', label: 'Favorites', icon: 'favorites' },
  { path: '/history', label: 'History', icon: 'history' },
  { path: '/schedule', label: 'Slideshow', icon: 'slideshow' },
  { path: '/settings', label: 'Settings', icon: 'settings' },
];

export const Sidebar: React.FC = () => {
  const { sidebarCollapsed, toggleSidebar } = useUIStore();

  return (
    <aside className={`sidebar ${sidebarCollapsed ? 'sidebar--collapsed' : ''}`} id="sidebar">
      <nav className="sidebar__nav" aria-label="Main">
        {NAV.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            title={sidebarCollapsed ? item.label : undefined}
            className={({ isActive }) => `sidebar__link ${isActive ? 'sidebar__link--active' : ''}`}
          >
            <svg
              className="sidebar__icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.7"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              {ICONS[item.icon]}
            </svg>
            {!sidebarCollapsed && <span className="sidebar__label">{item.label}</span>}
          </NavLink>
        ))}
      </nav>

      <div className="sidebar__footer">
        <button
          className="sidebar__toggle"
          onClick={toggleSidebar}
          aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 16 16"
            fill="none"
            style={{ transform: sidebarCollapsed ? 'rotate(180deg)' : 'none', transition: 'transform 200ms ease' }}
          >
            <path d="M10 3L5 8l5 5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
      </div>
    </aside>
  );
};
