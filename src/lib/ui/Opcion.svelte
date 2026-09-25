<script lang="ts">
  // Fila de ajuste con interruptor: título, descripción y el switch a la derecha.
  let { titulo, desc = '', activo, cambiar }: { titulo: string; desc?: string; activo: boolean; cambiar: () => void } = $props();
</script>

<label>
  <span><b>{titulo}</b>{#if desc}<small class="suave">{desc}</small>{/if}</span>
  <input type="checkbox" class="switch" checked={activo} onchange={cambiar} />
</label>

<style>
  label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 0;
    cursor: pointer;
  }
  label + :global(label) {
    border-top: 1px solid var(--linea);
  }
  span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  b {
    font-weight: 600;
  }
  small {
    font-size: 12px;
  }
</style>
