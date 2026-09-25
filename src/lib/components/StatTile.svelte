<script lang="ts">
  import { cubicOut } from 'svelte/easing';
  import { Tween } from 'svelte/motion';

  let { label, value, accent = false }: { label: string; value: string | number; accent?: boolean } = $props();

  const numeric = $derived(String(value).match(/^(\d+)(%?)$/));
  const counter = new Tween(0, { duration: 700, easing: cubicOut });
  $effect(() => {
    if (numeric) counter.target = +numeric[1];
  });
</script>

<div class="panel cut appear">
  <small>{label}</small>
  <b class="condensed" class:accent>{numeric ? `${Math.round(counter.current)}${numeric[2]}` : value}</b>
</div>

<style>
  div {
    padding: var(--space-4);
  }
  small {
    display: block;
    font-size: var(--text-xs);
    letter-spacing: 2px;
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
