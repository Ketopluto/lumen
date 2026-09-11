import React from 'react';
import { NavLink } from 'react-router-dom';
import { useUIStore } from '../../store/uiStore';
import './Sidebar.css';

const navItems = [
  { path: '/browse', label: 'Browse', icon: '🌐' },
  { path: '/local', label: 'Local', icon: '📁' },
  { path: '/favorites', label: 'Favorites', icon: '⭐' },
  { path: '/history', label: 'History', icon: '🕐' },
  { path: '/schedule', label: 'Schedule', icon: '⏱️' },
  { path: '/settings', label: 'Settings', icon: '⚙️' },
];

export const Sidebar: React.FC = () => {
  const { sidebarCollapsed, toggleSidebar } = useUIStore();

  return (
    <aside
      className={`sidebar ${sidebarCollapsed ? 'sidebar--collapsed' : ''}`}
      id="sidebar"
    >
      <nav className="sidebar__nav">
        {navItems.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            className={({ isActive }) =>
              `sidebar__link ${isActive ? 'sidebar__link--active' : ''}`
            }
            id={`nav-${item.path.slice(1)}`}
          >
            <span className="sidebar__icon">{item.icon}</span>
            {!sidebarCollapsed && (
              <span className="sidebar__label">{item.label}</span>
            )}
          </NavLink>
        ))}
      </nav>

      <div className="sidebar__footer">
        <button
          className="sidebar__toggle"
          onClick={toggleSidebar}
          id="btn-toggle-sidebar"
          aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 16 16"
            fill="none"
            style={{ transform: sidebarCollapsed ? 'rotate(180deg)' : 'none', transition: 'transform 200ms ease' }}
          >
            <path
              d="M10 3L5 8l5 5"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
      </div>
    </aside>
  );
};
