<script lang="ts">
  import { app } from '../app.svelte';
  import { abrir, CREADORES, ENLACES, perfilGithub } from '../proyecto';
  import Icono from '../ui/Icono.svelte';
  import Logo from '../ui/Logo.svelte';

  const T = $derived(app.T);
</script>

<section class="tarjeta destacado corte">
  <Logo tam={92} fase={app.estado.fase} />
  <div class="texto">
    <h2 class="condensado">XYRA <small>v{app.estado.version}</small></h2>
    <p class="suave">{T.gratis} · {T.ajustes.licencia}</p>
  </div>
  <div class="firmas">
    <small class="suave">{T.ajustes.creado_por}</small>
    {#each CREADORES as c (c.usuario)}
      <button onclick={() => abrir(perfilGithub(c.usuario))} title="@{c.usuario}">
        <img class="rombo" src={c.foto} alt="" />
        <span><b>{c.nombre}</b><small class="rojo">{c.equipo}</small></span>
      </button>
    {/each}
  </div>
</section>

<div class="dos">
  <section class="panel corte caja">
    <h3 class="sec">{T.ajustes.novedades_titulo}</h3>
    <ul>
      {#each T.ajustes.novedades as n, i (i)}
        <li><span class="rojo">■</span>{n}</li>
      {/each}
    </ul>
    <p class="suave hecho">{T.ajustes.hecho_con}</p>
  </section>

  <section class="panel corte caja">
    <h3 class="sec">{T.ajustes.enlaces}</h3>
    <div class="enlaces">
      <button class="btn" onclick={() => abrir(ENLACES.repo)}><Icono nombre="enlace" tam={14} />{T.ayuda.codigo}</button>
      <button class="btn" onclick={() => abrir(ENLACES.novedades)}><Icono nombre="descargar" tam={14} />{T.ayuda.novedades}</button>
      <button class="btn" onclick={() => abrir(ENLACES.problema)}><Icono nombre="enlace" tam={14} />{T.ayuda.reportar}</button>
      <button class="btn" onclick={() => app.abrirAjustes('ayuda')}><Icono nombre="ayuda" tam={14} />{T.ajustes.pestanas.ayuda}</button>
    </div>
    <p class="suave fuentes">{T.ajustes.fuentes}</p>
  </section>
</div>

<p class="suave legal">{T.ajustes.riot}</p>

<style>
  .tarjeta {
    display: flex;
    align-items: center;
    gap: 26px;
    padding: 26px;
    margin-bottom: 22px;
  }
  .texto {
    flex: 1;
  }
  .firmas {
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 1;
  }
  .firmas > small {
    font-size: 11px;
    letter-spacing: 2px;
    text-transform: uppercase;
  }
  .firmas button {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px 4px 4px;
    border: none;
    background: rgba(0, 0, 0, 0.35);
    text-align: left;
  }
  .firmas button:hover {
    background: rgba(229, 19, 43, 0.14);
  }
  .firmas img {
    width: 36px;
    height: 36px;
  }
  .firmas span {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .firmas small {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 1px;
  }
  h2 {
    margin: 0;
    font-size: 36px;
    font-variation-settings: 'wdth' 78;
  }
  h2 small {
    font-size: 20px;
    color: var(--suave);
    font-weight: 400;
  }
  .tarjeta p {
    margin: 8px 0 0;
  }
  .dos {
    display: grid;
    grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr);
    gap: 22px;
    margin-bottom: 18px;
    align-items: start;
  }
  .caja {
    padding: 16px 18px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  li {
    display: flex;
    gap: 10px;
  }
  .hecho,
  .fuentes {
    margin: 16px 0 0;
    padding-top: 12px;
    border-top: 1px solid var(--linea);
    font-size: 12px;
  }
  .enlaces {
    display: grid;
    gap: 8px;
  }
  .enlaces .btn {
    justify-content: flex-start;
  }
  .legal {
    font-size: 12px;
    max-width: 720px;
  }
</style>
