<script lang="ts">
  // Retrato, nombre y tier del campeón elegido; `detalle` agrega datos al renglón y `children` van a la derecha.
  import type { Snippet } from 'svelte';
  import { tierCampeon } from '../i18n';
  import type { CampeonInfo } from '../tipos';
  import Tier from './Tier.svelte';

  // `tier`: el tier y puesto son de ARAM: Caos; en la Grieta no se muestran
  let { campeon, detalle = '', tier: conTier = true, children }: { campeon: CampeonInfo; detalle?: string; tier?: boolean; children?: Snippet } =
    $props();
  const tier = $derived(tierCampeon(campeon.tier));
</script>

<section class="panel corte">
  <img class="corte" src={campeon.icono} alt="" />
  <div class="texto">
    <h2 class="condensado">{campeon.nombre}</h2>
    <span class="suave">
      {#if conTier}<Tier texto={tier[0]} color={tier[1]} tam={22} /> #{campeon.puesto ?? '—'}{detalle ? ' · ' : ''}{/if}{detalle}
    </span>
  </div>
  {#if children}<div class="acciones">{@render children()}</div>{/if}
</section>

<style>
  section {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 22px;
    padding: 14px 18px;
  }
  img {
    width: 64px;
    height: 64px;
    box-shadow: 0 0 0 2px var(--rojo);
  }
  .texto {
    flex: 1;
    min-width: 0;
  }
  h2 {
    margin: 0 0 8px;
    font-size: 28px;
  }
  span {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .acciones {
    display: flex;
    gap: 10px;
  }
</style>
