import type { Metadata } from "next";
import "./globals.css";
import { LocaleProvider } from '@/providers/LocaleProvider';
import { TauriProvider } from '@/providers/TauriProvider';
import { ThemeProvider } from '@/providers/ThemeProvider';
import { SettingsProvider } from '@/contexts/SettingsContext';
import { ToastProvider } from '@/contexts/ToastContext';

export const metadata: Metadata = {
  title: "NWN2 Save Editor",
  description: "Modern save editor for Neverwinter Nights 2 Enhanced Edition",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="h-screen overflow-hidden bg-[rgb(var(--color-background))] text-[rgb(var(--color-text-primary))]">
        <TauriProvider>
          <ThemeProvider>
            <SettingsProvider>
              <LocaleProvider>
                <ToastProvider>
                  {children}
                </ToastProvider>
              </LocaleProvider>
            </SettingsProvider>
          </ThemeProvider>
        </TauriProvider>
      </body>
    </html>
  );
}