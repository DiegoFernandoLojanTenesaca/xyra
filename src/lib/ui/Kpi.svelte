<script lang="ts">
  // Cifra grande con su rótulo (partidas, winrate…). `rojo` la resalta. Los números suben desde 0 al aparecer.
  import { cubicOut } from 'svelte/easing';
  import { Tween } from 'svelte/motion';

  let { etiqueta, valor, rojo = false }: { etiqueta: string; valor: string | number; rojo?: boolean } = $props();

  // "55%" -> 55 y "%"; lo que no es número ("—") se muestra tal cual
  const partes = $derived(String(valor).match(/^(\d+)(%?)$/));
  const cifra = new Tween(0, { duration: 700, easing: cubicOut });
  $effect(() => {
    if (partes) cifra.target = +partes[1];
  });
</script>

<div class="panel corte aparece">
  <small>{etiqueta}</small>
  <b class="condensado" class:rojo>{partes ? `${Math.round(cifra.current)}${partes[2]}` : valor}</b>
</div>

<style>
  div {
    padding: 14px 16px;
  }
  small {
    display: block;
    font-size: 11px;
    letter-spacing: 2px;
    color: var(--suave);
    text-transform: uppercase;
    font-weight: 700;
  }
  b {
    display: block;
    margin-top: 6px;
    font-size: 38px;
    letter-spacing: 0;
  }
</style>
