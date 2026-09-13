let dateLocale: string | undefined;

/// Locale for date output, set once from the OS regional format reported by
/// `get_app_config`. The webview's own default follows the UI language, which
/// is not the format the user picked in the OS.
export function setDateLocale(locale: string) {
  try {
    Intl.DateTimeFormat(locale);
    dateLocale = locale;
  } catch {
    dateLocale = undefined;
  }
}

export function formatDateTime(value: Date | number | string): string {
  const date = value instanceof Date ? value : new Date(value);
  if (isNaN(date.getTime())) return '-';
  return date.toLocaleString(dateLocale);
}

export function formatDate(value: Date | number | string): string {
  const date = value instanceof Date ? value : new Date(value);
  if (isNaN(date.getTime())) return '-';
  return date.toLocaleDateString(dateLocale);
}
