// Los datos que manda Rust se definen una sola vez, en crates/xyra-core: `cargo test` los regenera en ./generado.
export type { AjusteJuego } from './generado/AjusteJuego';
export type { AumentoFila } from './generado/AumentoFila';
export type { Build } from './generado/Build';
export type { CampeonInfo } from './generado/CampeonInfo';
export type { Carta } from './generado/Carta';
export type { Config } from './generado/Config';
export type { Estado } from './generado/Estado';
export type { Fase } from './generado/Fase';
export type { Fila } from './generado/Fila';
export type { Icono } from './generado/Icono';
export type { ModoBuild } from './generado/ModoBuild';
export type { Perfil } from './generado/Perfil';
export type { Rango } from './generado/Rango';
export type { Resumen } from './generado/Resumen';

// Solo de la interfaz
export type Pagina = 'inicio' | 'build' | 'aumentos' | 'campeones' | 'stats' | 'etiquetas' | 'juego' | 'ajustes';
export type PestanaAjustes = 'general' | 'perfil' | 'datos' | 'seguridad' | 'ayuda' | 'acerca';
