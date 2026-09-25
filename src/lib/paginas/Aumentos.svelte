<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { untrack } from 'svelte';
  import { app } from '../app.svelte';
  import { calidad } from '../i18n';
  import type { AumentoFila } from '../tipos';
  import CabeceraCampeon from '../ui/CabeceraCampeon.svelte';
  import Encabezado from '../ui/Encabezado.svelte';
  import Segmentos from '../ui/Segmentos.svelte';
  import SelectorCampeon from '../ui/SelectorCampeon.svelte';
  import Tier from '../ui/Tier.svelte';
  import Vacio from '../ui/Vacio.svelte';

  const T = $derived(app.T);

  // el modo arranca en el de la partida en curso (solo al abrir la página)
  let modo = $state<'KIWI' | 'CHERRY'>(untrack(() => app.estado.modo) === 'CHERRY' ? 'CHERRY' : 'KIWI');
  let rareza = $state<'' | 'kPrismatic' | 'kGold' | 'kSilver'>('');
  let filas = $state<AumentoFila[] | null>(null);
  let error = $state(false);

  $effect(app.campeonPorDefecto);

  // datos de OP.GG cada vez que cambian el campeón o el modo
  $effect(() => {
    const id = app.elegido;
    const m = modo;
    if (id === null) return;
    filas = null;
    error = false;
    invoke<AumentoFila[]>('get_aumentos', { campeon: id, modo: m })
      .then((f) => {
        if (id === app.elegido && m === modo) filas = f;
      })
      .catch(() => (error = true));
  });

  const actual = $derived(app.campeones.find((c) => c.id === app.elegido) ?? null);
  const visibles = $derived((filas ?? []).filter((f) => !rareza || f.rareza === rareza));
  const grupos = $derived(
    [...new Set(visibles.map((f) => f.tier))].sort((a, b) => a - b).map((t) => ({ tier: t, filas: visibles.filter((f) => f.tier === t) })),
  );
  const maxPopular = $derived(Math.max(1, ...(filas ?? []).map((f) => f.popular)));
</script>

<Encabezado titulo={T.nav.aumentos} sub={T.aumentos.subtitulo} />

<div class="herramientas">
  <SelectorCampeon placeholder={T.aumentos.buscar} />
  <Segmentos opciones={[['KIWI', T.modos.KIWI], ['CHERRY', T.modos.CHERRY]]} bind:valor={modo} />
  <Segmentos
    opciones={[['', T.aumentos.todas], ['kPrismatic', T.rarezas.kPrismatic], ['kGold', T.rarezas.kGold], ['kSilver', T.rarezas.kSilver]]}
    bind:valor={rareza}
  />
</div>

{#if actual}
  <CabeceraCampeon campeon={actual} detalle="{T.modos[modo]}{filas ? ` · ${filas.length}` : ''}" />
{/if}

{#if app.elegido === null}
  <Vacio icono="aumentos" texto={T.aumentos.elige} />
{:else if error}
  <Vacio icono="aumentos" texto={T.aumentos.error} />
{:else if filas === null}
  <p class="suave cargando">{T.aumentos.cargando}</p>
{:else if !grupos.length}
  <Vacio icono="aumentos" texto={T.aumentos.vacio} />
{:else}
  {#each grupos as g (g.tier)}
    {@const [palabra, color, letra] = calidad(T, g.tier)}
    <section class="grupo">
      <h3 class="sec" style="color:{color}"><Tier texto={letra} {color} tam={28} />{palabra} <small class="suave">{g.filas.length}</small></h3>
      <div class="filas">
        {#each g.filas as f (f.id)}
          <div class="fila panel" style="--c:{color}">
            <img src={f.icono} alt="" />
            <div class="nombre">
              <b>{f.nombre}</b>
              <small class={f.rareza}>{T.rarezas[f.rareza] ?? ''}</small>
            </div>
            <div class="medida" title={T.aumentos.rendimiento}>
              <div class="barra"><i style="width:{Math.max(4, Math.min(100, f.perf))}%"></i></div>
              <small class="suave">{f.perf.toFixed(0)}</small>
            </div>
            <div class="medida chica" title={T.aumentos.popularidad}>
              <div class="barra pop"><i style="width:{(100 * f.popular) / maxPopular}%"></i></div>
              <small class="suave">{f.popular.toFixed(1)}%</small>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/each}
{/if}

<style>
  .herramientas {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 18px;
  }
  .cargando {
    animation: latido 1.2s ease-in-out infinite;
  }
  @keyframes latido {
    50% {
      opacity: 0.4;
    }
  }
  .grupo {
    margin-bottom: 22px;
  }
  .grupo .sec {
    font-size: 13px;
  }
  .filas {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(460px, 1fr));
    gap: 8px;
  }
  .fila {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px 8px 8px;
    border-left: 3px solid var(--c);
  }
  .fila img {
    width: 38px;
    height: 38px;
  }
  .nombre {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .nombre b {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nombre small {
    font-size: 11px;
    letter-spacing: 1px;
    text-transform: uppercase;
  }
  .kPrismatic {
    color: #e879f9;
  }
  .kGold {
    color: #f5c542;
  }
  .kSilver {
    color: #c9c9ce;
  }
  .medida {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 120px;
  }
  .medida.chica {
    width: 100px;
  }
  .medida small {
    width: 38px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .barra {
    flex: 1;
    height: 4px;
    background: rgba(255, 255, 255, 0.07);
  }
  .barra i {
    display: block;
    height: 100%;
    background: var(--c);
  }
  .barra.pop i {
    background: var(--tier-3);
  }
</style>
