# Seguridad y juego limpio

Xyra está pensado para hacer **lo mismo que las apps que Riot permite** (Blitz, OP.GG, Mobalytics muestran tiers de
aumentos dentro del juego) y nada más. Estas reglas son parte del proyecto: un cambio que rompa alguna no se acepta.

## Lo que Xyra NUNCA hace

| Regla | Por qué |
|---|---|
| No lee ni escribe la memoria del juego | Es lo primero que vigila Vanguard y lo que usan los cheats. |
| No inyecta código en el juego ni engancha DirectX | Blitz y Discord lo hacen porque Riot los tiene aprobados; para cualquier otro es motivo de baneo. |
| No abre el proceso `League of Legends.exe` | Vanguard protege ese proceso. Para saber si el juego está al frente solo se lee el **título** de la ventana. |
| No simula teclas ni clics, ni usa ganchos de teclado o atajos globales | Actuar por el jugador está prohibido por la política de Riot. |
| No juega por ti: no elige campeón ni carta | Idem. Xyra **sugiere**; el jugador decide y hace clic. |
| No revela información oculta (nombres en ranked, temporizadores enemigos, etc.) | Riot prohíbe exponer información que el juego oculta a propósito. |
| No muestra anuncios | Riot prohibió los anuncios dentro del juego en 2025. |

## Lo que sí hace

- **Captura la pantalla** (como OBS o Discord) solo durante partidas de ARAM: Caos o Arena y solo con el juego al frente,
  y reconoce el texto con el OCR que trae Windows.
- **Dibuja encima** en una ventana propia, transparente, que deja pasar los clics y nunca toma el foco.
  Por eso necesita el juego en modo **Sin bordes**: en pantalla completa exclusiva Windows no deja mostrar nada encima.
- **Escucha al cliente local (LCU)** por su WebSocket, como hacen Blitz o Porofessor: se entera al instante de la fase
  (selección, partida, fin), de tu campeón y de la cuenta con la que entras, sin consultarlo a cada rato. La conexión es
  solo con `127.0.0.1` y verifica el certificado del cliente con la raíz oficial de Riot (`riotgames.pem`).
- **Consulta, solo lectura,** los nombres e íconos de aumentos y campeones, qué campeones tiene tu cuenta (para no
  recomendarte uno bloqueado), tu historial reciente de Caos y Arena (para Estadísticas) y tu perfil: nombre, nivel,
  región, rango y maestrías (para Ajustes → Perfil). Solo tus propios datos, que el cliente ya te muestra.
- **En la selección de la Grieta** mira los campeones rivales que el cliente ya muestra y te sugiere counters con
  estadísticas públicas de OP.GG, como las webs de builds. No revela nada que el juego oculte.
- **Escribe en el cliente solo cuando tocas "Importar"** en la página Build: crea una página de runas y un set de ítems
  para la tienda, y en la selección de campeones pone tus dos hechizos (lo mismo que hacen Blitz, OP.GG o Mobalytics).
  Nunca lo hace durante la partida y solo reemplaza las páginas y sets que él mismo creó (los que empiezan con "Xyra · ").
  Si activas **"Importar build sola"** (apagado de fábrica), hace lo mismo en la selección de campeones sin que toques
  el botón: es una preferencia del cliente, no una jugada.
- **Acepta la partida encontrada solo si activas "Aceptar partida solo"** (apagado de fábrica), después de la espera que
  elijas y solo si no la aceptaste ni rechazaste tú. Es una llamada al cliente, como hacía League Akari: no toca el juego.
  Aun así, Riot pidió en 2025 a las apps aprobadas quitar esta función, así que úsala bajo tu responsabilidad.
- **Cambia opciones oficiales del juego** desde la página Juego (sin bordes, rango de ataque, cronómetros del minimapa,
  rango de torres, minimapa): las mismas del menú Opciones del LoL, por la API del cliente, y solo las de una lista fija
  (`crates/xyra-core/src/game_settings.rs`). Los cronómetros son los que trae el propio juego: Xyra no dibuja cronómetros
  propios (Riot prohíbe, por ejemplo, los de definitivas enemigas).
- **Se conecta a internet solo** con OP.GG (estadísticas públicas de aumentos, builds y enfrentamientos),
  CommunityDragon (íconos) y GitHub (para ver si hay una versión nueva y, si tocas "Descargar e instalar", bajar su
  instalador y comprobar su SHA-256 antes de abrirlo), con la verificación de certificados normal de Windows. Sin telemetría, sin cuentas, sin
  servidores propios.

## Datos en tu PC

Todo queda en `%APPDATA%\com.indagalab.xyra\`: `config.json`, `stats.json` (tus partidas, separadas por cuenta), `profile.json` (tu nombre,
nivel y maestrías, para verlos con el LoL cerrado), `catalog.json` (nombres e íconos), `xyra.log` (solo errores; se reinicia
al pasar de 1 MB) y `screenshots\` (solo si activas "Guardar capturas"). Desde *Ajustes → Datos* puedes exportar tus
partidas a CSV o borrarlo todo.

## Riesgo

Ninguna herramienta externa puede garantizar que Riot no cambie sus reglas. Xyra se limita a lo que hacen las apps
aprobadas, pero úsalo bajo tu responsabilidad. Si Riot anuncia cambios que lo afecten, se publicará un aviso en el
repositorio.

## Reportar un problema de seguridad

Abre un *issue* con la etiqueta `seguridad` o, si es sensible, escribe al mantenedor antes de publicarlo.

---

## Security & fair play (English)

Xyra only does what Riot-approved apps do. It **never** reads or writes game memory, injects code, hooks DirectX,
opens the game process, simulates input, plays or picks for you, reveals hidden information or shows ads. It captures the
screen (like OBS) only during ARAM: Mayhem/Arena games with the game focused, recognizes text with Windows' built-in OCR,
and draws on its own click-through window (requires **Borderless** mode). It listens to the local client over its
WebSocket on `127.0.0.1`, verifying its certificate against Riot's root (read-only; it only writes a rune page or an
item set when you click "Import"), suggests Summoner's Rift counters from public OP.GG stats, and only talks to OP.GG, CommunityDragon and GitHub
(update check and installer, verified by SHA-256) over the internet. The optional match auto-accept (off by default) is a client call, but Riot asked approved
apps to drop it in 2025: use it at your own risk. No telemetry.