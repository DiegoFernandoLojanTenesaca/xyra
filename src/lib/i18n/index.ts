import i18next, { type TFunction } from 'i18next';

type Namespace = Record<string, unknown>;

export const BASE_LANGUAGE = 'en';

const files = import.meta.glob<Namespace>('$locales/*/*.json', { eager: true, import: 'default' });

const resources: Record<string, Record<string, Namespace>> = {};
for (const [path, content] of Object.entries(files)) {
  const [, language, namespace] = path.match(/locales\/([^/]+)\/([^/]+)\.json$/) ?? [];
  if (language && namespace) (resources[language] ??= {})[namespace] = content;
}

export const LANGUAGES = Object.keys(resources);

i18next.init({
  resources,
  lng: BASE_LANGUAGE,
  fallbackLng: BASE_LANGUAGE,
  defaultNS: 'common',
  ns: Object.keys(resources[BASE_LANGUAGE] ?? {}),
  interpolation: { escapeValue: false },
  returnObjects: true,
  initAsync: false,
});

export const translator = (language: string): TFunction => i18next.getFixedT(language);

export const languageName = (language: string) => translator(language)('common:languageName');

export { formatter, type Formatter } from './format';

/** Splits a translated sentence around an interpolated value so the value can be emphasized. */
export function around(text: string, value: string): [string, string] {
  const index = text.indexOf(value);
  return index < 0 ? [text, ''] : [text.slice(0, index), text.slice(index + value.length)];
}
