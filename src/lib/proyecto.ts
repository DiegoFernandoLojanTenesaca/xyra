// Datos del proyecto en un solo lugar: créditos, enlaces y novedades. Ayuda, Acerca de y la barra lateral los leen de aquí.
import { openUrl } from '@tauri-apps/plugin-opener';

// ponytail: repositorio aún sin publicar; confirmar la URL al subirlo a GitHub
export const REPO = 'https://github.com/DiegoFernandoLojanTenesaca/xyra';

export const ENLACES = {
  repo: REPO,
  problema: `${REPO}/issues/new?template=fallo.yml`,
  novedades: `${REPO}/releases`,
  opgg: 'https://op.gg/lol/modes/aram-mayhem',
  cdragon: 'https://www.communitydragon.org',
};

/** Quiénes hacen Xyra. `rol` es la clave del texto en i18n (T.creditos.roles). Las fotos van dentro de la app
 *  (static/creadores): Xyra no se conecta a GitHub para mostrarlas. */
export const CREADORES = [
  { nombre: 'Diego Fernando', usuario: 'DiegoFernandoLojanTenesaca', equipo: 'IndagaLab', rol: 'creador', foto: '/creadores/diego.jpg' },
  { nombre: '@jahirxtrap', usuario: 'jahirxtrap', equipo: 'Xynitra', rol: 'cocreador', foto: '/creadores/jahir.jpg' },
] as const;

export const perfilGithub = (usuario: string) => `https://github.com/${usuario}`;

/** Abre un enlace en el navegador del sistema (la ventana de Xyra no navega fuera de la app). */
export const abrir = (url: string) => void openUrl(url);
