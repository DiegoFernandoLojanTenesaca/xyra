<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { app, pct } from '../app.svelte';
  import type { Config, PestanaAjustes } from '../tipos';
  import Encabezado from '../ui/Encabezado.svelte';
  import Icono from '../ui/Icono.svelte';
  import Kpi from '../ui/Kpi.svelte';
  import Opcion from '../ui/Opcion.svelte';
  import Segmentos from '../ui/Segmentos.svelte';
  import Vacio from '../ui/Vacio.svelte';
  import Acerca from './Acerca.svelte';
  import Ayuda from './Ayuda.svelte';

  const T = $derived(app.T);
  const config = $derived(app.config);
  const stats = $derived(app.stats);
  const perfil = $derived(app.perfil);
  const PESTANAS: PestanaAjustes[] = ['general', 'perfil', 'datos', 'seguridad', 'ayuda', 'acerca'];

  let peso = $state<number | null>(null);
  let confirmar = $state(false);
  let aviso = $state('');

  $effect(() => {
    if (app.pestana === 'datos') invoke<number>('peso_datos').then((b) => (peso = b));
  });

  const tamano = (b: number) => (b < 1024 * 1024 ? `${Math.max(1, Math.round(b / 1024))} KB` : `${(b / 1024 / 1024).toFixed(1)} MB`);

  async function exportar() {
    aviso = `${T.ajustes.exportado} ${await invoke<string>('exportar_csv')}`;
  }

  async function borrar() {
    if (!confirmar) {
      confirmar = true;
      setTimeout(() => (confirmar = false), 4000);
      return;
    }
    confirmar = false;
    await invoke('borrar_datos');
    app.perfil = null;
    await app.recargar();
    peso = await invoke<number>('peso_datos');
    aviso = T.ajustes.borrado;
  }
</script>

<Encabezado eyebrow={T.nav.ajustes} titulo={T.ajustes.pestanas[app.pestana]} sub={T.ajustes.subtitulos[app.pestana]} />
<Segmentos pestanas opciones={PESTANAS.map((p) => [p, T.ajustes.pestanas[p]])} bind:valor={app.pestana} />

{#if app.pestana === 'general'}
  <div class="dos">
    <section class="panel corte caja">
      <h3 class="sec">{T.ajustes.pestanas.general}</h3>
      <label class="fila">
        <b>{T.ajustes.idioma}</b>
        <select onchange={(e) => app.guardar({ idioma: e.currentTarget.value as Config['idioma'] })}>
          <option value="auto" selected={config.idioma === 'auto'}>{T.ajustes.auto}</option>
          <option value="es" selected={config.idioma === 'es'}>Español</option>
          <option value="en" selected={config.idioma === 'en'}>English</option>
        </select>
      </label>
      <Opcion titulo={T.ajustes.autostart} desc={T.ajustes.autostart_desc} activo={config.autostart} cambiar={() => app.guardar({ autostart: !config.autostart })} />
      <Opcion titulo={T.ajustes.arena} desc={T.ajustes.arena_desc} activo={config.arena} cambiar={() => app.guardar({ arena: !config.arena })} />
    </section>
    <section class="panel corte caja">
      <h3 class="sec">{T.ajustes.diagnostico}</h3>
      <Opcion titulo={T.ajustes.grabar} desc={T.ajustes.grabar_desc} activo={config.grabar} cambiar={() => app.guardar({ grabar: !config.grabar })} />
      <div class="botones">
        <button class="btn" onclick={() => invoke('abrir_carpeta', { que: 'capturas' })}><Icono nombre="carpeta" tam={14} />{T.ajustes.abrir_capturas}</button>
        <button class="btn" onclick={() => invoke('abrir_carpeta', { que: 'registro' })}><Icono nombre="carpeta" tam={14} />{T.ajustes.abrir_logs}</button>
      </div>
    </section>
  </div>
{:else if app.pestana === 'perfil'}
  {#if perfil}
    <section class="tarjeta destacado corte">
      <div class="avatar">
        <img class="rombo" src={perfil.icono} alt="" />
        <span class="nv">{perfil.nivel}</span>
      </div>
      <div>
        <h2 class="condensado">{perfil.nombre} <small>#{perfil.tag}</small></h2>
        <div class="chips">
          {#if perfil.region}<span class="chip r">{perfil.region}</span>{/if}
          {#if perfil.rango}
            <span class="chip rango">
              <img src="https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests/{perfil.rango.liga.toLowerCase()}.svg" alt="" />
              {T.ligas[perfil.rango.liga] ?? perfil.rango.liga} {perfil.rango.division} · {perfil.rango.lp} LP
            </span>
          {:else}
            <span class="chip">{T.ajustes.sin_rango}</span>
          {/if}
          <span class="chip">{app.estado.idioma_cliente.replace('_', '-')}</span>
        </div>
      </div>
    </section>
    <div class="kpis">
      <Kpi etiqueta={T.stats.partidas} valor={stats?.partidas ?? 0} />
      <Kpi etiqueta={T.stats.winrate} valor={stats?.partidas ? `${pct(stats.victorias, stats.partidas)}%` : '—'} rojo />
      <Kpi etiqueta={T.ajustes.campeones_jugados} valor={stats?.campeones.length ?? 0} />
    </div>
    {#if perfil.maestrias.length}
      <h3 class="sec">{T.ajustes.maestria}</h3>
      <div class="panel corte maestrias">
        {#each perfil.maestrias as m (m.id)}
          <button onclick={() => app.verAumentos(m.id)}>
            <img class="corte" src={m.icono} alt="" />
            <b>{m.nombre}</b>
            <small class="suave">{Math.round(m.puntos / 1000)} K · M{m.nivel}</small>
          </button>
        {/each}
      </div>
    {/if}
  {:else}
    <div class="panel corte"><Vacio icono="inicio" texto={T.ajustes.sin_perfil} /></div>
  {/if}
{:else if app.pestana === 'datos'}
  <div class="dos">
    <section class="panel corte caja">
      <h3 class="sec">{T.ajustes.pestanas.datos}</h3>
      <div class="dato"><span class="suave">{T.ajustes.partidas_guardadas}</span><b>{stats?.partidas ?? 0}</b></div>
      <div class="dato"><span class="suave">{T.ajustes.espacio}</span><b>{peso === null ? '…' : tamano(peso)}</b></div>
      <div class="dato"><span class="suave">{T.ajustes.carpeta}</span><b>%APPDATA%\com.indagalab.xyra</b></div>
      <div class="botones columna">
        <button class="btn" onclick={exportar}><Icono nombre="descargar" tam={14} />{T.ajustes.exportar}</button>
        <button class="btn" onclick={() => invoke('abrir_carpeta', { que: 'datos' })}><Icono nombre="carpeta" tam={14} />{T.ajustes.abrir_carpeta}</button>
        <button class="btn peligro" class:r={confirmar} onclick={borrar}><Icono nombre="borrar" tam={14} />{confirmar ? T.ajustes.confirmar : T.ajustes.borrar}</button>
      </div>
      {#if aviso}<p class="suave aviso">{aviso}</p>{/if}
    </section>
    <section class="panel corte caja">
      <h3 class="sec">{T.ajustes.pestanas.seguridad}</h3>
      <p class="priv"><b>■ {T.ajustes.sin_telemetria}</b> {T.ajustes.conexiones}</p>
    </section>
  </div>
{:else if app.pestana === 'seguridad'}
  <section class="panel corte caja">
    <ul class="seguridad">
      {#each T.ajustes.seguridad_items as item}
        <li><span class="check">✓</span>{item}</li>
      {/each}
    </ul>
    <p class="suave nota">{T.ajustes.seguridad_nota}</p>
  </section>
{:else if app.pestana === 'ayuda'}
  <Ayuda />
{:else}
  <Acerca />
{/if}

<style>
  .dos {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 22px;
    align-items: start;
  }
  .caja {
    padding: 16px 18px;
  }
  .fila {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 4px 0 12px;
    border-bottom: 1px solid var(--linea);
  }
  .fila b {
    font-weight: 600;
  }
  .botones {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
  .botones.columna {
    display: grid;
    margin-top: 16px;
  }
  .dato {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 0;
    font-size: 13px;
  }
  .dato + .dato {
    border-top: 1px solid var(--linea);
  }
  .aviso {
    margin: 12px 0 0;
    font-size: 12px;
    word-break: break-all;
  }
  .priv {
    margin: 0;
    line-height: 1.6;
    color: var(--suave);
  }
  .priv b {
    color: var(--ok);
  }
  .tarjeta {
    display: flex;
    align-items: center;
    gap: 26px;
    padding: 26px;
    margin-bottom: 18px;
  }
  .tarjeta h2 {
    margin: 0;
    font-size: 36px;
    font-variation-settings: 'wdth' 78;
    text-transform: none;
  }
  .tarjeta h2 small {
    font-size: 20px;
    color: var(--suave);
    font-weight: 400;
  }
  .avatar {
    position: relative;
    flex: none;
  }
  .avatar img {
    width: 96px;
    height: 96px;
    filter: drop-shadow(0 0 12px rgba(229, 19, 43, 0.6));
  }
  .nv {
    position: absolute;
    bottom: -8px;
    left: 50%;
    transform: translateX(-50%);
    padding: 2px 10px;
    font-size: 12px;
    font-weight: 800;
    background: #0a0a0a;
    border: 1px solid var(--rojo-2);
  }
  .chips {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .chip {
    padding: 4px 10px;
    font-size: 11px;
    letter-spacing: 1.5px;
    font-weight: 700;
    text-transform: uppercase;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--linea);
  }
  .chip.rango {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding-left: 6px;
  }
  .chip.rango img {
    width: 18px;
    height: 18px;
  }
  .chip.r {
    border-color: rgba(229, 19, 43, 0.5);
    color: var(--rosa);
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
    margin-bottom: 22px;
  }
  .maestrias {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    padding: 16px;
  }
  .maestrias button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px;
    border: none;
    background: none;
  }
  .maestrias button:hover {
    background: rgba(229, 19, 43, 0.08);
  }
  .maestrias img {
    width: 56px;
    height: 56px;
    margin-bottom: 4px;
  }
  .maestrias small {
    font-size: 11px;
  }
  .seguridad {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
  }
  .seguridad li {
    display: flex;
    gap: 12px;
  }
  .check {
    color: var(--ok);
    font-weight: 800;
  }
  .nota {
    margin: 18px 0 0;
    padding-top: 14px;
    border-top: 1px solid var(--linea);
  }
</style>
