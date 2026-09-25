<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../app.svelte';
  import { abrir, CREADORES, ENLACES, perfilGithub } from '../proyecto';
  import Icono from '../ui/Icono.svelte';

  const T = $derived(app.T);
</script>

<div class="dos">
  <section class="panel corte caja">
    <h3 class="sec">{T.ayuda.pasos_titulo}</h3>
    <ol>
      {#each T.ayuda.pasos as paso, i (i)}
        <li><span class="rombo num">{i + 1}</span>{paso}</li>
      {/each}
    </ol>
  </section>

  <section class="panel corte caja">
    <h3 class="sec">{T.creditos.titulo}</h3>
    {#each CREADORES as c (c.usuario)}
      <button class="creador" onclick={() => abrir(perfilGithub(c.usuario))} title={perfilGithub(c.usuario)}>
        <img class="rombo" src={c.foto} alt={c.nombre} />
        <span>
          <b>{c.nombre}</b>
          {#if !c.nombre.startsWith('@')}<small class="suave">@{c.usuario}</small>{/if}
        </span>
        <span class="equipo"><small>{T.creditos.roles[c.rol]}</small><b class="rojo">{c.equipo}</b></span>
      </button>
    {/each}
    <p class="suave gracias">{T.creditos.gracias}</p>
  </section>
</div>

<h3 class="sec">{T.ayuda.faq_titulo}</h3>
<div class="faq">
  {#each T.ayuda.faq as [pregunta, respuesta], i (i)}
    <details class="panel">
      <summary>{pregunta}</summary>
      <p class="suave">{respuesta}</p>
    </details>
  {/each}
</div>

<section class="panel corte caja soporte">
  <div>
    <h3 class="sec">{T.ayuda.soporte_titulo}</h3>
    <p class="suave">{T.ayuda.soporte}</p>
  </div>
  <div class="botones">
    <button class="btn r" onclick={() => abrir(ENLACES.problema)}><Icono nombre="enlace" tam={14} />{T.ayuda.reportar}</button>
    <button class="btn" onclick={() => invoke('abrir_carpeta', { que: 'registro' })}><Icono nombre="carpeta" tam={14} />{T.ayuda.registro}</button>
  </div>
</section>

<style>
  .dos {
    display: grid;
    grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr);
    gap: 22px;
    margin-bottom: 22px;
    align-items: start;
  }
  .caja {
    padding: 16px 18px;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .num {
    display: grid;
    place-items: center;
    flex: none;
    width: 28px;
    height: 28px;
    background: var(--rojo);
    font-weight: 800;
    font-size: 13px;
  }
  .creador {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
    padding: 10px 6px;
    border: none;
    background: none;
    text-align: left;
  }
  .creador:hover {
    background: rgba(229, 19, 43, 0.08);
  }
  .creador + .creador {
    border-top: 1px solid var(--linea);
  }
  .creador img {
    width: 52px;
    height: 52px;
    flex: none;
  }
  .creador span {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .creador span:nth-of-type(1) {
    flex: 1;
  }
  .creador small {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .equipo {
    align-items: flex-end;
  }
  .equipo small {
    letter-spacing: 2px;
    text-transform: uppercase;
    color: var(--suave);
  }
  .gracias {
    margin: 12px 0 0;
    font-size: 12px;
  }
  .faq {
    display: grid;
    gap: 6px;
    margin-bottom: 22px;
  }
  details {
    padding: 0 16px;
  }
  details[open] {
    border-color: rgba(229, 19, 43, 0.45);
  }
  summary {
    padding: 12px 0;
    font-weight: 600;
    cursor: pointer;
    list-style: none;
  }
  summary::before {
    content: '+';
    display: inline-block;
    width: 18px;
    color: var(--rojo-2);
    font-weight: 800;
  }
  details[open] summary::before {
    content: '–';
  }
  details p {
    margin: 0 0 14px 18px;
    line-height: 1.55;
  }
  .soporte {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
  }
  .soporte p {
    margin: 0;
  }
  .botones {
    display: flex;
    gap: 10px;
    flex: none;
  }
</style>
