import React, { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Sidebar } from './Sidebar';
import { Titlebar } from './Titlebar';
import ToastContainer from '../common/Toast';
import { api, applyTheme } from '../../api';
import { useAppStore } from '../../store/appStore';
import './Layout.css';

interface LayoutProps {
  children: React.ReactNode;
}

export const Layout: React.FC<LayoutProps> = ({ children }) => {
  useEffect(() => {
    api.getSettings().then((s) => applyTheme(s.theme)).catch(console.error);
    useAppStore.getState().loadFavorites();

    const unlisten = listen<{ id: string; received: number; total: number | null }>('download-progress', (e) =>
      useAppStore.getState().setProgress(e.payload.id, e.payload.received, e.payload.total),
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return (
    <div className="layout" id="app-layout">
      <Titlebar />
      <div className="layout__body">
        <Sidebar />
        <main className="layout__content">
          <div className="layout__content-inner">{children}</div>
        </main>
      </div>
      <ToastContainer />
    </div>
  );
};
