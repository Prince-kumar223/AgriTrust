import { Navigate, Route, Routes } from 'react-router-dom';

import { AppShell } from './components/AppShell';
import { AdminAnalyticsPage } from './pages/AdminAnalyticsPage';
import { BuyerDashboardPage } from './pages/BuyerDashboardPage';
import { FarmerDashboardPage } from './pages/FarmerDashboardPage';
import { LandingPage } from './pages/LandingPage';
import { TradeDetailPage } from './pages/TradeDetailPage';
import { WalletPage } from './pages/WalletPage';

function App() {
  return (
    <Routes>
      <Route path="/" element={<LandingPage />} />
      <Route element={<AppShell />}>
        <Route path="/wallet" element={<WalletPage />} />
        <Route path="/farmer" element={<FarmerDashboardPage />} />
        <Route path="/buyer" element={<BuyerDashboardPage />} />
        <Route path="/trades/:tradeId" element={<TradeDetailPage />} />
        <Route path="/admin" element={<AdminAnalyticsPage />} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

export default App;
