
import React, { createContext, useContext, useState, useEffect } from 'react';
import { IntlProvider } from 'react-intl';

type LocaleContextType = {
  locale: string;
  setLocale: (locale: string) => void;
  messages: Record<string, unknown>;
};

const LocaleContext = createContext<LocaleContextType | undefined>(undefined);

export const useLocale = () => {
  const context = useContext(LocaleContext);
  if (!context) {
    throw new Error('useLocale must be used within LocaleProvider');
  }
  return context;
};

const loadMessages = async (locale: string) => {
  try {
    const messages = await import(`../../i18n/${locale}.json`);
    return messages.default;
  } catch (error) {
    console.error(`Failed to load messages for locale: ${locale}`, error);
    const messages = await import('../../i18n/en.json');
    return messages.default;
  }
};

export function LocaleProvider({ children }: { children: React.ReactNode }) {
  const [locale, setLocaleState] = useState('en');
  const [messages, setMessages] = useState<Record<string, unknown>>({});
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const savedLocale = localStorage.getItem('locale') || 'en';
    setLocaleState(savedLocale);
    
    loadMessages(savedLocale).then((msgs) => {
      setMessages(msgs);
      setIsLoading(false);
    });
  }, []);

  const setLocale = async (newLocale: string) => {
    setIsLoading(true);
    const newMessages = await loadMessages(newLocale);
    setMessages(newMessages);
    setLocaleState(newLocale);
    localStorage.setItem('locale', newLocale);
    setIsLoading(false);
  };

  return (
    <LocaleContext.Provider value={{ locale, setLocale, messages }}>
      <IntlProvider messages={messages as Record<string, string>} locale={locale}>
        {children}
      </IntlProvider>
    </LocaleContext.Provider>
  );
}