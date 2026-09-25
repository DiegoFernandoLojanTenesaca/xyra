<script lang="ts">
  let { label, color, size = 'var(--size-badge)' }: { label: string; color: string; size?: string } = $props();
</script>

<span class="diamond" style="--badge-color:{color}; --badge-size:{size}">{label}</span>

<style>
  span {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: var(--badge-size);
    height: var(--badge-size);
    background: var(--badge-color);
    color: var(--color-textOnLight);
    font-weight: 800;
    font-size: calc(var(--badge-size) * 0.46);
  }
</style>
