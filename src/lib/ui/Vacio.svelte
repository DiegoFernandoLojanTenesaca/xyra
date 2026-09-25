<script lang="ts">
  // Aviso centrado para cuando todavía no hay datos.
  import Icono from './Icono.svelte';

  let { icono, texto }: { icono: string; texto: string } = $props();
</script>

<div>
  <Icono nombre={icono} tam={34} />
  <p>{texto}</p>
</div>

<style>
  div {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 30px 20px;
    color: var(--tenue);
    text-align: center;
  }
  p {
    margin: 0;
    color: var(--suave);
  }
</style>
