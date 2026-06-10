import en from './en';
import zh from './zh';
import { store } from '../stores.svelte';

const messages: Record<string, Record<string, string>> = { en, zh };

export type Locale = 'en' | 'zh';

export function detectLocale(): Locale {
  const lang = navigator.language.toLowerCase();
  if (lang.startsWith('zh')) return 'zh';
  return 'en';
}

export function t(key: string, params?: Record<string, string | number>): string {
  const locale = store.locale;
  const dict = messages[locale] ?? messages.en;
  let text = dict[key] ?? messages.en[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      text = text.replace(`{${k}}`, String(v));
    }
  }
  return text;
}
