import { BASE_LANGUAGE, formatter, LANGUAGES, translator } from '$shared/i18n';
import type { BuildMode, Position } from '$shared/types';
import { link, LinkError } from './link.svelte';

export const TABS = ['live', 'build', 'meta', 'stats', 'settings'] as const;
export type Tab = (typeof TABS)[number];

const deviceLanguage = () => navigator.language.split('-')[0];

/** The address can open a tab, and Build on a champion: #meta, #build/222. */
const [linkedTab, linkedChampion] = location.hash.slice(1).split('/');

/** What the phone app shows: the tab, the champion open in Build and the texts in the PC's language. */
class Mobile {
  tab = $state<Tab>(TABS.find((tab) => tab === linkedTab) ?? 'live');
  buildChampion = $state<number | null>(Number(linkedChampion) || null);
  buildMode = $state<BuildMode | null>(null);
  buildPosition = $state<Position | null>(null);
  language = $derived(link.pc?.language ?? (LANGUAGES.includes(deviceLanguage()) ? deviceLanguage() : BASE_LANGUAGE));
  t = $derived(translator(this.language));
  format = $derived(formatter(this.language));

  openBuild = (champion: number, mode: BuildMode | null = null, position: Position | null = null) => {
    this.buildChampion = champion;
    this.buildMode = mode;
    this.buildPosition = position;
    this.tab = 'build';
  };

  /** A failure the PC reported, or that it could not be reached. */
  errorText = (error: unknown) => {
    if (!(error instanceof LinkError)) return this.t('mobile:unreachable');
    const failure = error.failure;
    return this.t(`errors:${failure.code}`, { detail: 'detail' in failure ? failure.detail : '' });
  };

  /** Runs an action on the PC and returns the text to show about it. */
  act = async (method: 'GET' | 'POST', path: string, params: Record<string, string> = {}, done = 'mobile:done') => {
    try {
      await link.call(method, path, params);
      return this.t(done);
    } catch (error) {
      return this.errorText(error);
    }
  };
}

export const mobile = new Mobile();
