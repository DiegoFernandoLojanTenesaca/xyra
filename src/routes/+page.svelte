<script lang="ts">
  import '$lib/tema.css';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { app } from '$lib/app.svelte';
  import type { Pagina } from '$lib/tipos';
  import { abrir, REPO } from '$lib/proyecto';
  import Icono from '$lib/ui/Icono.svelte';
  import Logo from '$lib/ui/Logo.svelte';
  import Inicio from '$lib/paginas/Inicio.svelte';
  import Aumentos from '$lib/paginas/Aumentos.svelte';
  import Build from '$lib/paginas/Build.svelte';
  import Juego from '$lib/paginas/Juego.svelte';
  import Campeones from '$lib/paginas/Campeones.svelte';
  import Estadisticas from '$lib/paginas/Estadisticas.svelte';
  import Etiquetas from '$lib/paginas/Etiquetas.svelte';
  import Ajustes from '$lib/paginas/Ajustes.svelte';

  const ventana = getCurrentWindow();
  const PAGINAS: Pagina[] = ['inicio', 'build', 'aumentos', 'campeones', 'stats', 'etiquetas', 'juego'];
  const T = $derived(app.T);

  onMount(app.iniciar);
</script>

<div class="app">
  <header class="barra" data-tauri-drag-region>
    <div class="logo condensado" data-tauri-drag-region><Logo tam={26} fase={app.listo ? app.estado.fase : 'sin_lol'} />XYRA</div>
    {#if app.listo}
      <span class="pill corte-s {app.estado.fase}" data-tauri-drag-region>
        <i></i>{T.fase[app.estado.fase]}{app.estado.fase === 'partida' && app.estado.campeon
          ? ` · ${app.estado.campeon} · ${T.modos[app.estado.modo ?? ''] ?? ''}`
          : ''}
      </span>
    {/if}
    <div class="der">
      {#if app.perfil}
        <button class="perfil" onclick={() => app.abrirAjustes('perfil')}>
          <img class="rombo" src={app.perfil.icono} alt="" />
          <span><b>{app.perfil.nombre}</b><small>{T.nivel} {app.perfil.nivel}{app.perfil.region ? ` · ${app.perfil.region}` : ''}</small></span>
        </button>
      {/if}
      <button class="tuerca" class:on={app.pagina === 'ajustes'} onclick={() => app.abrirAjustes('general')} aria-label={T.nav.ajustes} title={T.nav.ajustes}>
        <Icono nombre="tuerca" tam={19} />
      </button>
      <div class="controles">
        <button onclick={() => ventana.minimize()} aria-label={T.ventana.minimizar}><Icono nombre="minimizar" tam={16} /></button>
        <button onclick={() => ventana.toggleMaximize()} aria-label={T.ventana.maximizar}><Icono nombre="maximizar" tam={14} /></button>
        <button class="cerrar" onclick={() => ventana.close()} aria-label={T.ventana.cerrar}><Icono nombre="cerrar" tam={16} /></button>
      </div>
    </div>
  </header>

  <nav>
    {#each PAGINAS as p}
      <button class:on={app.pagina === p} onclick={() => app.ir(p)}>
        <Icono nombre={p} tam={18} />{T.nav[p]}
        {#if p === 'stats' && app.nuevas}<span class="nuevas">+{app.nuevas}</span>{/if}
      </button>
    {/each}
    <div class="pie">
      <button class="ayuda" class:on={app.pagina === 'ajustes' && app.pestana === 'ayuda'} onclick={() => app.abrirAjustes('ayuda')}>
        <Icono nombre="ayuda" tam={16} />{T.ajustes.pestanas.ayuda}
      </button>
      <!-- crédito discreto: los creadores están en Ayuda -->
      <button class="firma" onclick={() => abrir(REPO)} title={REPO}>{app.listo ? `v${app.estado.version} · ` : ''}Xynitra × IndagaLab ↗</button>
    </div>
  </nav>

  <main>
    {#if app.listo}
      {#key app.pagina}
        <div class="pagina">
          {#if app.pagina === 'inicio'}
            <Inicio />
          {:else if app.pagina === 'build'}
            <Build />
          {:else if app.pagina === 'aumentos'}
            <Aumentos />
          {:else if app.pagina === 'campeones'}
            <Campeones />
          {:else if app.pagina === 'stats'}
            <Estadisticas />
          {:else if app.pagina === 'etiquetas'}
            <Etiquetas />
          {:else if app.pagina === 'juego'}
            <Juego />
          {:else}
            <Ajustes />
          {/if}
        </div>
      {/key}
    {/if}
  </main>
</div>

<style>
  .app {
    display: grid;
    grid-template-columns: 228px minmax(0, 1fr);
    grid-template-rows: 46px minmax(0, 1fr);
    height: 100vh;
  }
  .barra {
    grid-column: 1 / 3;
    display: flex;
    align-items: center;
    gap: 16px;
    padding-left: 20px;
    background: var(--barra);
    border-bottom: 1px solid var(--linea);
  }
  .logo {
    width: 192px;
    display: flex;
    align-items: center;
    gap: 11px;
    font-size: 19px;
    letter-spacing: 5px;
    font-variation-settings: 'wdth' 78;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 13px;
    font-size: 12px;
    letter-spacing: 1.5px;
    font-weight: 600;
    text-transform: uppercase;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--linea);
    white-space: nowrap;
  }
  .pill i {
    width: 7px;
    height: 7px;
    background: var(--tenue);
  }
  .pill.cliente i {
    background: var(--ok);
    box-shadow: 0 0 10px var(--ok);
  }
  .pill.seleccion,
  .pill.partida {
    background: rgba(229, 19, 43, 0.12);
    border-color: rgba(229, 19, 43, 0.45);
  }
  .pill.seleccion i,
  .pill.partida i {
    background: var(--rojo-2);
    box-shadow: 0 0 10px var(--rojo-2);
  }
  .pill.pausado i {
    background: var(--aviso);
  }
  .der {
    margin-left: auto;
    display: flex;
    align-items: stretch;
    height: 100%;
  }
  .der button {
    border: none;
    background: none;
    color: var(--suave);
  }
  .der button:hover {
    background: rgba(255, 255, 255, 0.05);
    color: var(--texto);
  }
  .der .perfil {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 16px;
    border-left: 1px solid var(--linea);
    text-align: left;
  }
  .perfil img {
    width: 28px;
    height: 28px;
  }
  .perfil span {
    display: flex;
    flex-direction: column;
  }
  .perfil b {
    font-size: 13px;
    line-height: 1.1;
    color: var(--texto);
  }
  .perfil small {
    font-size: 11px;
    letter-spacing: 1px;
  }
  .der .tuerca {
    width: 48px;
    display: grid;
    place-items: center;
    border-left: 1px solid var(--linea);
  }
  .der .tuerca.on {
    color: var(--rojo-2);
    background: rgba(229, 19, 43, 0.12);
  }
  .controles {
    display: flex;
    border-left: 1px solid var(--linea);
  }
  .controles button {
    width: 46px;
    display: grid;
    place-items: center;
  }
  .controles .cerrar:hover {
    background: #c42b1c;
    color: #fff;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 22px 0 16px;
    background: var(--barra);
    border-right: 1px solid var(--linea);
  }
  nav button {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 22px;
    border: none;
    background: none;
    color: var(--suave);
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 2px;
    text-transform: uppercase;
    text-align: left;
  }
  nav button:hover {
    color: var(--texto);
  }
  nav button.on {
    color: var(--texto);
    background: linear-gradient(90deg, rgba(229, 19, 43, 0.22), transparent 80%);
  }
  nav button.on::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--rojo);
    box-shadow: 0 0 12px var(--rojo);
  }
  nav button.on :global(svg) {
    color: var(--rojo-2);
  }
  .nuevas {
    margin-left: auto;
    padding: 1px 6px;
    font-size: 10px;
    letter-spacing: 1px;
    background: var(--rojo);
    color: #fff;
  }
  .pie {
    margin-top: auto;
    padding: 12px 0 0;
    border-top: 1px solid var(--linea);
  }
  nav .pie button {
    width: 100%;
    padding: 9px 22px;
    font-size: 12px;
  }
  nav .pie .firma {
    padding-top: 4px;
    font-size: 11px;
    letter-spacing: 0.5px;
    text-transform: none;
    color: var(--tenue);
  }
  nav .pie .firma:hover {
    color: var(--rojo-2);
  }
  main {
    overflow-y: auto;
    overflow-x: hidden;
    padding: 26px 32px 34px;
  }
  .pagina {
    max-width: 1120px;
    margin: 0 auto;
    animation: entrar 160ms ease-out;
  }
  @keyframes entrar {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
