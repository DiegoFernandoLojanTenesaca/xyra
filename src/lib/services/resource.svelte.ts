import { untrack } from 'svelte';
import type { AppError } from '../types';
import { toAppError } from './errors';

/** Loads `load(key)` each time `key()` changes, keeping only the answer to the latest key; call it while a component initializes. */
export function resource<K, T>(key: () => K | null, load: (key: K) => Promise<T>) {
  let value = $state<T | null>(null);
  let error = $state<AppError | null>(null);
  let latest = 0;
  $effect(() => {
    const current = key();
    const request = ++latest;
    value = null;
    error = null;
    if (current === null) return;
    untrack(() => load(current)).then(
      (result) => {
        if (request === latest) value = result;
      },
      (failure) => {
        if (request === latest) error = toAppError(failure);
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
