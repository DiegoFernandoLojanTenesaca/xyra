<script lang="ts">
  import { app } from '../app.svelte';
  import { tierCampeon } from '../i18n';
  import Buscador from '../ui/Buscador.svelte';
  import Encabezado from '../ui/Encabezado.svelte';
  import Tier from '../ui/Tier.svelte';
  import Vacio from '../ui/Vacio.svelte';

  const T = $derived(app.T);
  let busqueda = $state('');

  const filtrados = $derived(
    app.campeones.filter((c) => !busqueda.trim() || c.nombre.toLowerCase().includes(busqueda.trim().toLowerCase())),
  );
  const niveles = $derived(
    [1, 2, 3, 4, 5, null].map((t) => ({ tier: t, lista: filtrados.filter((c) => c.tier === t) })).filter((n) => n.lista.length),
  );
</script>

<Encabezado titulo={T.nav.campeones} sub={T.campeones.subtitulo}>
  <Buscador bind:valor={busqueda} placeholder={T.campeones.buscar} />
</Encabezado>

{#if !app.campeones.length}
  <Vacio icono="campeones" texto={T.campeones.sin_catalogo} />
{/if}

{#each niveles as n (n.tier)}
  {@const [etiqueta, color] = tierCampeon(n.tier)}
  <section class="nivel panel corte" style="--c:{color}">
    <div class="etiqueta">
      <Tier texto={etiqueta} {color} tam={46} />
      <small>{n.lista.length}</small>
    </div>
    <div class="grilla">
      {#each n.lista as c (c.id)}
        <button class="campeon" onclick={() => app.verAumentos(c.id)} title="{c.nombre} #{c.puesto ?? '—'}">
          <img src={c.icono} alt="" loading="lazy" />
          <span>{c.nombre}</span>
        </button>
      {/each}
    </div>
  </section>
{/each}

<style>
  .nivel {
    display: flex;
    gap: 18px;
    padding: 14px;
    margin-bottom: 12px;
    border-left: 3px solid var(--c);
  }
  .etiqueta {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 8px;
    gap: 8px;
    width: 64px;
    flex: none;
  }
  .etiqueta small {
    color: var(--suave);
  }
  .grilla {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(76px, 1fr));
    gap: 10px;
    flex: 1;
  }
  .campeon {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: 6px 2px;
    border: 1px solid transparent;
    background: none;
    transition: background 0.15s, border-color 0.15s;
  }
  .campeon:hover {
    background: rgba(229, 19, 43, 0.1);
    border-color: rgba(229, 19, 43, 0.4);
  }
  .campeon img {
    width: 52px;
    height: 52px;
  }
  .campeon span {
    font-size: 11px;
    color: var(--suave);
    max-width: 74px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
