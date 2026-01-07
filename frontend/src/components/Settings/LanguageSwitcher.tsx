
import { useLocale } from '@/providers/LocaleProvider';
import { useState } from 'react';
import { Button } from '@/components/ui/Button';

const languages = [
  { code: 'en', name: 'English', flag: '🇬🇧' },
  { code: 'de', name: 'Deutsch', flag: '🇩🇪' },
  { code: 'fr', name: 'Français', flag: '🇫🇷' },
  { code: 'es', name: 'Español', flag: '🇪🇸' },
  { code: 'it', name: 'Italiano', flag: '🇮🇹' },
  { code: 'pl', name: 'Polski', flag: '🇵🇱' },
  { code: 'ru', name: 'Русский', flag: '🇷🇺' },
  { code: 'zh', name: '中文', flag: '🇨🇳' },
  { code: 'ja', name: '日本語', flag: '🇯🇵' },
  { code: 'ko', name: '한국어', flag: '🇰🇷' },
];

export default function LanguageSwitcher() {
  const { locale, setLocale } = useLocale();
  const [isOpen, setIsOpen] = useState(false);

  const currentLanguage = languages.find(lang => lang.code === locale) || languages[0];

  const handleLanguageChange = (newLocale: string) => {
    setLocale(newLocale);
    setIsOpen(false);
  };

  return (
    <div className="relative">
      <Button
        onClick={() => setIsOpen(!isOpen)}
        variant="ghost"
        size="sm"
        className="gap-1"
      >
        <span className="text-lg">{currentLanguage.flag}</span>
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </Button>
      
      {isOpen && (
        <div className="absolute top-full right-0 mt-1 w-48 bg-[rgb(var(--color-surface-2))] rounded-md shadow-elevation-3 border border-[rgb(var(--color-surface-border)/0.6)] z-50">
          {languages.map((lang) => (
            <Button
              key={lang.code}
              onClick={() => handleLanguageChange(lang.code)}
              variant="ghost"
              size="sm"
              className={`w-full justify-start rounded-none gap-2 ${
                lang.code === locale ? 'bg-[rgb(var(--color-surface-3))] text-[rgb(var(--color-primary))] font-medium' : ''
              }`}
            >
              <span className="text-lg">{lang.flag}</span>
              <span className="text-sm">{lang.name}</span>
            </Button>
          ))}
        </div>
      )}
    </div>
  );
}