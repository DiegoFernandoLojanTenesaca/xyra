<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { app } from '../app.svelte';
  import type { AjusteJuego } from '../tipos';
  import Encabezado from '../ui/Encabezado.svelte';
  import Icono from '../ui/Icono.svelte';
  import Opcion from '../ui/Opcion.svelte';
  import Vacio from '../ui/Vacio.svelte';

  const T = $derived(app.T);
  const config = $derived(app.config);
  let ajustes = $state<AjusteJuego[] | null>(null);
  let sinCliente = $state(false);

  // se vuelven a leer al conectar el cliente (cambia la fase)
  $effect(() => {
    void app.estado.fase;
    invoke<AjusteJuego[]>('get_ajustes_juego')
      .then((a) => ((ajustes = a), (sinCliente = false)))
      .catch(() => (sinCliente = true));
  });

  async function cambiar(a: AjusteJuego, valor: boolean | number) {
    await invoke('set_ajuste_juego', { seccion: a.seccion, clave: a.clave, valor });
    a.valor = valor;
  }
  // WindowMode es un número (2 = sin bordes); lo demás, sí/no
  const activo = (a: AjusteJuego) => (a.clave === 'WindowMode' ? a.valor === 2 : a.valor === true);
</script>

<Encabezado titulo={T.nav.juego} sub={T.juego.subtitulo} />

<div class="dos">
  <section class="panel corte caja">
    <h3 class="sec">{T.juego.opciones}</h3>
    <p class="suave desc">{T.juego.opciones_desc}</p>
    {#if sinCliente}
      <Vacio icono="juego" texto={T.juego.sin_cliente} />
    {:else if ajustes}
      {#each ajustes as a (a.clave)}
        {@const [titulo, desc] = T.juego.ajustes[a.clave] ?? [a.clave, '']}
        {#if a.clave === 'MinimapScale'}
          <label class="rango">
            <span><b>{titulo}</b><small class="suave">{desc}</small></span>
            <input type="range" min="1" max="3" step="0.1" value={a.valor} onchange={(e) => cambiar(a, +e.currentTarget.value)} />
            <b class="valor">{Number(a.valor).toFixed(1)}</b>
          </label>
        {:else}
          <Opcion {titulo} {desc} activo={activo(a)} cambiar={() => cambiar(a, a.clave === 'WindowMode' ? (activo(a) ? 0 : 2) : !activo(a))} />
        {/if}
      {/each}
    {/if}
  </section>

  <section class="panel corte caja">
    <h3 class="sec">{T.juego.automatico}</h3>
    <p class="suave desc">{T.juego.automatico_desc}</p>
    <Opcion titulo={T.juego.auto_bordes[0]} desc={T.juego.auto_bordes[1]} activo={config.auto_bordes} cambiar={() => app.guardar({ auto_bordes: !config.auto_bordes })} />
    <Opcion titulo={T.juego.auto_runas[0]} desc={T.juego.auto_runas[1]} activo={config.auto_runas} cambiar={() => app.guardar({ auto_runas: !config.auto_runas })} />
    <Opcion
      titulo={T.juego.cerrar_en_partida[0]}
      desc={T.juego.cerrar_en_partida[1]}
      activo={config.cerrar_en_partida}
      cambiar={() => app.guardar({ cerrar_en_partida: !config.cerrar_en_partida })}
    />
    <p class="seguro"><Icono nombre="escudo" tam={15} />{T.juego.seguro}</p>
  </section>
</div>

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
  .desc {
    margin: -4px 0 6px;
    font-size: 13px;
  }
  .rango {
    display: grid;
    grid-template-columns: 1fr 140px 34px;
    align-items: center;
    gap: 14px;
    padding: 12px 0;
    border-top: 1px solid var(--linea);
  }
  .rango span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .rango b {
    font-weight: 600;
  }
  .rango small {
    font-size: 12px;
  }
  .valor {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .seguro {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 12px 0 0;
    padding-top: 12px;
    border-top: 1px solid var(--linea);
    color: var(--ok);
    font-size: 12px;
  }
</style>
