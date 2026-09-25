<script lang="ts">
  // Buscador de campeón con sugerencias: elegir uno cambia el campeón de Build y Aumentos (app.elegido).
  import { app } from '../app.svelte';
  import { tierCampeon } from '../i18n';
  import type { CampeonInfo } from '../tipos';
  import Buscador from './Buscador.svelte';
  import Tier from './Tier.svelte';

  let { placeholder }: { placeholder: string } = $props();
  let busqueda = $state('');
  let abierto = $state(false);

  const sugerencias = $derived(
    busqueda.trim() ? app.campeones.filter((c) => c.nombre.toLowerCase().includes(busqueda.trim().toLowerCase())).slice(0, 8) : [],
  );

  function elegir(c: CampeonInfo) {
    app.elegido = c.id;
    busqueda = '';
    abierto = false;
  }
</script>

<Buscador bind:valor={busqueda} {placeholder} onfocus={() => (abierto = true)} onblur={() => setTimeout(() => (abierto = false), 150)}>
  {#if abierto && sugerencias.length}
    <ul>
      {#each sugerencias as c (c.id)}
        {@const [etiqueta, color] = tierCampeon(c.tier)}
        <li>
          <button onclick={() => elegir(c)}>
            <img src={c.icono} alt="" />
            <span>{c.nombre}</span>
            <Tier texto={etiqueta} {color} tam={22} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</Buscador>

<style>
  ul {
    position: absolute;
    top: 42px;
    left: 0;
    right: 0;
    z-index: 5;
    list-style: none;
    margin: 0;
    padding: 6px;
    background: var(--panel-2);
    border: 1px solid var(--linea);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.6);
  }
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 8px;
    border: none;
    background: none;
    text-align: left;
  }
  button:hover {
    background: rgba(229, 19, 43, 0.14);
  }
  img {
    width: 28px;
    height: 28px;
  }
  span {
    flex: 1;
  }
</style>
