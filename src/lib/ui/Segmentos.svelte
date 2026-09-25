<script lang="ts" generics="V extends string">
  // Grupo de opciones excluyentes. `pestanas` = subrayado rojo (Ajustes); si no, botones juntos (filtros).
  let {
    opciones,
    valor = $bindable(),
    pestanas = false,
  }: { opciones: [V, string][]; valor: V; pestanas?: boolean } = $props();
</script>

<div class:pestanas role="tablist">
  {#each opciones as [id, texto] (id)}
    <button role="tab" aria-selected={valor === id} class:on={valor === id} onclick={() => (valor = id)}>{texto}</button>
  {/each}
</div>

<style>
  div {
    display: flex;
    gap: 2px;
    padding: 3px;
    background: var(--panel);
    border: 1px solid var(--linea);
  }
  button {
    padding: 7px 14px;
    border: none;
    background: none;
    color: var(--suave);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 1.5px;
    text-transform: uppercase;
  }
  button:hover {
    color: var(--texto);
  }
  button.on {
    background: var(--rojo);
    color: #fff;
  }
  .pestanas {
    gap: 4px;
    padding: 0;
    margin-bottom: 22px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--linea);
  }
  .pestanas button {
    position: relative;
    padding: 10px 18px;
  }
  .pestanas button.on {
    background: none;
  }
  .pestanas button.on::after {
    content: '';
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: -1px;
    height: 3px;
    background: var(--rojo-2);
    box-shadow: 0 0 12px var(--rojo);
  }
</style>
