import { BarChart3, Home, Sprout, Store, WalletCards } from 'lucide-react';
import { NavLink, Outlet } from 'react-router-dom';

const links = [
  { to: '/wallet', label: 'Wallet', icon: WalletCards },
  { to: '/farmer', label: 'Farmer', icon: Sprout },
  { to: '/buyer', label: 'Buyer', icon: Store },
  { to: '/admin', label: 'Analytics', icon: BarChart3 },
];

export function AppShell() {
  return (
    <div className="min-h-screen bg-[#F7F4EC] text-[#172018]">
      <header className="sticky top-0 z-20 border-b border-[#DAD2BE] bg-[#F7F4EC]/95 backdrop-blur">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-3">
          <NavLink to="/" className="flex items-center gap-2 font-semibold text-[#1B4332]">
            <Home size={20} />
            AgriTrust
          </NavLink>
          <nav className="flex gap-1 overflow-x-auto">
            {links.map(({ to, label, icon: Icon }) => (
              <NavLink
                key={to}
                to={to}
                className={({ isActive }) =>
                  [
                    'flex items-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition',
                    isActive
                      ? 'bg-[#1B4332] text-white'
                      : 'text-[#4A5649] hover:bg-white hover:text-[#1B4332]',
                  ].join(' ')
                }
              >
                <Icon size={16} />
                <span className="hidden sm:inline">{label}</span>
              </NavLink>
            ))}
          </nav>
        </div>
      </header>
      <Outlet />
    </div>
  );
}
