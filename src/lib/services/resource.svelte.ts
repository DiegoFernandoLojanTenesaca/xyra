import { untrack } from 'svelte';
import type { AppError } from '../types';
import { toAppError } from './errors';

/**
 * Loads `load(key)` again whenever the key changes; only the latest request is kept, and failures go through `toError`.
 * Keys are compared by content, so a state update that rebuilds an equal key keeps what is loaded instead of reloading it.
 */
export function resource<K, T, E = AppError>(
  key: () => K | null,
  load: (key: K) => Promise<T>,
  toError: (failure: unknown) => E = toAppError as (failure: unknown) => E,
) {
  let value = $state<T | null>(null);
  let error = $state<E | null>(null);
  let latest = 0;
  let loaded: string | undefined;
  $effect(() => {
    const current = key();
    const same = JSON.stringify(current);
    if (same === loaded) return;
    loaded = same;
    const request = ++latest;
    value = null;
    error = null;
    if (current === null) return;
    untrack(() => load(current)).then(
      (result) => {
        if (request === latest) value = result;
      },
      (failure) => {
        if (request === latest) error = toError(failure);
      },
    );
  });
  return {
    get value() {
      return value;
    },
    get error() {
      return error;
    },
  };
}
