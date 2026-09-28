<script lang="ts">
  import type { LpGame } from '../types';

  /** Wide and short, so the line reads as a trend; the SVG stretches to its box. */
  const WIDTH = 100;
  const HEIGHT = 30;

  /** The LP of the last games, oldest on the left; `games` come newest first. Shared by the PC and the phone. */
  let { games }: { games: LpGame[] } = $props();

  const points = $derived.by(() => {
    const deltas = [...games].reverse().map((game) => game.delta);
    const totals = deltas.reduce<number[]>((sums, delta) => [...sums, (sums.at(-1) ?? 0) + delta], [0]);
    const [low, high] = [Math.min(...totals), Math.max(...totals)];
    const span = high - low || 1;
    return totals.map((total, i) => `${(i / Math.max(1, totals.length - 1)) * WIDTH},${HEIGHT - ((total - low) / span) * HEIGHT}`).join(' ');
  });
  const rising = $derived(games.reduce((sum, game) => sum + game.delta, 0) >= 0);
</script>

<svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" aria-hidden="true" class:rising>
  <polyline {points} />
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: var(--size-thumbLg);
    overflow: visible;
  }
  polyline {
    fill: none;
    stroke: var(--color-textMuted);
    stroke-width: 2;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }
  .rising polyline {
    stroke: var(--color-success);
  }
</style>
