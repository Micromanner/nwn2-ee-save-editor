import { useIntl } from 'react-intl';

export function useTranslations() {
  const intl = useIntl();
  
  return (key: string, values?: Record<string, unknown>) => {
    // Handle nested keys like 'app.title'
    const keys = key.split('.');
    let message: unknown = intl.messages;
    
    for (const k of keys) {
      if (message && typeof message === 'object' && k in message) {
        message = (message as Record<string, unknown>)[k];
      } else {
        console.warn(`Translation key not found: ${key}`);
        return key;
      }
    }
    
    if (typeof message === 'string') {
      return values ? intl.formatMessage({ id: key, defaultMessage: message }, values as Record<string, string | number>) : message;
    }
    
    return key;
  };
}