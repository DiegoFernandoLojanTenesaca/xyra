import tokens from '$design/tokens.json';
import type { Quality, Rarity } from '../types';

type TokenTree = { [key: string]: string | TokenTree };

const ALIAS = /^\{(.+)\}$/;

const property = (path: string) => `--${path.replaceAll('.', '-')}`;

export function applyTokens(root: HTMLElement = document.documentElement) {
  const walk = (node: TokenTree, prefix: string) => {
    for (const [key, value] of Object.entries(node)) {
      if (typeof value !== 'string') walk(value, `${prefix}${key}.`);
      else root.style.setProperty(property(`${prefix}${key}`), value.replace(ALIAS, (_, alias) => `var(${property(alias)})`));
    }
  };
  walk(tokens as TokenTree, '');
}

export const qualityColor = (quality: Quality) => `var(--quality-${quality})`;

export const championTierColor = (tier: number | null) => `var(--championTier-${tier ?? 'none'})`;

export const rarityColor = (rarity: Rarity) => `var(--rarity-${rarity})`;
