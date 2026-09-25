<script lang="ts" generics="V extends string">
  let { options, value = $bindable(), tabs = false }: { options: [V, string][]; value: V; tabs?: boolean } = $props();
</script>

<div class:tabs role="tablist">
  {#each options as [id, label] (id)}
    <button role="tab" aria-selected={value === id} class:active={value === id} onclick={() => (value = id)}>{label}</button>
  {/each}
</div>

<style>
  div {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--color-panel);
    border: 1px solid var(--color-line);
  }
  button {
    padding: var(--space-2) var(--space-4);
    border: none;
    background: none;
    color: var(--color-textMuted);
    font-size: var(--text-sm);
    font-weight: 700;
    letter-spacing: 1.5px;
    text-transform: uppercase;
  }
  button:hover {
    color: var(--color-text);
  }
  button.active {
    background: var(--color-accent);
    color: var(--color-white);
  }
  .tabs {
    gap: var(--space-1);
    padding: 0;
    margin-bottom: var(--space-6);
    background: none;
    border: none;
    border-bottom: 1px solid var(--color-line);
  }
  .tabs button {
    position: relative;
    padding: var(--space-3) var(--space-4);
  }
  .tabs button.active {
    background: none;
  }
  .tabs button.active::after {
    content: '';
    position: absolute;
    left: var(--space-3);
    right: var(--space-3);
    bottom: -1px;
    height: 3px;
    background: var(--color-accentBright);
    box-shadow: 0 0 var(--space-3) var(--color-accent);
  }
</style>
