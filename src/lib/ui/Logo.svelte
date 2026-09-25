<script lang="ts">
  // Logo de Xyra (diseño de Jahir, variante "núcleo grande"): cuatro rombos rojos y un núcleo en el centro.
  // El núcleo cuenta el estado: gira un cuarto de vuelta cada tanto; en partida se vuelve rojo y gira seguido;
  // en pausa se queda quieto. La misma forma genera el ícono de Windows (app-icon.png).
  import type { Fase } from '../tipos';

  let { tam = 32, fase = 'cliente' }: { tam?: number; fase?: Fase } = $props();
  const id = $props.id();
</script>

<svg viewBox="0 0 256 256" width={tam} height={tam} class:vivo={fase === 'partida'} class:quieto={fase === 'pausado'} aria-hidden="true">
  <defs>
    <!-- degradados radiales desde el centro: los cuatro rombos quedan iguales (simetría) -->
    <radialGradient id="{id}r" cx="128" cy="128" r="108" gradientUnits="userSpaceOnUse"><stop offset="0.35" stop-color="#b80f26" /><stop offset="1" stop-color="#ff3448" /></radialGradient>
    <radialGradient id="{id}g" cx="128" cy="128" r="36" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#9a9aa0" /><stop offset="1" stop-color="#5a5a60" /></radialGradient>
    <!-- el hueco alrededor del núcleo es transparente: se ve bien sobre cualquier fondo -->
    <mask id="{id}m"><rect width="256" height="256" fill="#fff" /><polygon points="128,75 181,128 128,181 75,128" fill="#000" /></mask>
  </defs>
  <g class="petalos" mask="url(#{id}m)" fill="url(#{id}r)">
    <polygon points="128,20 176,68 128,116 80,68" />
    <polygon points="68,80 116,128 68,176 20,128" />
    <polygon points="188,80 236,128 188,176 140,128" />
    <polygon points="128,140 176,188 128,236 80,188" />
  </g>
  <polygon class="nucleo" points="128,92 164,128 128,164 92,128" fill={fase === 'partida' ? `url(#${id}r)` : `url(#${id}g)`} />
</svg>

<style>
  svg {
    display: block;
    flex: none;
    overflow: visible;
  }
  .nucleo {
    transform-box: view-box;
    transform-origin: 50% 50%;
    animation: tic 3.2s cubic-bezier(0.7, 0, 0.3, 1) infinite;
  }
  /* quieto casi todo el ciclo y un giro corto de 90°: se nota vivo sin distraer */
  @keyframes tic {
    0%,
    78% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(90deg);
    }
  }
  .vivo .nucleo {
    animation: giro 1.6s linear infinite;
  }
  .vivo .petalos {
    filter: drop-shadow(0 0 10px rgba(229, 19, 43, 0.65));
  }
  @keyframes giro {
    to {
      transform: rotate(360deg);
    }
  }
  .quieto .nucleo {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .nucleo {
      animation: none !important;
    }
  }
</style>
