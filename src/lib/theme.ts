import tokens from '$shared/tokens.json';
import type { Quality } from './types';

type TokenTree = { [key: string]: string | TokenTree };

/** Publishes every token as a CSS custom property: color.accent -> --color-accent. */
export function applyTokens(root: HTMLElement = document.documentElement) {
  const walk = (node: TokenTree, prefix: string) => {
    for (const [key, value] of Object.entries(node)) {
      if (typeof value === 'string') root.style.setProperty(`--${prefix}${key}`, value);
      else walk(value, `${prefix}${key}-`);
    }
  };
  walk(tokens as TokenTree, '');
}

export const qualityColor = (quality: Quality) => `var(--quality-${quality})`;

export const championTierColor = (tier: number | null) => `var(--championTier-${tier ?? 'none'})`;

export const rarityColor = (rarity: string) => `var(--rarity-${rarity})`;
