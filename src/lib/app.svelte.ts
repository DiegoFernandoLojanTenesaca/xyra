// Estado de la interfaz en un solo lugar: lo que manda el motor en Rust más la navegación.
// Las páginas lo importan en vez de recibirlo por props.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { idiomaEfectivo, textos } from './i18n';
import type { CampeonInfo, Config, Estado, ModoBuild, Pagina, Perfil, PestanaAjustes, Resumen } from './tipos';

class App {
  // null solo hasta la primera respuesta del motor: la interfaz no se dibuja antes (ver `listo`)
  estado = $state<Estado>(null!);
  config = $state<Config>(null!);
  listo = $derived(!!this.estado && !!this.config);
  stats = $state<Resumen | null>(null);
  campeones = $state<CampeonInfo[]>([]);
  perfil = $state<Perfil | null>(null);
  pagina = $state<Pagina>('inicio');
  pestana = $state<PestanaAjustes>('general');
  /** Campeón elegido en Build y Aumentos (se puede abrir desde otras páginas). */
  elegido = $state<number | null>(null);
  /** Build de ARAM o de la Grieta y, en la Grieta, la posición (null = la más jugada). */
  modoBuild = $state<ModoBuild>('aram');
  posicion = $state<string | null>(null);
  /** Partidas nuevas desde la última visita a Estadísticas: el "+1" de la barra lateral. */
  nuevas = $state(0);
  T = $derived(textos(idiomaEfectivo(this.config?.idioma ?? 'auto', this.estado?.idioma_cliente ?? '')));

  /** Carga todo y escucha al motor. Devuelve la función para dejar de escuchar. */
  iniciar = () => {
    invoke<Estado>('get_estado').then((e) => (this.estado = e));
    invoke<Config>('get_config').then((c) => (this.config = c));
    this.recargar();
    this.cargarPerfil();
    const quitar = [
      listen<Estado>('estado', (e) => {
        // al conectar con el cliente se puede leer el perfil
        if (this.estado?.fase === 'sin_lol' && e.payload.fase !== 'sin_lol') this.cargarPerfil();
        this.estado = e.payload;
      }),
      listen<Config>('config', (e) => (this.config = e.payload)),
      // partidas importadas o catálogo recargado
      listen('stats', () => this.recargar()),
    ];
    return () => quitar.forEach((q) => void q.then((f) => f()));
  };

  recargar = async () => {
    const antes = this.stats?.partidas;
    const stats = await invoke<Resumen>('get_stats');
    if (antes !== undefined && stats.partidas > antes && this.pagina !== 'stats') this.nuevas += stats.partidas - antes;
    this.stats = stats;
    this.campeones = await invoke<CampeonInfo[]>('get_campeones');
  };

  cargarPerfil = async () => {
    this.perfil = await invoke<Perfil | null>('get_perfil');
  };

  guardar = async (cambios: Partial<Config>) => {
    this.config = await invoke<Config>('set_config', { config: { ...this.config, ...cambios } });
  };

  ir = (p: Pagina) => {
    this.pagina = p;
    if (p === 'stats') this.nuevas = 0;
  };

  verAumentos = (id: number) => {
    this.elegido = id;
    this.ir('aumentos');
  };

  verBuild = (id: number) => {
    this.elegido = id;
    this.seguirPartida();
    this.ir('build');
  };

  /** Modo y posición de la build según la selección o la partida en curso (Grieta si es una normal o clasificatoria). */
  seguirPartida = () => {
    const modo = this.estado.seleccion?.modo || this.estado.modo;
    if (!modo) return;
    this.modoBuild = modo === 'CLASSIC' ? 'grieta' : 'aram';
    this.posicion = this.estado.seleccion?.posicion ?? null;
  };

  /** Si no hay campeón elegido: el de la selección, el de la partida, tu más jugado o el primero de la tier list. */
  campeonPorDefecto = () => {
    if (this.elegido !== null || !this.campeones.length) return;
    const enPartida = this.campeones.find((c) => c.nombre === this.estado.campeon)?.id;
    this.elegido = this.estado.seleccion?.campeon?.id ?? enPartida ?? this.stats?.campeones[0]?.id ?? this.campeones[0].id;
    this.seguirPartida();
  };

  /** Lleva runas o ítems de la build al cliente. Devuelve el mensaje para mostrar (éxito o motivo del fallo). */
  importarBuild = async (campeon: number, que: 'runas' | 'items') => {
    const T = this.T.build;
    try {
      await invoke('importar_build', { campeon, que, modo: this.modoBuild, posicion: this.posicion });
      return que === 'runas' ? T.ok_runas : T.ok_items;
    } catch (e) {
      return e === 'sin_cliente' ? T.sin_cliente : e === 'sin_espacio' ? T.sin_espacio : String(e);
    }
  };

  abrirAjustes = (p: PestanaAjustes) => {
    this.pestana = p;
    this.ir('ajustes');
  };
}

export const app = new App();

/** Porcentaje entero, 0 si no hay total. */
export const pct = (v: number, n: number) => (n ? Math.round((100 * v) / n) : 0);
