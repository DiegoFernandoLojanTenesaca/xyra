<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../app.svelte';
  import type { Build, Icono as Cosa } from '../tipos';
  import CabeceraCampeon from '../ui/CabeceraCampeon.svelte';
  import Encabezado from '../ui/Encabezado.svelte';
  import Icono from '../ui/Icono.svelte';
  import Segmentos from '../ui/Segmentos.svelte';
  import SelectorCampeon from '../ui/SelectorCampeon.svelte';
  import Vacio from '../ui/Vacio.svelte';

  const T = $derived(app.T);
  let build = $state<Build | null>(null);
  let error = $state(false);
  let aviso = $state('');
  let ocupado = $state(false);

  $effect(app.campeonPorDefecto);

  // la build se pide de nuevo al cambiar campeón, modo o posición
  $effect(() => {
    const [id, modo, posicion] = [app.elegido, app.modoBuild, app.modoBuild === 'grieta' ? app.posicion : null];
    if (id === null) return;
    build = null;
    error = false;
    aviso = '';
    invoke<Build>('get_build', { campeon: id, modo, posicion })
      .then((b) => {
        if (id === app.elegido && modo === app.modoBuild) build = b;
      })
      .catch(() => (error = true));
  });

  const POSICIONES = ['top', 'jungle', 'mid', 'adc', 'support'];


  const actual = $derived(app.campeones.find((c) => c.id === app.elegido) ?? null);
  const sinNombres = $derived(!!build && !build.runas.principal.icono);
  const bloques = $derived(build ? [build.inicio, build.botas, build.nucleo, build.situacionales] : []);

  async function importar(que: 'runas' | 'items') {
    if (!build) return;
    ocupado = true;
    aviso = await app.importarBuild(build.campeon, que);
    ocupado = false;
  }
</script>

{#snippet cosa(c: Cosa, tam = 40)}
  {#if c.icono}
    <img class="cosa" src={c.icono} alt={c.nombre} title={c.nombre} style="--t:{tam}px" />
  {:else}
    <span class="cosa nombre" title={c.nombre} style="--t:{tam}px">{c.nombre}</span>
  {/if}
{/snippet}

<Encabezado titulo={T.nav.build} sub={T.build.subtitulo}>
  <SelectorCampeon placeholder={T.aumentos.buscar} />
</Encabezado>

<div class="herramientas">
  <Segmentos opciones={[['aram', T.build.modos.aram], ['grieta', T.build.modos.grieta]]} bind:valor={app.modoBuild} />
  {#if app.modoBuild === 'grieta'}
    <Segmentos opciones={POSICIONES.map((p) => [p, T.build.posiciones[p]] as [string, string])} bind:valor={() => app.posicion ?? build?.posicion ?? 'mid', (p) => (app.posicion = p)} />
  {/if}
</div>

{#if actual}
  {@const lugar = app.modoBuild === 'grieta' ? `${T.build.modos.grieta} · ${T.build.posiciones[build?.posicion ?? ''] ?? ''}` : T.build.modos.aram}
  <CabeceraCampeon campeon={actual} tier={app.modoBuild === 'aram'} detalle={build ? `${lugar} · ${build.winrate.toFixed(1)}% WR · ${build.partidas.toLocaleString()} ${T.build.partidas}` : lugar}>
    <button class="btn" disabled={!build || ocupado} onclick={() => importar('items')}><Icono nombre="descargar" tam={14} />{T.build.importar_items}</button>
    <button class="btn r" disabled={!build || ocupado} onclick={() => importar('runas')}><Icono nombre="descargar" tam={14} />{T.build.importar_runas}</button>
  </CabeceraCampeon>
{/if}

{#if aviso}<p class="aviso">{aviso}</p>{/if}

{#if error}
  <Vacio icono="build" texto={T.build.error} />
{:else if !build}
  <p class="suave cargando">{T.build.cargando}</p>
{:else}
  {#if sinNombres}<p class="suave">{T.build.sin_catalogo}</p>{/if}
  <div class="grid">
    <section class="panel corte caja">
      <h3 class="sec">{T.build.runas} <small>{build.runas.winrate.toFixed(1)}% WR · {build.runas.uso.toFixed(0)}% {T.build.uso}</small></h3>
      <div class="arbol">
        <div class="estilo">{@render cosa(build.runas.principal, 30)}<b>{build.runas.principal.nombre}</b></div>
        <div class="fila">
          {#each build.runas.runas as r, i (i)}
            {@render cosa(r, i === 0 ? 64 : 44)}
          {/each}
        </div>
      </div>
      <div class="arbol">
        <div class="estilo">{@render cosa(build.runas.secundaria, 26)}<b>{build.runas.secundaria.nombre}</b></div>
        <div class="fila">
          {#each build.runas.secundarias as r, i (i)}
            {@render cosa(r, 40)}
          {/each}
        </div>
      </div>
      <div class="arbol">
        <div class="estilo"><b class="suave">{T.build.fragmentos}</b></div>
        <div class="fila">
          {#each build.runas.fragmentos as r, i (i)}
            {@render cosa(r, 28)}
          {/each}
        </div>
      </div>
    </section>

    <div class="col">
      <section class="panel corte caja">
        <h3 class="sec">{T.build.hechizos}</h3>
        <div class="fila">
          {#each build.hechizos as h (h.id)}
            {@render cosa(h, 44)}
          {/each}
        </div>
      </section>
      <section class="panel corte caja">
        <h3 class="sec">{T.build.items}</h3>
        {#each bloques as lista, i (i)}
          {#if lista.length}
            <div class="bloque">
              <small class="suave">{T.build.bloques[i]}</small>
              <div class="fila">
                {#each lista as it, j (j)}
                  {#if i === 2 && j > 0}<span class="flecha">›</span>{/if}
                  {@render cosa(it, i === 2 ? 48 : 40)}
                {/each}
              </div>
            </div>
          {/if}
        {/each}
      </section>
    </div>
  </div>

  {#if build.habilidades.length}
    <section class="panel corte caja">
      <h3 class="sec">{T.build.habilidades} <small>{T.build.prioridad}: {build.prioridad.join(' › ')}</small></h3>
      <div class="habilidades" style="--n:{build.habilidades.length}">
        {#each ['Q', 'W', 'E', 'R'] as tecla}
          <b class="tecla">{tecla}</b>
          {#each build.habilidades as h, nivel (nivel)}
            <span class:on={h === tecla}>{h === tecla ? nivel + 1 : ''}</span>
          {/each}
        {/each}
      </div>
    </section>
  {/if}

  <p class="suave nota">{T.build.nota}</p>
{/if}

<style>
  .herramientas {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin: -6px 0 18px;
  }
  .aviso {
    margin: -8px 0 18px;
    color: var(--ok);
    font-weight: 600;
  }
  .cargando {
    animation: latido 1.2s ease-in-out infinite;
  }
  @keyframes latido {
    50% {
      opacity: 0.4;
    }
  }
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 22px;
    margin-bottom: 22px;
    align-items: start;
  }
  .col {
    display: grid;
    gap: 22px;
  }
  .caja {
    padding: 16px 18px;
  }
  .arbol + .arbol {
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--linea);
  }
  .estilo {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
  }
  .estilo b {
    font-size: 13px;
    letter-spacing: 1.5px;
    text-transform: uppercase;
  }
  .fila {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }
  .cosa {
    width: var(--t);
    height: var(--t);
    flex: none;
  }
  .cosa.nombre {
    display: grid;
    place-items: center;
    padding: 2px;
    overflow: hidden;
    font-size: 9px;
    color: var(--suave);
    background: var(--panel-2);
    border: 1px solid var(--linea);
  }
  .bloque + .bloque {
    margin-top: 14px;
  }
  .bloque small {
    display: block;
    margin-bottom: 8px;
    font-size: 11px;
    letter-spacing: 2px;
    text-transform: uppercase;
    font-weight: 700;
  }
  .flecha {
    color: var(--rojo-2);
    font-size: 22px;
    font-weight: 700;
  }
  .habilidades {
    display: grid;
    grid-template-columns: 30px repeat(var(--n), minmax(0, 1fr));
    gap: 4px;
  }
  .tecla {
    display: grid;
    place-items: center;
    color: var(--rojo-2);
  }
  .habilidades span {
    display: grid;
    place-items: center;
    height: 28px;
    font-size: 12px;
    font-weight: 700;
    background: var(--panel-2);
  }
  .habilidades span.on {
    background: var(--rojo);
    color: #fff;
  }
  .nota {
    font-size: 12px;
  }
</style>
