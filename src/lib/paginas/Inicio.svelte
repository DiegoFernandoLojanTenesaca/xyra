<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { app, pct } from '../app.svelte';
  import { calidad, tierCampeon } from '../i18n';
  import type { CampeonInfo } from '../tipos';
  import Icono from '../ui/Icono.svelte';
  import Kpi from '../ui/Kpi.svelte';
  import Logo from '../ui/Logo.svelte';
  import Tier from '../ui/Tier.svelte';
  import Vacio from '../ui/Vacio.svelte';

  const T = $derived(app.T);
  const estado = $derived(app.estado);
  const config = $derived(app.config);
  const stats = $derived(app.stats);

  let aviso = $state('');
  const cartas = $derived([...estado.cartas].sort((a, b) => a.x - b.x));
  const mejor = $derived(cartas.find((c) => c.mejor) ?? null);
  const mio = $derived(estado.seleccion?.campeon ?? null);
  /** El mejor de la banca, solo si rinde más que el tuyo. */
  const mejorBanca = $derived.by(() => {
    const banca = (estado.seleccion?.banca ?? []).filter((c) => c.puesto !== null);
    const m = banca.sort((a, b) => a.puesto! - b.puesto!)[0];
    return m && (!mio?.puesto || m.puesto! < mio.puesto) ? m : null;
  });
  const enPartida = $derived(app.campeones.find((c) => c.nombre === estado.campeon) ?? null);
  /** Los 5 mejores de ARAM: Caos según OP.GG (la lista de campeones ya viene ordenada por puesto). */
  const top = $derived(app.campeones.filter((c) => c.puesto !== null).slice(0, 5));
  const modo = $derived(T.modos[estado.modo ?? ''] ?? '');

  /** Retrato, antetítulo, titular (con la parte destacada) y bajada del bloque principal, según la fase. */
  const hero = $derived.by(() => {
    const f = estado.fase;
    if (f === 'partida') {
      const base = { icono: enPartida?.icono, eyebrow: `● ${T.inicio.en_vivo} · ${modo}` };
      return mejor
        ? { ...base, antes: T.inicio.elige, fuerte: mejor.nombre, despues: '', texto: T.inicio.mejor_de(cartas.length, estado.campeon ?? '') }
        : { ...base, antes: estado.campeon ?? '', fuerte: '', despues: '', texto: T.inicio.partida };
    }
    if (f === 'seleccion') {
      const base = { icono: (mejorBanca ?? mio)?.icono, eyebrow: `● ${T.fase.seleccion}` };
      return mejorBanca
        ? { ...base, antes: T.inicio.toma, fuerte: mejorBanca.nombre, despues: T.inicio.de_la_banca, texto: T.inicio.mejor_que(mio?.nombre ?? '', mejorBanca.puesto!) }
        : { ...base, antes: '', fuerte: mio?.nombre ?? T.fase.seleccion, despues: '', texto: T.inicio.tu_campeon(mio?.puesto ?? null) };
    }
    const [titulo, texto] = T.inicio[f];
    return { icono: undefined, eyebrow: T.fase[f], antes: titulo, fuerte: '', despues: '', texto };
  });

  async function sinBordes() {
    try {
      await invoke('poner_sin_bordes');
      aviso = T.inicio.listo;
    } catch (e) {
      aviso = e === 'partida' ? T.inicio.cerrar_partida : String(e);
    }
  }
</script>

{#snippet retrato(c: CampeonInfo, destacado: boolean)}
  {@const [etiqueta, color] = tierCampeon(c.tier)}
  <button class="retrato" class:mejor={destacado} onclick={() => app.verAumentos(c.id)} title="{c.nombre} #{c.puesto ?? '—'}">
    <img src={c.icono} alt={c.nombre} />
    <span><Tier texto={etiqueta} {color} tam={22} /></span>
  </button>
{/snippet}

<section class="hero destacado corte" class:vivo={estado.fase === 'partida' || estado.fase === 'seleccion'}>
  {#if hero.icono}<img class="corte" src={hero.icono} alt="" />{:else}<Logo tam={76} fase={estado.fase} />{/if}
  <div class="texto">
    <span class="eyebrow">{hero.eyebrow}</span>
    <h2 class="condensado">{hero.antes} {#if hero.fuerte}<em>{hero.fuerte}</em>{/if} {hero.despues}</h2>
    <p class="suave">{hero.texto}</p>
    {#if aviso && estado.fase === 'seleccion'}<p class="aviso-hero">{aviso}</p>{/if}
  </div>
  <div class="acciones">
    {#if estado.fase === 'seleccion' && mio}
      <!-- en la selección lo útil es la build del campeón que te tocó -->
      <button class="btn" onclick={() => app.verBuild(mio.id)}><Icono nombre="build" tam={14} />{T.inicio.ver_build}</button>
      <button class="btn r" onclick={async () => (aviso = await app.importarBuild(mio.id, 'runas'))}><Icono nombre="descargar" tam={14} />{T.build.importar_runas}</button>
    {:else}
      <button class="btn" disabled={estado.fase === 'partida'} onclick={() => invoke('probar_capa')}><Icono nombre="probar" tam={14} />{T.inicio.probar}</button>
      <button class="btn r" onclick={() => app.guardar({ pausado: !config.pausado })}>
        <Icono nombre={config.pausado ? 'probar' : 'pausa'} tam={14} />{config.pausado ? T.inicio.reanudar : T.inicio.pausar}
      </button>
    {/if}
  </div>
</section>

<div class="grid">
  <div class="col">
    <h3 class="sec">{estado.fase === 'partida' ? T.inicio.cartas : T.inicio.ultimas}</h3>
    {#if cartas.length}
      <div class="cartas">
        {#each cartas as c, i (c.id)}
          {@const [palabra, color, letra] = calidad(T, c.tier)}
          <div class="carta panel corte aparece" class:mejor={c.mejor} style="--i:{i}">
            {#if c.mejor}<div class="corona">★ {T.inicio.mejor_opcion}</div>{/if}
            {#if c.icono}<img src={c.icono} alt="" />{/if}
            <h4>{c.nombre || `#${c.id}`}</h4>
            <span class="calidad" style="color:{color}"><Tier texto={letra} {color} />{palabra}</span>
            <div class="barra"><i style="width:{c.tier === null ? 0 : Math.max(4, Math.min(100, c.perf))}%"></i></div>
            {#if c.cambiar}<small class="cambiar">↻ {T.cambiala}</small>{/if}
          </div>
        {/each}
      </div>
    {:else}
      <div class="panel corte"><Vacio icono="aumentos" texto={T.inicio.ninguna} /></div>
    {/if}

    <div class="par">
      {#if top.length}
        <section>
          <h3 class="sec">{T.inicio.top_aram}</h3>
          <div class="panel corte lista">
            {#each top as c, i (c.id)}
              {@const [etiqueta, color] = tierCampeon(c.tier)}
              <button class="fila aparece" style="--i:{i}" onclick={() => app.verBuild(c.id)}>
                <b class="puesto">#{c.puesto}</b>
                <img src={c.icono} alt="" />
                <span>{c.nombre}</span>
                <Tier texto={etiqueta} {color} tam={22} />
              </button>
            {/each}
          </div>
        </section>
      {/if}
      {#if stats?.recientes.length}
        <section>
          <h3 class="sec">{T.inicio.ultimas_partidas}</h3>
          <div class="panel corte lista">
            {#each stats.recientes.slice(0, 5) as p, i (p.game_id)}
              <button class="fila aparece partida" class:gano={p.victoria} style="--i:{i}" onclick={() => app.ir('stats')}>
                {#if p.icono}<img src={p.icono} alt="" />{/if}
                <span>{p.campeon}<small class="suave">{T.modos[p.modo] ?? p.modo}</small></span>
                <b>{p.victoria ? T.stats.victoria : T.stats.derrota}</b>
              </button>
            {/each}
          </div>
        </section>
      {/if}
    </div>

    {#if stats?.aumentos.length}
      <h3 class="sec espacio">{T.inicio.tus_aumentos} <small>({T.stats.min})</small></h3>
      <div class="panel corte lista">
        {#each stats.aumentos.slice(0, 4) as a, i (a.id)}
          <div class="fila aparece" style="--i:{i}">
            {#if a.icono}<img src={a.icono} alt="" />{/if}
            <span>{a.nombre}</span>
            <small class="suave">{a.partidas}</small>
            <b class="rojo">{pct(a.victorias, a.partidas)}%</b>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <aside class="col">
    <div class="kpis">
      <Kpi etiqueta={T.stats.partidas} valor={stats?.partidas ?? 0} />
      <Kpi etiqueta={T.stats.winrate} valor={stats?.partidas ? `${pct(stats.victorias, stats.partidas)}%` : '—'} rojo />
    </div>

    {#if estado.seleccion}
      <div class="panel corte caja">
        <h3 class="sec">{T.inicio.banca}</h3>
        <div class="banca">
          {#if mio}{@render retrato(mio, false)}{/if}
          {#each estado.seleccion.banca as c (c.id)}
            {@render retrato(c, c.id === mejorBanca?.id)}
          {/each}
        </div>
        {#if !estado.seleccion.banca.length}<p class="suave">{T.inicio.sin_banca}</p>{/if}
      </div>
    {/if}

    <div class="panel corte caja">
      <h3 class="sec">{T.inicio.ventana}</h3>
      {#if estado.sin_bordes === true}
        <p class="ok">■ {T.inicio.borderless_ok}</p>
      {:else if estado.sin_bordes === false}
        <p class="mal">⚠ {T.inicio.borderless_mal}</p>
        <button class="btn r" onclick={sinBordes}>{T.inicio.arreglar}</button>
      {:else}
        <p class="suave">{T.inicio.borderless_nose}</p>
      {/if}
      {#if aviso}<p class="suave aviso">{aviso}</p>{/if}
    </div>

    <div class="panel corte caja">
      <h3 class="sec">{T.inicio.sistema}</h3>
      <div class="dato"><span class="suave">{T.inicio.ocr}</span><b>{estado.ocr ?? '—'}</b></div>
      <div class="dato"><span class="suave">{T.nav.etiquetas}</span><b>{T.estilos[config.estilo as keyof typeof T.estilos]?.[0] ?? config.estilo}</b></div>
      <div class="dato"><span class="suave">{T.etiquetas.voz}</span><b>{config.voz ? 'ON' : 'OFF'}</b></div>
      {#if !estado.ocr}<p class="mal">⚠ {T.inicio.sin_ocr}</p>{/if}
    </div>
  </aside>
</div>

<style>
  .hero {
    display: flex;
    align-items: center;
    gap: 22px;
    padding: 22px 26px;
    margin-bottom: 22px;
  }
  /* sin partida ni selección el rojo baja de intensidad */
  .hero:not(.vivo) {
    background: linear-gradient(110deg, rgba(229, 19, 43, 0.06), transparent 55%), var(--panel);
    border-color: #22080b;
  }
  .hero > img {
    width: 76px;
    height: 76px;
    flex: none;
    object-fit: cover;
    box-shadow: 0 0 0 2px var(--rojo);
  }
  .texto {
    flex: 1;
    min-width: 0;
  }
  .hero h2 {
    margin: 4px 0 0;
    font-size: 30px;
    font-variation-settings: 'wdth' 78;
  }
  .hero em {
    font-style: normal;
    color: var(--rojo-2);
  }
  .hero p {
    margin: 6px 0 0;
  }
  .acciones {
    display: flex;
    gap: 10px;
    z-index: 1;
  }
  .aviso-hero {
    margin: 6px 0 0;
    color: var(--ok);
    font-weight: 600;
  }
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 22px;
    align-items: start;
  }
  .col {
    display: grid;
    gap: 14px;
    align-content: start;
  }
  .col > .sec {
    margin: 0;
  }
  .sec.espacio {
    margin-top: 8px;
  }
  .cartas {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }
  .carta {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 28px 14px 18px;
    text-align: center;
  }
  .carta.mejor {
    border-color: var(--rojo);
    background: linear-gradient(180deg, rgba(229, 19, 43, 0.18), transparent 65%), var(--panel);
    animation: aparecer 0.35s ease-out both, latido 2.4s ease-in-out 0.4s infinite;
    animation-delay: calc(var(--i, 0) * 60ms), 0.4s;
  }
  @keyframes latido {
    50% {
      background-color: #1d0a0c;
      border-color: var(--rojo-2);
    }
  }
  .corona {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    padding: 4px;
    background: var(--rojo);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 3px;
    text-transform: uppercase;
  }
  .carta img {
    width: 58px;
    height: 58px;
  }
  .carta h4 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .calidad {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 2px;
  }
  .barra {
    align-self: stretch;
    height: 4px;
    margin: 2px 8px 0;
    background: rgba(255, 255, 255, 0.07);
  }
  .barra i {
    display: block;
    height: 100%;
    background: var(--rojo);
  }
  .cambiar {
    color: var(--aviso);
    font-weight: 600;
  }
  .lista .fila {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 14px;
    font-size: 13px;
    border: none;
    background: none;
    text-align: left;
  }
  button.fila:hover {
    background: rgba(229, 19, 43, 0.08);
  }
  .par {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 14px;
    margin-top: 8px;
  }
  .par .sec {
    margin-bottom: 10px;
  }
  .puesto {
    width: 26px;
    color: var(--suave);
    font-size: 12px;
  }
  .partida span {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .partida small {
    font-size: 11px;
  }
  .partida b {
    font-size: 12px;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--suave);
  }
  .partida.gano b {
    color: var(--rojo-2);
  }
  .lista .fila + .fila {
    border-top: 1px solid var(--linea);
  }
  .fila img {
    width: 30px;
    height: 30px;
  }
  .fila span {
    flex: 1;
  }
  .kpis {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .caja {
    padding: 14px 16px;
    font-size: 13px;
  }
  .caja .sec {
    margin-bottom: 10px;
  }
  .caja p {
    margin: 0 0 10px;
  }
  .caja p:last-child {
    margin-bottom: 0;
  }
  .ok {
    color: var(--ok);
    font-weight: 700;
    letter-spacing: 1.5px;
    text-transform: uppercase;
    font-size: 12px;
  }
  .mal {
    color: var(--aviso);
  }
  .caja p.aviso {
    margin: 10px 0 0;
    font-size: 12px;
  }
  .dato {
    display: flex;
    justify-content: space-between;
    padding: 6px 0;
  }
  .dato + .dato {
    border-top: 1px solid var(--linea);
  }
  .banca {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .retrato {
    position: relative;
    padding: 0;
    border: none;
    background: none;
    opacity: 0.65;
    transition: opacity 0.15s;
  }
  .retrato:first-child,
  .retrato:hover,
  .retrato.mejor {
    opacity: 1;
  }
  .retrato img {
    width: 44px;
    height: 44px;
  }
  .retrato.mejor img {
    box-shadow: 0 0 0 2px var(--rojo-2), 0 0 14px var(--rojo);
  }
  .retrato span {
    position: absolute;
    right: -7px;
    bottom: -7px;
  }
</style>
