<script lang="ts">
  // Rombo con la letra o el número del tier.
  let { texto, color, tam = 26 }: { texto: string; color: string; tam?: number } = $props();
</script>

<span class="rombo" style="--c:{color}; --t:{tam}px">{texto}</span>

<style>
  span {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: var(--t);
    height: var(--t);
    background: var(--c);
    color: #070707;
    font-weight: 800;
    font-size: calc(var(--t) * 0.46);
  }
</style>
