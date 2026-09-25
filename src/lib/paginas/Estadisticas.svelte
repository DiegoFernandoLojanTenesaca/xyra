<script lang="ts">
  import { app, pct } from '../app.svelte';
  import type { Fila } from '../tipos';
  import Encabezado from '../ui/Encabezado.svelte';
  import Kpi from '../ui/Kpi.svelte';
  import Vacio from '../ui/Vacio.svelte';

  const T = $derived(app.T);
  const stats = $derived(app.stats);
  const usados = $derived(new Set((stats?.recientes ?? []).flatMap((p) => p.aumentos.map((a) => a.id))).size);
</script>

{#snippet filas(lista: Fila[], abrir?: (id: number) => void)}
  <div class="panel corte lista">
    {#each lista as f (f.id)}
      {@const wr = pct(f.victorias, f.partidas)}
      <button class="fila" disabled={!abrir} onclick={() => abrir?.(f.id)}>
        {#if f.icono}<img src={f.icono} alt="" />{/if}
        <span class="nombre">{f.nombre}</span>
        <small class="suave">{f.partidas}</small>
        <div class="barra"><i class:alto={wr >= 55} style="width:{wr}%"></i></div>
        <b class:rojo={wr >= 55}>{wr}%</b>
      </button>
    {/each}
  </div>
{/snippet}

<Encabezado titulo={T.nav.stats} sub={T.stats.subtitulo} />

{#if stats && stats.partidas}
  <div class="kpis">
    <Kpi etiqueta={T.stats.partidas} valor={stats.partidas} />
    <Kpi etiqueta={T.stats.victorias} valor={stats.victorias} />
    <Kpi etiqueta={T.stats.winrate} valor="{pct(stats.victorias, stats.partidas)}%" rojo />
    <Kpi etiqueta={T.stats.aumentos_distintos} valor={usados} />
  </div>

  <div class="dos">
    <section>
      <h3 class="sec">{T.stats.campeones}</h3>
      {@render filas(stats.campeones.slice(0, 12), app.verAumentos)}
    </section>
    <section>
      <h3 class="sec">{T.stats.aumentos} <small>({T.stats.min})</small></h3>
      {@render filas(stats.aumentos)}
    </section>
  </div>

  <h3 class="sec">{T.stats.recientes}</h3>
  <div class="recientes">
    {#each stats.recientes as p (p.game_id)}
      <div class="partida panel" class:gano={p.victoria}>
        {#if p.icono}<img src={p.icono} alt="" />{/if}
        <div class="col">
          <b class="resultado condensado">{p.victoria ? T.stats.victoria : T.stats.derrota}</b>
          <small class="suave">{p.campeon} · {T.modos[p.modo] ?? p.modo} · {p.fecha}</small>
        </div>
        <div class="aumentos">
          {#each p.aumentos as a (a.id)}
            {#if a.icono}<img src={a.icono} alt={a.nombre} title={a.nombre} />{/if}
          {/each}
        </div>
      </div>
    {/each}
  </div>
{:else if stats}
  <div class="panel corte"><Vacio icono="stats" texto={T.stats.vacio} /></div>
{/if}

<style>
  .kpis {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 14px;
    margin-bottom: 22px;
  }
  .dos {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 22px;
    margin-bottom: 22px;
  }
  .lista {
    padding: 6px 0;
  }
  .fila {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 14px;
    border: none;
    background: none;
    text-align: left;
  }
  .fila:disabled {
    cursor: default;
  }
  .fila:hover:not(:disabled) {
    background: rgba(229, 19, 43, 0.08);
  }
  .fila img {
    width: 30px;
    height: 30px;
  }
  .nombre {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .barra {
    width: 90px;
    height: 4px;
    background: rgba(255, 255, 255, 0.07);
  }
  .barra i {
    display: block;
    height: 100%;
    background: var(--tier-3);
  }
  .barra i.alto {
    background: var(--rojo);
  }
  .fila b {
    width: 42px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .recientes {
    display: grid;
    gap: 6px;
  }
  .partida {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 14px;
    border-left: 3px solid var(--tenue);
  }
  .partida.gano {
    border-left-color: var(--rojo);
    background: linear-gradient(90deg, rgba(229, 19, 43, 0.1), transparent 40%), var(--panel);
  }
  .partida > img {
    width: 42px;
    height: 42px;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
  }
  .resultado {
    font-size: 18px;
    color: var(--suave);
  }
  .gano .resultado {
    color: var(--rojo-2);
  }
  .aumentos {
    display: flex;
    gap: 5px;
  }
  .aumentos img {
    width: 30px;
    height: 30px;
  }
</style>
