import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Layout } from './components/Layout/Layout';
import { BrowsePage } from './pages/Browse/BrowsePage';
import { LocalPage } from './pages/Local/LocalPage';
import { FavoritesPage } from './pages/Favorites/FavoritesPage';
import { HistoryPage } from './pages/History/HistoryPage';
import { SchedulePage } from './pages/Schedule/SchedulePage';
import { SettingsPage } from './pages/Settings/SettingsPage';
import './components/common/common.css';
import './App.css';

function App() {
  return (
    <HashRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Navigate to="/browse" replace />} />
          <Route path="/browse" element={<BrowsePage />} />
          <Route path="/local" element={<LocalPage />} />
          <Route path="/favorites" element={<FavoritesPage />} />
          <Route path="/history" element={<HistoryPage />} />
          <Route path="/schedule" element={<SchedulePage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/browse" replace />} />
        </Routes>
      </Layout>
    </HashRouter>
  );
}

export default App;
