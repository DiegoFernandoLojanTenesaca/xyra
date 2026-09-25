<script lang="ts">
  import type { Phase } from '../types';

  let { size = 32, phase = 'client' }: { size?: number; phase?: Phase } = $props();
  const id = $props.id();
</script>

<svg viewBox="0 0 256 256" width={size} height={size} class:live={phase === 'in_game'} class:still={phase === 'paused'} aria-hidden="true">
  <defs>
    <radialGradient id="{id}-red" cx="128" cy="128" r="108" gradientUnits="userSpaceOnUse">
      <stop offset="0.35" style="stop-color: var(--color-accentDeep)" />
      <stop offset="1" style="stop-color: var(--color-accentBright)" />
    </radialGradient>
    <radialGradient id="{id}-gray" cx="128" cy="128" r="36" gradientUnits="userSpaceOnUse">
      <stop offset="0" style="stop-color: var(--color-logoCore)" />
      <stop offset="1" style="stop-color: var(--color-logoCoreDeep)" />
    </radialGradient>
    <mask id="{id}-gap"><rect width="256" height="256" fill="#fff" /><polygon points="128,75 181,128 128,181 75,128" fill="#000" /></mask>
  </defs>
  <g class="petals" mask="url(#{id}-gap)" fill="url(#{id}-red)">
    <polygon points="128,20 176,68 128,116 80,68" />
    <polygon points="68,80 116,128 68,176 20,128" />
    <polygon points="188,80 236,128 188,176 140,128" />
    <polygon points="128,140 176,188 128,236 80,188" />
  </g>
  <polygon class="core" points="128,92 164,128 128,164 92,128" fill={phase === 'in_game' ? `url(#${id}-red)` : `url(#${id}-gray)`} />
</svg>

<style>
  svg {
    display: block;
    flex: none;
    overflow: visible;
  }
  .core {
    transform-box: view-box;
    transform-origin: 50% 50%;
    animation: tick 3.2s cubic-bezier(0.7, 0, 0.3, 1) infinite;
  }
  @keyframes tick {
    0%,
    78% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(90deg);
    }
  }
  .live .core {
    animation: spin 1.6s linear infinite;
  }
  .live .petals {
    filter: drop-shadow(0 0 10px color-mix(in srgb, var(--color-accent) 65%, transparent));
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .still .core {
    animation: none;
  }
</style>
