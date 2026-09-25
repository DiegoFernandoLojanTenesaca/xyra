<script lang="ts">
  // Campo de búsqueda con lupa. Lo demás (placeholder, onfocus…) pasa directo al <input>.
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';
  import Icono from './Icono.svelte';

  let { valor = $bindable(''), children, ...resto }: { valor?: string; children?: Snippet } & HTMLInputAttributes = $props();
</script>

<div>
  <Icono nombre="buscar" tam={16} />
  <input type="search" bind:value={valor} {...resto} />
  {@render children?.()}
</div>

<style>
  div {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 240px;
    max-width: 420px;
  }
  div > :global(svg) {
    position: absolute;
    left: 12px;
    color: var(--suave);
  }
  input {
    width: 100%;
    padding-left: 36px;
  }
</style>
