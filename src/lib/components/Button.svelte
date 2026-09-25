<script lang="ts">
  import type { Component, Snippet } from 'svelte';

  let {
    variant = 'default',
    icon: IconComponent,
    disabled = false,
    title,
    onclick,
    children,
  }: {
    variant?: 'default' | 'primary' | 'danger';
    icon?: Component<{ size?: number }>;
    disabled?: boolean;
    title?: string;
    onclick?: () => void;
    children: Snippet;
  } = $props();
</script>

<button class="button {variant}" {disabled} {title} {onclick}>
  {#if IconComponent}<IconComponent size={14} />{/if}
  {@render children()}
</button>

<style>
  .button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    font-size: var(--text-sm);
    font-weight: 700;
    letter-spacing: 2px;
    text-transform: uppercase;
    border: 1px solid var(--color-line);
    background: var(--color-panelRaised);
    transition: background 0.15s, border-color 0.15s, transform 0.15s;
    clip-path: polygon(var(--size-cutSm) 0, 100% 0, 100% calc(100% - var(--size-cutSm)), calc(100% - var(--size-cutSm)) 100%, 0 100%, 0 var(--size-cutSm));
  }
  .button:hover:not(:disabled) {
    border-color: var(--color-lineStrong);
    transform: translateY(-1px);
  }
  .primary {
    background: var(--color-accent);
    border-color: var(--color-accent);
  }
  .primary:hover:not(:disabled) {
    background: var(--color-accentBright);
  }
  .danger {
    border-color: color-mix(in srgb, var(--color-accent) 50%, transparent);
    color: var(--color-accentSoft);
  }
  .button:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
