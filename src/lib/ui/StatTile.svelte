<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { Tween } from 'svelte/motion';

  const COUNT_UP_MS = 700;
  const EMPTY = '—';

  let {
    label,
    value,
    format = String,
    accent = false,
  }: { label: string; value: number | null; format?: (value: number) => string; accent?: boolean } = $props();

  const counter = new Tween(0, { duration: COUNT_UP_MS, easing: cubicOut });

  $effect(() => {
    if (value !== null) counter.target = value;
  });
</script>

<div class="panel cut appear">
  <small>{label}</small>
  <b class="condensed" class:accent>{value === null ? EMPTY : format(Math.round(counter.current))}</b>
</div>

<style>
  div {
    padding: var(--space-4);
  }
  small {
    display: block;
    font-size: var(--text-xs);
    letter-spacing: var(--tracking-wide);
    color: var(--color-textMuted);
    text-transform: uppercase;
    font-weight: 700;
  }
  b {
    display: block;
    margin-top: var(--space-2);
    font-size: var(--text-3xl);
    letter-spacing: 0;
  }
</style>
