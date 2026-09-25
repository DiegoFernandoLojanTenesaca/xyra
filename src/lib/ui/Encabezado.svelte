<script lang="ts">
  // Cabecera de página: antetítulo rojo, título grande condensado, subtítulo y acciones a la derecha.
  import type { Snippet } from 'svelte';

  let { titulo, sub = '', eyebrow = '', children }: { titulo: string; sub?: string; eyebrow?: string; children?: Snippet } = $props();
</script>

<header>
  <div>
    {#if eyebrow}<span class="eyebrow">{eyebrow}</span>{/if}
    <h1 class="condensado">{titulo}<span class="rojo">.</span></h1>
    {#if sub}<p class="suave">{sub}</p>{/if}
  </div>
  {#if children}<div class="acciones">{@render children()}</div>{/if}
</header>

<style>
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 22px;
  }
  h1 {
    margin: 2px 0 0;
    font-size: 42px;
  }
  p {
    margin: 8px 0 0;
    font-size: 13px;
    letter-spacing: 0.5px;
  }
  .acciones {
    display: flex;
    gap: 10px;
  }
</style>
