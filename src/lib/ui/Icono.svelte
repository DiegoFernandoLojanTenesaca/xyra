<script lang="ts">
  // Íconos de línea (estilo Lucide) dibujados a mano: sin dependencias.
  let { nombre, tam = 18 }: { nombre: string; tam?: number } = $props();

  const TRAZOS: Record<string, string[]> = {
    inicio: ['M3 10.5 12 3l9 7.5', 'M5 9v11h5v-6h4v6h5V9'],
    aumentos: ['M7 3h10a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z', 'M12 7l1.5 3.2 3.5.4-2.6 2.4.7 3.5L12 14.8 8.9 16.5l.7-3.5L7 10.6l3.5-.4z'],
    campeones: ['M3 8l4.5 4L12 5l4.5 7L21 8l-2 11H5z', 'M5 19h14'],
    build: ['M14.5 3.5 20.5 9.5 10 20H4v-6z', 'M12.5 5.5l6 6', 'M4 20l4-4'],
    stats: ['M4 20V11', 'M10 20V5', 'M16 20v-7', 'M21 20H3'],
    juego: ['M6 9h12a4 4 0 0 1 0 8c-1.5 0-2.3-1-3-2H9c-.7 1-1.5 2-3 2a4 4 0 0 1 0-8z', 'M8 11v4', 'M6 13h4', 'M15.5 12.5h.01', 'M17.5 14.5h.01'],
    etiquetas: ['M4 12V5a1 1 0 0 1 1-1h7l8 8-8 8z', 'M8.5 8.5h.01'],
    ajustes: ['M4 7h9', 'M17 7h3', 'M4 17h3', 'M11 17h9', 'M15 5v4', 'M9 15v4'],
    escudo: ['M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z', 'M9 12l2 2 4-4'],
    minimizar: ['M5 12h14'],
    maximizar: ['M6 6h12v12H6z'],
    cerrar: ['M6 6l12 12', 'M18 6 6 18'],
    buscar: ['M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14z', 'M20 20l-4-4'],
    probar: ['M8 5v14l11-7z'],
    pausa: ['M8 5v14', 'M16 5v14'],
    ventana: ['M4 5h16v14H4z', 'M4 9h16'],
    ojo: ['M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z', 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z'],
    voz: ['M11 5 6 9H3v6h3l5 4z', 'M15.5 8.5a5 5 0 0 1 0 7', 'M18.5 5.5a9 9 0 0 1 0 13'],
    carpeta: ['M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z'],
    flecha: ['M5 12h14', 'M13 6l6 6-6 6'],
    tuerca: [
      'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
      'M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z',
    ],
    descargar: ['M12 4v11', 'M7 10l5 5 5-5', 'M4 20h16'],
    borrar: ['M4 7h16', 'M9 7V4h6v3', 'M6 7l1 13h10l1-13'],
    ayuda: ['M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z', 'M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6', 'M12 17h.01'],
    enlace: ['M14 4h6v6', 'M20 4l-9 9', 'M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5'],
  };
</script>

<svg width={tam} height={tam} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#each TRAZOS[nombre] ?? [] as d}
    <path {d} />
  {/each}
</svg>
