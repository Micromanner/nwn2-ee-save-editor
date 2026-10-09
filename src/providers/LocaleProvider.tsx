
import React, { createContext, useContext, useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { IntlProvider } from 'react-intl';
import { setDateLocale } from '@/utils/dateFormat';

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

const flattenMessages = (nestedMessages: any, prefix = '') => {
  return Object.keys(nestedMessages).reduce((messages: any, key) => {
    const value = nestedMessages[key];
    const prefixedKey = prefix ? `${prefix}.${key}` : key;

    if (typeof value === 'string') {
      messages[prefixedKey] = value;
    } else {
      Object.assign(messages, flattenMessages(value, prefixedKey));
    }

    return messages;
  }, {});
};

const loadMessages = async (locale: string) => {
  try {
    const messages = await import(`../../i18n/${locale}.json`);
    return flattenMessages(messages.default);
  } catch (error) {
    console.error(`Failed to load messages for locale: ${locale}`, error);
    const messages = await import('../../i18n/en.json');
    return flattenMessages(messages.default);
  }
};

export function LocaleProvider({ children }: { children: React.ReactNode }) {
  const [locale, setLocaleState] = useState('en');
  const [messages, setMessages] = useState<Record<string, unknown>>({});

  useEffect(() => {
    invoke<{ language: string; system_locale: string }>('get_app_config')
      .then((config) => {
        setDateLocale(config.system_locale);
        setLocaleState(config.language);
        return loadMessages(config.language);
      })
      .catch(() => loadMessages('en'))
      .then((msgs) => {
        setMessages(msgs);
      });
  }, []);

  const setLocale = async (newLocale: string) => {
    const newMessages = await loadMessages(newLocale);
    setMessages(newMessages);
    setLocaleState(newLocale);
  };

  return (
    <LocaleContext.Provider value={{ locale, setLocale, messages }}>
      <IntlProvider messages={messages as Record<string, string>} locale={locale}>
        {children}
      </IntlProvider>
    </LocaleContext.Provider>
  );
}