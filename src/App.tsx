import { Routes, Route } from 'react-router-dom';
import { LocaleProvider } from '@/providers/LocaleProvider';
import { TauriProvider } from '@/providers/TauriProvider';
import { ThemeProvider } from '@/providers/ThemeProvider';
import { FontSizeProvider } from '@/providers/FontSizeProvider';
import { ToastProvider } from '@/contexts/ToastContext';
import { PortalProvider } from '@blueprintjs/core';
import Shell from '@/components/layout/Shell';

function AppProviders({ children }: { children: React.ReactNode }) {
  return (
    <TauriProvider>
      <ThemeProvider>
        <FontSizeProvider>
          <LocaleProvider>
            <PortalProvider portalClassName="bp-app">
              <ToastProvider>
                {children}
              </ToastProvider>
            </PortalProvider>
          </LocaleProvider>
        </FontSizeProvider>
      </ThemeProvider>
    </TauriProvider>
  );
}

export default function App() {
  return (
    <AppProviders>
      <Routes>
        <Route path="/" element={<Shell />} />
      </Routes>
    </AppProviders>
  );
}
