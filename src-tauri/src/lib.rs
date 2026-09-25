mod capa;
mod motor;
mod pantalla;
mod voz;

use motor::Compartido;
use std::{
    fs,
    path::Path,
    sync::{atomic::Ordering, Arc},
    thread,
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::Color,
    AppHandle, Emitter, Manager, RunEvent, State, Theme, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_opener::OpenerExt;
use xyra_core::{
    catalogo::Catalogo,
    config::Config,
    juego::{self, AjusteJuego},
    lol,
    modelo::{AumentoFila, Build, CampeonInfo, Estado, Fase, ModoBuild},
    opgg,
    perfil::{self, Perfil},
    stats,
};

type App = Arc<Compartido>;

const ESTILOS: [(&str, &str, &str); 5] = [
    ("placa", "Placa", "Plate"),
    ("insignia", "Insignia", "Badge"),
    ("cinta", "Cinta", "Ribbon"),
    ("podio", "Podio", "Podium"),
    ("enfoque", "Enfoque", "Focus"),
];

#[tauri::command]
fn get_estado(c: State<App>) -> Estado {
    c.estado.lock().unwrap().clone()
}

#[tauri::command]
fn get_config(c: State<App>) -> Config {
    c.config()
}

#[tauri::command]
fn set_config(app: AppHandle, c: State<App>, config: Config) -> Config {
    aplicar_config(&app, &c, config)
}

fn aplicar_config(app: &AppHandle, c: &Compartido, config: Config) -> Config {
    let antes = c.config();
    if config.autostart != antes.autostart {
        let a = app.autolaunch();
        let _ = if config.autostart { a.enable() } else { a.disable() };
    }
    config.guardar(&c.rutas.config);
    *c.config.lock().unwrap() = config.clone();
    let _ = app.emit_to("main", "config", config.clone());
    armar_bandeja(app, c);
    config
}

#[tauri::command]
fn get_stats(c: State<App>) -> stats::Resumen {
    let cat = c.catalogo.lock().unwrap();
    let vacio = Catalogo::default();
    stats::resumen(&c.stats.lock().unwrap(), cat.as_ref().unwrap_or(&vacio))
}

/// Muestra etiquetas de ejemplo 5 segundos en el centro de la pantalla (fuera de partida).
#[tauri::command]
fn probar_capa(c: State<App>) {
    if c.estado.lock().unwrap().fase != Fase::Partida {
        c.demo.store(true, Ordering::Relaxed);
    }
}

/// Todos los campeones con su tier en ARAM: Caos, ordenados por puesto.
#[tauri::command]
fn get_campeones(c: State<App>) -> Vec<CampeonInfo> {
    let ids: Vec<u32> = c.catalogo.lock().unwrap().as_ref().map(|x| x.campeones.keys().copied().collect()).unwrap_or_default();
    let mut lista: Vec<CampeonInfo> = ids.into_iter().map(|id| c.campeon(id)).collect();
    lista.sort_by(|a, b| (a.puesto.is_none(), a.puesto, &a.nombre).cmp(&(b.puesto.is_none(), b.puesto, &b.nombre)));
    lista
}

/// Tier list de aumentos de un campeón (modo "KIWI" = Caos, "CHERRY" = Arena), de mejor a peor.
#[tauri::command]
async fn get_aumentos(c: State<'_, App>, campeon: u32, modo: String) -> Result<Vec<AumentoFila>, String> {
    let c = Arc::clone(&c);
    tauri::async_runtime::spawn_blocking(move || {
        let datos = if modo == "CHERRY" { opgg::arena(&c.http, campeon)? } else { opgg::caos(&c.http, campeon)? };
        let cat = c.catalogo.lock().unwrap();
        let mut lista: Vec<AumentoFila> = datos
            .into_iter()
            .filter_map(|(id, d)| {
                let cat = cat.as_ref()?;
                let (nombre, icono) = cat.aumentos.get(&id)?.clone();
                Some(AumentoFila { id, nombre, icono, rareza: cat.rareza.get(&id).cloned().unwrap_or_default(), tier: d.tier, perf: d.perf, popular: d.popular })
            })
            .collect();
        lista.sort_by(|a, b| a.tier.cmp(&b.tier).then(b.perf.total_cmp(&a.perf)));
        Ok(lista)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Build de OP.GG de un campeón. `posicion`: solo en la Grieta; sin ella, la más jugada.
#[tauri::command]
async fn get_build(c: State<'_, App>, campeon: u32, modo: ModoBuild, posicion: Option<String>) -> Result<Build, String> {
    let c = Arc::clone(&c);
    tauri::async_runtime::spawn_blocking(move || c.build(campeon, modo, posicion.as_deref())).await.map_err(|e| e.to_string())?
}

/// Opciones oficiales del juego que Xyra deja cambiar (rango de ataque, cronómetros del minimapa...). Necesita el cliente.
#[tauri::command]
async fn get_ajustes_juego(c: State<'_, App>) -> Result<Vec<AjusteJuego>, String> {
    let c = Arc::clone(&c);
    tauri::async_runtime::spawn_blocking(move || {
        let lcu = lol::Lcu::conectar(&c.meta.carpeta).ok_or("sin_cliente")?;
        juego::leer(&lcu, &c.http)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_ajuste_juego(c: State<'_, App>, seccion: String, clave: String, valor: serde_json::Value) -> Result<(), String> {
    let c = Arc::clone(&c);
    tauri::async_runtime::spawn_blocking(move || {
        let lcu = lol::Lcu::conectar(&c.meta.carpeta).ok_or("sin_cliente")?;
        juego::cambiar(&lcu, &c.http, &seccion, &clave, valor)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Lleva la build al cliente, solo cuando el jugador toca "Importar". `que`: "runas" o "items".
/// Errores: "sin_cliente" (LoL cerrado), "sin_espacio" (no hay páginas de runas libres) u otro texto.
#[tauri::command]
async fn importar_build(c: State<'_, App>, campeon: u32, que: String, modo: ModoBuild, posicion: Option<String>) -> Result<(), String> {
    let c = Arc::clone(&c);
    tauri::async_runtime::spawn_blocking(move || c.importar(campeon, modo, posicion.as_deref(), &que)).await.map_err(|e| e.to_string())?
}

/// Tu perfil leído del cliente; con el LoL cerrado, el último que se leyó.
#[tauri::command]
async fn get_perfil(c: State<'_, App>) -> Result<Option<Perfil>, String> {
    let c = Arc::clone(&c);
    tauri::async_runtime::spawn_blocking(move || {
        let campeones = c.catalogo.lock().unwrap().as_ref().map(|x| x.campeones.clone()).unwrap_or_default();
        match lol::Lcu::conectar(&c.meta.carpeta).map(|lcu| perfil::leer(&lcu, &c.http, &campeones)) {
            Some(Ok(p)) => {
                perfil::guardar(&c.rutas.perfil, &p);
                Some(p)
            }
            _ => perfil::cargar(&c.rutas.perfil),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

fn peso(ruta: &Path) -> u64 {
    let entradas = fs::read_dir(ruta).into_iter().flatten().flatten();
    entradas.map(|e| e.metadata().map_or(0, |m| if m.is_dir() { peso(&e.path()) } else { m.len() })).sum()
}

/// Bytes que ocupan los datos de Xyra en el disco.
#[tauri::command]
fn peso_datos(c: State<App>) -> u64 {
    peso(&c.rutas.datos)
}

/// Guarda tus partidas como CSV en Descargas y lo muestra en el Explorador. Devuelve la ruta.
#[tauri::command]
fn exportar_csv(app: AppHandle, c: State<App>) -> Result<String, String> {
    let texto = {
        let cat = c.catalogo.lock().unwrap();
        let vacio = Catalogo::default();
        stats::csv(&c.stats.lock().unwrap(), cat.as_ref().unwrap_or(&vacio))
    };
    let ruta = app.path().download_dir().map_err(|e| e.to_string())?.join("xyra-partidas.csv");
    // BOM para que Excel lea bien las tildes
    fs::write(&ruta, format!("\u{feff}{texto}")).map_err(|e| e.to_string())?;
    let _ = app.opener().reveal_item_in_dir(&ruta);
    Ok(ruta.to_string_lossy().into())
}

/// Borra tus datos personales (partidas, perfil, capturas y registro). Los ajustes se quedan.
#[tauri::command]
fn borrar_datos(c: State<App>) {
    c.stats.lock().unwrap().clear();
    for f in [&c.rutas.stats, &c.rutas.perfil, &c.rutas.registro] {
        let _ = fs::remove_file(f);
    }
    let _ = fs::remove_dir_all(&c.rutas.capturas);
}

#[tauri::command]
fn poner_sin_bordes(c: State<App>) -> Result<(), String> {
    lol::poner_sin_bordes(&c.meta.carpeta, &c.http)?;
    c.estado.lock().unwrap().sin_bordes = lol::sin_bordes(&c.meta.carpeta);
    Ok(())
}

#[tauri::command]
fn abrir_carpeta(app: AppHandle, c: State<App>, que: String) {
    let ruta = match que.as_str() {
        "capturas" => c.rutas.capturas.clone(),
        "registro" => c.rutas.registro.clone(),
        _ => c.rutas.datos.clone(),
    };
    if que == "capturas" {
        let _ = fs::create_dir_all(&ruta);
    }
    let _ = app.opener().open_path(ruta.to_string_lossy(), None::<&str>);
}

/// La ventana principal se crea al abrirla y se destruye al cerrarla: mientras juegas no hay navegador corriendo.
fn mostrar_principal(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Xyra")
        .inner_size(1180.0, 760.0)
        .min_inner_size(960.0, 620.0)
        .center()
        .decorations(false) // barra de título propia (con los botones de ventana) dentro de la interfaz
        .shadow(true)
        .theme(Some(Theme::Dark))
        .background_color(Color(6, 6, 6, 255)) // el negro del tema: sin destello blanco al abrir
        .build();
}

/// Menú de la bandeja: abrir, pausa, estilo, voz y salir. Se rearma cuando cambia la configuración.
fn armar_bandeja(app: &AppHandle, c: &Compartido) {
    let config = c.config();
    let es = config.idioma_efectivo(&c.meta.idioma) == "es";
    let t = |a: &'static str, b: &'static str| if es { a } else { b };
    let menu = (|| -> tauri::Result<Menu<tauri::Wry>> {
        let estilos: Vec<CheckMenuItem<tauri::Wry>> = ESTILOS
            .iter()
            .map(|(id, nes, nen)| CheckMenuItem::with_id(app, format!("estilo:{id}"), if es { *nes } else { *nen }, true, config.estilo == *id, None::<&str>))
            .collect::<Result<_, _>>()?;
        let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = estilos.iter().map(|e| e as _).collect();
        Menu::with_items(
            app,
            &[
                &MenuItem::with_id(app, "abrir", t("Abrir Xyra", "Open Xyra"), true, None::<&str>)?,
                &PredefinedMenuItem::separator(app)?,
                &CheckMenuItem::with_id(app, "pausa", t("Pausar", "Pause"), true, config.pausado, None::<&str>)?,
                &Submenu::with_items(app, t("Estilo", "Style"), true, &refs)?,
                &CheckMenuItem::with_id(app, "voz", t("Aviso por voz", "Voice hint"), true, config.voz, None::<&str>)?,
                &PredefinedMenuItem::separator(app)?,
                &MenuItem::with_id(app, "salir", t("Salir", "Quit"), true, None::<&str>)?,
            ],
        )
    })();
    if let (Ok(menu), Some(tray)) = (menu, app.tray_by_id("principal")) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn al_elegir_menu(app: &AppHandle, id: &str) {
    let c: App = Arc::clone(app.state::<App>().inner());
    let mut config = c.config();
    match id {
        "abrir" => return mostrar_principal(app),
        "salir" => return app.exit(0),
        "pausa" => config.pausado = !config.pausado,
        "voz" => config.voz = !config.voz,
        otro => match otro.strip_prefix("estilo:") {
            Some(estilo) => config.estilo = estilo.into(),
            None => return,
        },
    }
    aplicar_config(app, &c, config);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--muestras") {
        let (fondo, salida) = (args.get(i + 1).map_or("docs/fondo-cartas.png", |s| s), args.get(i + 2).map_or("muestras", |s| s));
        if let Err(e) = motor::muestras(fondo.as_ref(), salida.as_ref()) {
            eprintln!("muestras: {e}");
        }
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| mostrar_principal(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--oculto"])))
        .setup(|app| {
            let datos = app.path().app_data_dir()?;
            let c: App = Arc::new(Compartido::nuevo(datos));
            app.manage(Arc::clone(&c));

            if c.config().autostart && !app.autolaunch().is_enabled().unwrap_or(false) {
                let _ = app.autolaunch().enable();
            }
            TrayIconBuilder::with_id("principal")
                .icon(app.default_window_icon().cloned().expect("ícono"))
                .tooltip("Xyra")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| al_elegir_menu(app, e.id().as_ref()))
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        mostrar_principal(tray.app_handle());
                    }
                })
                .build(app)?;
            armar_bandeja(app.handle(), &c);

            // --demo: etiquetas de ejemplo al arrancar (para probar la capa sin abrir la ventana)
            if std::env::args().any(|a| a == "--demo") {
                c.demo.store(true, Ordering::Relaxed);
            }
            // si ya hay una partida en curso no se abre la ventana (no le quita el foco al juego): queda en la bandeja
            let oculto = std::env::args().any(|a| a == "--oculto" || a == "--demo") || lol::partida(&c.http).is_some();
            if !oculto {
                mostrar_principal(app.handle());
            }
            let handle = app.handle().clone();
            thread::spawn(move || motor::correr(handle, c));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_estado,
            get_config,
            set_config,
            get_stats,
            get_campeones,
            get_aumentos,
            get_perfil,
            get_build,
            importar_build,
            get_ajustes_juego,
            set_ajuste_juego,
            peso_datos,
            exportar_csv,
            borrar_datos,
            probar_capa,
            poner_sin_bordes,
            abrir_carpeta
        ])
        .build(tauri::generate_context!())
        .expect("error al iniciar Xyra")
        .run(|_, evento| {
            // cerrar la última ventana no cierra la app: sigue en la bandeja (se sale desde su menú)
            if let RunEvent::ExitRequested { api, code: None, .. } = evento {
                api.prevent_exit();
            }
        });
}
