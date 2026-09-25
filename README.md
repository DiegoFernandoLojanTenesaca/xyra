# Xyra

**Xyra** etiqueta las cartas de aumento de **ARAM: Caos** (ARAM: Mayhem) y **Arena** en League of Legends con qué tan
buenas son **para tu campeón**, con estadísticas de OP.GG. La mejor queda marcada y las malas te sugieren cambiarlas.
Liviano, solo para Windows, en español e inglés.

Un proyecto de **Xynitra · IndagaLab**.

![Placa](docs/placa.jpg)

*English version below.*

## Qué hace

- **Etiquetas sobre las cartas**, en 5 estilos (Placa, Insignia, Cinta, Podio, Enfoque), y un titular directo en la
  app: *"Elige ¡Comienza a Emocionarte!"*.
- **Sugiere cambiar** las cartas malas cuando en la mesa hay una buena.
- **Selección de campeones**: tier de tu campeón y de la banca, y te avisa si en la banca hay uno mejor.
- **Build**: runas, hechizos, ítems y orden de habilidades para ARAM: Caos y para la Grieta (normales, por posición),
  con botones para importar las runas y el set de ítems a tu cliente.
- **Juego**: activa en un clic opciones oficiales del LoL (rango de ataque, cronómetros del minimapa, rango de torres,
  tamaño del minimapa) y automatiza lo aburrido: mantener Sin bordes, importar runas solas, cerrar la ventana al jugar.
- **Tier lists** de aumentos por campeón y de campeones en ARAM: Caos.
- **Aviso por voz** opcional: "Elige la de la derecha".
- **Tus estadísticas y tu perfil**: winrate por campeón, tus mejores aumentos, nivel, rango y maestrías.
- **Tus datos**: exportar tus partidas a CSV o borrarlo todo desde Ajustes.
- **Arena (beta)**: califica los aumentos por su puesto promedio en OP.GG.
- **Calibración** de altura y tamaño para otras resoluciones, y modo "guardar capturas" para reportar fallos.
- Vive en la **bandeja del sistema** y solo trabaja durante tus partidas.

## Seguridad

No lee la memoria del juego, no le inyecta nada, no simula teclas ni clics y no automatiza nada: solo mira la pantalla y
dibuja encima. Todos los detalles en [SEGURIDAD.md](SEGURIDAD.md).

## Instalación

1. Descarga el instalador `Xyra_x.y.z_x64-setup.exe` desde [Releases](../../releases).
2. Pon el LoL en **Sin bordes** (*Opciones → Video → Modo de ventana*). Xyra te avisa si no lo está y puede cambiarlo
   por ti con el juego cerrado.
3. Juega ARAM: Caos. Xyra arranca con Windows y queda en la bandeja.

Requisitos: Windows 10 u 11 (WebView2 viene incluido en Windows 11) y el reconocimiento de texto de Windows para el idioma
de tu cliente (suele venir con el idioma de Windows).

## Desarrollo

Rust (núcleo, motor y capa nativa con Direct2D) + Tauri 2 y Svelte 5 (ventana principal).

```powershell
pnpm install
pnpm tauri dev              # desarrollo
pnpm tauri build            # instalador en target/release/bundle/nsis
cargo test --workspace      # pruebas; también regenera src/lib/generado
pnpm check                  # tipos de la interfaz
```

`xyra.exe --demo` muestra etiquetas de ejemplo al arrancar, para probar la capa sin jugar.

### Cómo está organizado

Cada cosa se define **una sola vez** y el resto la usa:

| Dónde | Qué es la fuente única de |
|---|---|
| `crates/xyra-core/` | La lógica que no depende de Windows: reconocer y calificar cartas, OP.GG, cliente del LoL, estadísticas, perfil, ajustes. Sin interfaz, con pruebas; se puede reutilizar (p. ej. en un compañero para el celular). |
| `crates/xyra-core/src/modelo.rs` | Los datos que ven las interfaces. `cargo test` los exporta a TypeScript en `src/lib/generado/` (ts-rs): Rust y la interfaz nunca se desalinean. |
| `src-tauri/` | La app de Windows: motor (`motor.rs`), captura y OCR (`pantalla.rs`), capa nativa (`capa.rs`), voz, bandeja y comandos. |
| `src/lib/tema.css` | Colores, tipografía y piezas visuales (paneles, botones, esquinas cortadas). Las páginas no definen colores. |
| `src/lib/app.svelte.ts` | El estado de la interfaz (lo que manda el motor, la navegación) y las acciones. Las páginas lo importan. |
| `src/lib/i18n.ts` | Todos los textos, en español e inglés. |
| `src/lib/ui/` | Componentes reutilizables: encabezado, cifras, tiers, interruptores, buscador, pestañas. |
| `src/lib/paginas/` | Las páginas: solo acomodan piezas de lo anterior. |

## Creadores

| | | |
|---|---|---|
| <img src="https://github.com/DiegoFernandoLojanTenesaca.png" width="48"> | **Diego Fernando** · [@DiegoFernandoLojanTenesaca](https://github.com/DiegoFernandoLojanTenesaca) | Creador · **IndagaLab** |
| <img src="https://github.com/jahirxtrap.png" width="48"> | **[@jahirxtrap](https://github.com/jahirxtrap)** | Cocreador · **Xynitra** |

Gracias a OP.GG por las estadísticas y a CommunityDragon por los íconos.

## Aviso

Proyecto de la comunidad. Xyra no está respaldado por Riot Games ni refleja sus opiniones; League of Legends es marca de
Riot Games, Inc. Sin relación con OP.GG. Úsalo bajo tu responsabilidad.

---

## English

**Xyra** labels **ARAM: Mayhem** and **Arena** augment cards with how good each one is **for your champion**, using
OP.GG stats. The best card is highlighted and bad ones suggest a reroll. Lightweight, Windows-only, Spanish/English.
By **Diego Fernando** (IndagaLab) and **@jahirxtrap** (Xynitra).

- 5 label styles, champion select tiers, builds with rune and item set import, augment and champion tier lists, optional voice hint, personal stats and
  profile, CSV export, Arena (beta), calibration for other resolutions.
- Screen reading + a native click-through overlay only: no memory access, no injection, no simulated input
  ([SEGURIDAD.md](SEGURIDAD.md)).
- Install from [Releases](../../releases), set League to **Borderless**, play.

Xyra isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games. MIT License.
