import { derived } from 'svelte/store';
import { locale } from '../i18n';
import { shellMessages, type ShellMessageKey } from './messages';

export type TranslationParams = Record<string, string | number>;

function interpolate(template: string, params: TranslationParams) {
  return template.replace(/\{([a-zA-Z0-9_]+)\}/g, (match, key: string) => {
    const value = params[key];
    return value === undefined ? match : String(value);
  });
}

export const translator = derived(locale, ($locale) => {
  return (key: ShellMessageKey, params: TranslationParams = {}) => {
    const template = shellMessages[$locale][key] ?? shellMessages.en[key];
    return interpolate(template, params);
  };
});
