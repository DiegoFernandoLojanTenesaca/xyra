<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../app.svelte';
  import { idiomaEfectivo, textos } from '../i18n';
  import Encabezado from '../ui/Encabezado.svelte';
  import Icono from '../ui/Icono.svelte';
  import Opcion from '../ui/Opcion.svelte';

  const T = $derived(app.T);
  const config = $derived(app.config);
  const ESTILOS = ['placa', 'insignia', 'cinta', 'podio', 'enfoque'] as const;

  function probarVoz() {
    const idioma = idiomaEfectivo(config.idioma, app.estado.idioma_cliente);
    const u = new SpeechSynthesisUtterance(textos(idioma).voz[2]);
    u.lang = idioma.toLowerCase().startsWith('es') ? 'es-MX' : 'en-US';
    speechSynthesis.cancel();
    speechSynthesis.speak(u);
  }
</script>

<Encabezado titulo={T.nav.etiquetas} sub={T.etiquetas.subtitulo}>
  <button class="btn r" disabled={app.estado.fase === 'partida'} onclick={() => invoke('probar_capa')}><Icono nombre="probar" tam={14} />{T.inicio.probar}</button>
</Encabezado>

<div class="galeria">
  {#each ESTILOS as id}
    <button class="estilo panel corte" class:on={config.estilo === id} onclick={() => app.guardar({ estilo: id })}>
      <img src="/estilos/{id}.jpg" alt={T.estilos[id][0]} />
      <div class="texto">
        <b class="condensado">{T.estilos[id][0]}</b>
        {#if config.estilo === id}<span class="activo">{T.etiquetas.activo}</span>{/if}
      </div>
      <small class="suave">{T.estilos[id][1]}</small>
    </button>
  {/each}
</div>

<div class="dos">
  <section class="panel corte caja">
    <h3 class="sec">{T.etiquetas.calibracion}</h3>
    <p class="suave">{T.etiquetas.calibracion_desc}</p>
    <label class="rango">
      <span>{T.etiquetas.offset}</span>
      <input type="range" min="-80" max="80" step="2" value={config.offset_y} onchange={(e) => app.guardar({ offset_y: +e.currentTarget.value })} />
      <b>{config.offset_y > 0 ? '+' : ''}{config.offset_y}</b>
    </label>
    <label class="rango">
      <span>{T.etiquetas.escala}</span>
      <input type="range" min="0.7" max="1.4" step="0.05" value={config.escala} onchange={(e) => app.guardar({ escala: +e.currentTarget.value })} />
      <b>{Math.round(config.escala * 100)}%</b>
    </label>
  </section>

  <section class="panel corte caja">
    <h3 class="sec">{T.etiquetas.voz}</h3>
    <Opcion titulo={T.etiquetas.voz} desc={T.etiquetas.voz_desc} activo={config.voz} cambiar={() => app.guardar({ voz: !config.voz })} />
    <button class="btn" onclick={probarVoz}><Icono nombre="voz" tam={14} />{T.etiquetas.probar_voz}</button>
  </section>
</div>

<style>
  .galeria {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 14px;
    margin-bottom: 22px;
  }
  .estilo {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 10px 14px;
    text-align: left;
    transition: border-color 0.15s, background 0.15s;
  }
  .estilo:hover {
    border-color: #34343a;
  }
  .estilo.on {
    border-color: var(--rojo);
    background: linear-gradient(180deg, rgba(229, 19, 43, 0.16), transparent 60%), var(--panel);
  }
  .estilo img {
    width: 100%;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    object-position: center 60%;
  }
  .texto {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 4px;
  }
  .texto b {
    font-size: 20px;
  }
  .estilo small {
    padding: 0 4px;
  }
  .activo {
    padding: 2px 8px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 2px;
    text-transform: uppercase;
    background: var(--rojo);
  }
  .dos {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 22px;
  }
  .caja {
    padding: 16px 18px;
  }
  .caja p {
    margin: 0 0 14px;
  }
  .rango {
    display: grid;
    grid-template-columns: 70px 1fr 48px;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
  }
  .rango b {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .caja .btn {
    margin-top: 8px;
  }
</style>
