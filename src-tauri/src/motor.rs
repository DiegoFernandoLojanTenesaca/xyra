//! Bucle principal: detecta la partida, lee las cartas de la pantalla y dibuja las etiquetas en la capa nativa.
use crate::{
    capa::{self, Capa},
    pantalla,
    voz::{self, Voz},
};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use xyra_core::{
    cartas::{self, Accion, Candidata, Carta, Detector},
    catalogo::Catalogo,
    config::Config,
    importar, juego, lol,
    modelo::{Build, CampeonInfo, Estado, Fase, ModoBuild, Seleccion},
    opgg, stats,
};

pub struct Rutas {
    /// Carpeta de datos de la app: todo lo de abajo vive aquí.
    pub datos: PathBuf,
    pub config: PathBuf,
    pub stats: PathBuf,
    pub catalogo: PathBuf,
    pub perfil: PathBuf,
    pub registro: PathBuf,
    pub capturas: PathBuf,
}

/// Estado compartido entre el motor, los comandos de la interfaz y la bandeja.
pub struct Compartido {
    pub config: Mutex<Config>,
    pub estado: Mutex<Estado>,
    pub stats: Mutex<Vec<stats::Guardada>>,
    pub catalogo: Mutex<Option<Catalogo>>,
    /// Tier list de campeones de ARAM: Caos (OP.GG): id -> (tier, puesto).
    pub tiers: Mutex<HashMap<u32, (u8, u32)>>,
    pub rutas: Rutas,
    pub meta: lol::Meta,
    pub http: reqwest::blocking::Client,
    /// Pedido de la interfaz: mostrar etiquetas de ejemplo.
    pub demo: AtomicBool,
}

impl Compartido {
    pub fn nuevo(datos: PathBuf) -> Compartido {
        let rutas = Rutas {
            config: datos.join("config.json"),
            stats: datos.join("stats.json"),
            catalogo: datos.join("catalogo.json"),
            perfil: datos.join("perfil.json"),
            registro: datos.join("xyra.log"),
            capturas: datos.join("capturas"),
            datos: datos.clone(),
        };
        let _ = fs::create_dir_all(&datos);
        // el registro se reinicia si pasa de 1 MB
        if fs::metadata(&rutas.registro).is_ok_and(|m| m.len() > 1_000_000) {
            let _ = fs::remove_file(&rutas.registro);
        }
        let meta = lol::riot_meta();
        Compartido {
            config: Mutex::new(Config::cargar(&rutas.config)),
            estado: Mutex::new(Estado {
                fase: Fase::SinLol,
                campeon: None,
                modo: None,
                cartas: Vec::new(),
                seleccion: None,
                sin_bordes: lol::sin_bordes(&meta.carpeta),
                idioma_cliente: meta.idioma.clone(),
                ocr: None,
                version: env!("CARGO_PKG_VERSION").into(),
            }),
            stats: Mutex::new(stats::cargar(&rutas.stats)),
            catalogo: Mutex::new(Catalogo::cargar(&rutas.catalogo)),
            tiers: Mutex::new(HashMap::new()),
            rutas,
            meta,
            http: lol::cliente_http(),
            demo: AtomicBool::new(false),
        }
    }

    pub fn registrar(&self, texto: &str) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.rutas.registro) {
            let ahora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let _ = writeln!(f, "{ahora} {texto}");
        }
    }

    pub fn campeon(&self, id: u32) -> CampeonInfo {
        let (nombre, icono) = self.catalogo.lock().unwrap().as_ref().and_then(|c| c.campeones.get(&id).cloned()).unwrap_or_default();
        let t = self.tiers.lock().unwrap().get(&id).copied();
        CampeonInfo { id, nombre, icono, tier: t.map(|x| x.0), puesto: t.map(|x| x.1) }
    }

    pub fn config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    /// Build de OP.GG con nombres e íconos del catálogo.
    pub fn build(&self, campeon: u32, modo: ModoBuild, posicion: Option<&str>) -> Result<Build, String> {
        // copia del catálogo: no se bloquea al motor mientras se espera a OP.GG
        let cat = self.catalogo.lock().unwrap().clone().unwrap_or_default();
        opgg::build(&self.http, campeon, modo, posicion, &cat)
    }

    /// Lleva la build al cliente. `que`: "runas" o "items". Errores: "sin_cliente", "sin_espacio" u otro texto.
    pub fn importar(&self, campeon: u32, modo: ModoBuild, posicion: Option<&str>, que: &str) -> Result<(), String> {
        let lcu = lol::Lcu::conectar(&self.meta.carpeta).ok_or("sin_cliente")?;
        let b = self.build(campeon, modo, posicion)?;
        let nombre = self.campeon(campeon).nombre;
        if que == "runas" {
            return importar::runas(&lcu, &self.http, &b, &nombre);
        }
        let es = self.config().idioma_efectivo(&self.meta.idioma) == "es";
        let bloques = if es { ["Inicio", "Botas", "Núcleo", "Situacionales"] } else { ["Starting items", "Boots", "Core", "Situational"] };
        importar::items(&lcu, &self.http, &b, &nombre, bloques)
    }

    fn textos(&self, config: &Config) -> (&'static capa::Textos, &'static str) {
        let idioma = config.idioma_efectivo(&self.meta.idioma);
        (if idioma == "es" { &capa::ES } else { &capa::EN }, idioma)
    }
}

fn actualizar_estado(app: &AppHandle, c: &Compartido, cambio: impl FnOnce(&mut Estado)) {
    let nuevo = {
        let mut e = c.estado.lock().unwrap();
        let antes = e.clone();
        cambio(&mut e);
        (*e != antes).then(|| e.clone())
    };
    if let Some(e) = nuevo {
        let _ = app.emit_to("main", "estado", e);
    }
}

fn importar_stats(c: &Compartido, lcu: &lol::Lcu) -> usize {
    let mut lista = c.stats.lock().unwrap();
    match stats::importar(lcu, &c.http, &mut lista) {
        Ok(n) if n > 0 => {
            stats::guardar(&c.rutas.stats, &lista);
            n
        }
        _ => 0,
    }
}

fn guardar_captura(c: &Compartido, ancho: i32, alto: i32) {
    let Some(bgra) = pantalla::capturar(0, 0, ancho, alto) else { return };
    let rgba: Vec<u8> = bgra.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], 255]).collect();
    let _ = fs::create_dir_all(&c.rutas.capturas);
    let nombre = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let Ok(archivo) = fs::File::create(c.rutas.capturas.join(format!("{nombre}.png"))) else { return };
    let mut png = png::Encoder::new(std::io::BufWriter::new(archivo), ancho as u32, alto as u32);
    png.set_color(png::ColorType::Rgba);
    if let Ok(mut w) = png.write_header() {
        let _ = w.write_image_data(&rgba);
    }
}

/// Tu campeón y la banca durante la selección de campeones (solo lectura del cliente).
fn leer_seleccion(c: &Compartido, lcu: &lol::Lcu) -> Option<Seleccion> {
    if lcu.get(&c.http, "/lol-gameflow/v1/gameflow-phase").ok()?.as_str()? != "ChampSelect" {
        return None;
    }
    let s = lcu.get(&c.http, "/lol-champ-select/v1/session").ok()?;
    let yo = s["localPlayerCellId"].as_i64()?;
    let celda = s["myTeam"].as_array()?.iter().find(|m| m["cellId"].as_i64() == Some(yo))?;
    let mio = celda["championId"].as_u64().unwrap_or(0);
    let banca = s["benchChampions"].as_array().into_iter().flatten().filter_map(|b| b["championId"].as_u64()).map(|id| c.campeon(id as u32));
    let modo = lcu.get(&c.http, "/lol-gameflow/v1/session").ok().and_then(|g| g["gameData"]["queue"]["gameMode"].as_str().map(String::from));
    // el cliente nombra las posiciones distinto que OP.GG
    let posicion = match celda["assignedPosition"].as_str().unwrap_or_default() {
        "top" => Some("top"),
        "jungle" => Some("jungle"),
        "middle" => Some("mid"),
        "bottom" => Some("adc"),
        "utility" => Some("support"),
        _ => None,
    };
    Some(Seleccion {
        campeon: (mio > 0).then(|| c.campeon(mio as u32)),
        banca: banca.collect(),
        modo: modo.unwrap_or_default(),
        posicion: posicion.map(String::from),
    })
}

/// Cartas de ejemplo en el centro de la pantalla, para el botón "Probar etiquetas" y las vistas previas.
/// Posiciones medidas en una partida real (docs/fondo-cartas.png).
fn cartas_demo(aumentos: &HashMap<u32, (String, String)>, ancho: i32, alto: i32) -> Vec<Carta> {
    let k = alto as f64 / 1200.0;
    let (cx, y) = (ancho as f64 / 2.0, alto as f64 * 0.4175);
    let demo = [(1211, -410.0, Some(0), 83.0), (1098, 0.0, Some(5), 70.0), (2128, 410.0, Some(0), 91.0)];
    let candidatas: Vec<Candidata> = demo.iter().map(|&(id, dx, ..)| Candidata { ids: vec![id], x: cx + dx * k, y }).collect();
    let datos = demo.iter().filter_map(|&(id, _, t, p)| Some((id, opgg::Dato { tier: t?, perf: p, popular: 0.0 }))).collect();
    cartas::calificar(&candidatas, &datos, aumentos)
}

/// Bucle del motor. Corre en su propio hilo mientras la app esté abierta; es dueño de la capa.
pub fn correr(app: AppHandle, c: Arc<Compartido>) {
    pantalla::iniciar_hilo();
    let ocr = pantalla::Ocr::nuevo(&c.meta.idioma);
    let idioma_ocr = ocr.as_ref().map(|o| o.idioma.clone());
    actualizar_estado(&app, &c, |e| e.ocr = idioma_ocr.clone());
    let (ancho, alto) = pantalla::tamano();
    let capa = Capa::nueva(ancho, alto).map_err(|e| c.registrar(&format!("capa: {e}"))).ok();
    let voz = Voz::nueva();
    c.registrar(&format!(
        "inicio v{}: carpeta={:?} idioma={} ocr={:?} pantalla={ancho}x{alto} capa={} voz={}",
        env!("CARGO_PKG_VERSION"),
        c.meta.carpeta,
        c.meta.idioma,
        idioma_ocr,
        capa.is_some(),
        voz.is_some()
    ));

    // la tier list de campeones no depende del cliente: se pide ya, para verla aunque el LoL esté cerrado
    thread::spawn({
        let (app, c) = (app.clone(), Arc::clone(&c));
        move || match opgg::tiers_campeones(&c.http) {
            Ok(t) => {
                *c.tiers.lock().unwrap() = t;
                let _ = app.emit_to("main", "stats", ());
            }
            Err(e) => c.registrar(&format!("OP.GG tiers: {e}")),
        }
    });

    // zona central donde salen las cartas
    let zona = (ancho / 8, alto / 10, ancho * 3 / 4, alto * 7 / 10);
    let mut catalogo_fresco = false; // el del disco puede ser de otro idioma o parche: se recarga una vez por sesión del cliente
    let mut datos: HashMap<u32, opgg::Dato> = HashMap::new();
    let mut datos_de: Option<(u32, String)> = None;
    let mut detector = Detector::default();
    let mut en_partida_antes = false;
    // runas automáticas: el campeón que te tocó, desde cuándo, y el último para el que ya se importaron
    let mut candidato_runas: Option<(u32, Instant)> = None;
    let mut runas_de: Option<u32> = None;
    let mut importar_desde: Option<Instant> = None;
    let mut ultima_revision = Instant::now() - Duration::from_secs(60);
    let mut ultimo_diagnostico = Instant::now();
    let mut demo_hasta: Option<Instant> = None;

    // espera procesando los mensajes de la capa y los pedidos de demostración
    let esperar = |ms: u64, detector: &mut Detector, demo_hasta: &mut Option<Instant>| {
        let fin = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < fin {
            if let Some(capa) = &capa {
                capa.bombear();
                if c.demo.swap(false, Ordering::Relaxed) {
                    let config = c.config();
                    let (t, _) = c.textos(&config);
                    let demo = {
                        let cat = c.catalogo.lock().unwrap();
                        cartas_demo(cat.as_ref().map_or(&HashMap::new(), |x| &x.aumentos), ancho, alto)
                    };
                    if let Err(e) = capa.mostrar(&demo, "Brand", &config, t) {
                        c.registrar(&format!("demo: {e}"));
                    }
                    *demo_hasta = Some(Instant::now() + Duration::from_secs(5));
                    detector.reiniciar();
                }
                if demo_hasta.is_some_and(|h| Instant::now() > h) {
                    *demo_hasta = None;
                    capa.ocultar();
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    };
    let ocultar = |detector: &mut Detector| {
        if let Some(capa) = &capa {
            capa.ocultar();
        }
        detector.reiniciar();
    };

    loop {
        let config = c.config();
        let lcu = lol::Lcu::conectar(&c.meta.carpeta);

        // catálogo y estadísticas: al conectar con el cliente y al terminar cada partida
        if let (Some(lcu), false) = (&lcu, catalogo_fresco) {
            if let Ok(cat) = Catalogo::desde_cliente(lcu, &c.http) {
                cat.guardar(&c.rutas.catalogo);
                *c.catalogo.lock().unwrap() = Some(cat);
                catalogo_fresco = true;
                let n = importar_stats(&c, lcu);
                c.registrar(&format!("catálogo cargado; {n} partidas nuevas en estadísticas"));
                match opgg::tiers_campeones(&c.http) {
                    Ok(t) => *c.tiers.lock().unwrap() = t,
                    Err(e) => c.registrar(&format!("OP.GG tiers: {e}")),
                }
                // nombres, íconos y tiers nuevos: la interfaz recarga estadísticas y campeones
                let _ = app.emit_to("main", "stats", ());
            }
        }
        if lcu.is_none() {
            catalogo_fresco = false;
        }
        let partida = lol::partida(&c.http);
        if en_partida_antes && partida.is_none() {
            importar_desde = Some(Instant::now());
            c.registrar("fin de partida");
        }
        if !en_partida_antes {
            if let Some(p) = &partida {
                c.registrar(&format!("partida {} con {} ({})", p.modo, p.campeon, p.alias));
            }
        }
        // al empezar la partida la ventana se cierra si así se pidió: Xyra sigue en la bandeja sin navegador abierto
        if !en_partida_antes && partida.is_some() && config.cerrar_en_partida {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.close();
            }
        }
        en_partida_antes = partida.is_some();
        if let (Some(desde), Some(lcu)) = (importar_desde, &lcu) {
            // el historial tarda en registrar la partida: se reintenta un par de minutos
            if desde.elapsed() > Duration::from_secs(15) {
                let n = importar_stats(&c, lcu);
                if n > 0 {
                    let _ = app.emit_to("main", "stats", ());
                }
                if n > 0 || desde.elapsed() > Duration::from_secs(150) {
                    importar_desde = None;
                }
            }
        }

        let seleccion = if partida.is_none() { lcu.as_ref().and_then(|l| leer_seleccion(&c, l)) } else { None };
        let fase = match (&partida, &lcu) {
            _ if config.pausado => Fase::Pausado,
            (Some(_), _) => Fase::Partida,
            _ if seleccion.is_some() => Fase::Seleccion,
            (None, Some(_)) => Fase::Cliente,
            _ => Fase::SinLol,
        };
        let revisar_ventana = partida.is_none() && ultima_revision.elapsed() > Duration::from_secs(10);
        if revisar_ventana {
            ultima_revision = Instant::now();
            // Sin bordes automático: con el cliente abierto se usa su API (como el menú de Opciones); si no, game.cfg
            if config.auto_bordes && lol::sin_bordes(&c.meta.carpeta) == Some(false) {
                let r = match &lcu {
                    Some(l) => juego::cambiar(l, &c.http, "General", "WindowMode", 2.into()),
                    None => lol::poner_sin_bordes(&c.meta.carpeta, &c.http),
                };
                c.registrar(&format!("sin bordes automático: {r:?}"));
            }
        }

        // runas automáticas: cuando el campeón que te tocó no cambia por 3 s (en ARAM puedes cambiarlo con la banca)
        match seleccion.as_ref().and_then(|s| s.campeon.as_ref().map(|ch| (ch.id, s))) {
            Some((id, s)) if config.auto_runas && runas_de != Some(id) => match candidato_runas {
                Some((cid, desde)) if cid == id && desde.elapsed() > Duration::from_secs(3) => {
                    let r = c.importar(id, ModoBuild::de_juego(&s.modo), s.posicion.as_deref(), "runas");
                    c.registrar(&format!("runas automáticas campeón {id}: {r:?}"));
                    runas_de = Some(id);
                }
                Some((cid, _)) if cid == id => {}
                _ => candidato_runas = Some((id, Instant::now())),
            },
            Some(_) => {}
            None => {
                candidato_runas = None;
                runas_de = None;
            }
        }
        actualizar_estado(&app, &c, |e| {
            e.fase = fase.clone();
            e.campeon = partida.as_ref().map(|p| p.campeon.clone());
            e.modo = partida.as_ref().map(|p| p.modo.clone());
            e.seleccion = seleccion.clone();
            if revisar_ventana {
                e.sin_bordes = lol::sin_bordes(&c.meta.carpeta);
            }
        });

        let Some(p) = partida.filter(|p| !config.pausado && (p.modo == "KIWI" || (p.modo == "CHERRY" && config.arena))) else {
            if detector.atento() {
                ocultar(&mut detector);
            }
            esperar(if fase == Fase::Partida { 3000 } else { 1500 }, &mut detector, &mut demo_hasta);
            continue;
        };

        // datos de OP.GG del campeón (una vez por partida)
        let campeon = c.catalogo.lock().unwrap().as_ref().and_then(|cat| cat.alias.get(&p.alias).copied());
        let Some(campeon) = campeon else {
            c.registrar(&format!("campeón sin identificar: {}", p.alias));
            esperar(3000, &mut detector, &mut demo_hasta);
            continue;
        };
        if datos_de.as_ref() != Some(&(campeon, p.modo.clone())) {
            let r = if p.modo == "CHERRY" { opgg::arena(&c.http, campeon) } else { opgg::caos(&c.http, campeon) };
            match r {
                Ok(d) => {
                    c.registrar(&format!("OP.GG {} campeón {campeon}: {} aumentos con datos", p.modo, d.len()));
                    datos = d;
                    datos_de = Some((campeon, p.modo.clone()));
                }
                Err(e) => {
                    c.registrar(&format!("OP.GG: {e}"));
                    esperar(5000, &mut detector, &mut demo_hasta);
                    continue;
                }
            }
        }

        // solo se lee la pantalla con el juego al frente (si miras otra ventana no se lee ni se dibuja nada)
        let (Some(ocr), true) = (&ocr, pantalla::juego_al_frente()) else {
            if detector.atento() {
                ocultar(&mut detector);
                actualizar_estado(&app, &c, |e| e.cartas.clear());
            }
            esperar(1000, &mut detector, &mut demo_hasta);
            continue;
        };
        let lineas = pantalla::capturar(zona.0, zona.1, zona.2, zona.3)
            .and_then(|bgra| ocr.leer(&bgra, zona.2, zona.3).map_err(|e| c.registrar(&format!("ocr: {e}"))).ok())
            .unwrap_or_default();
        let candidatas = {
            let cat = c.catalogo.lock().unwrap();
            let Some(cat) = cat.as_ref() else { continue };
            cartas::fila(&cartas::buscar(&lineas, &cat.nombres, zona.0 as f64, zona.1 as f64), alto as f64)
        };
        if ultimo_diagnostico.elapsed() > Duration::from_secs(60) {
            ultimo_diagnostico = Instant::now();
            c.registrar(&format!("diagnóstico: ocr {} líneas, {} cartas", lineas.len(), candidatas.len()));
        }

        match detector.leer(candidatas) {
            Accion::Nada => {}
            Accion::Ocultar => {
                if let Some(capa) = &capa {
                    capa.ocultar();
                }
                actualizar_estado(&app, &c, |e| e.cartas.clear());
            }
            Accion::Mostrar(candidatas) => {
                let lista = {
                    let cat = c.catalogo.lock().unwrap();
                    let vacio = HashMap::new();
                    cartas::calificar(&candidatas, &datos, cat.as_ref().map_or(&vacio, |x| &x.aumentos))
                };
                c.registrar(&format!(
                    "cartas {} campeón {campeon}: {:?}",
                    p.modo,
                    lista.iter().map(|x| (x.id, x.tier, x.mejor)).collect::<Vec<_>>()
                ));
                actualizar_estado(&app, &c, |e| e.cartas = lista.clone());
                let (t, idioma) = c.textos(&config);
                if let Some(capa) = &capa {
                    if let Err(e) = capa.mostrar(&lista, &p.campeon, &config, t) {
                        c.registrar(&format!("capa: {e}"));
                    }
                }
                if config.voz {
                    if let (Some(v), Some(f)) = (&voz, voz::frase(&lista, idioma)) {
                        let _ = v.decir(f, idioma);
                    }
                }
                if config.grabar {
                    guardar_captura(&c, ancho, alto);
                }
            }
        }
        // con cartas en pantalla (o por confirmar) se lee más seguido: aparecen y se van al momento
        esperar(if detector.atento() { 250 } else { 700 }, &mut detector, &mut demo_hasta);
    }
}

/// `xyra --muestras <captura.png> <carpeta>`: dibuja los 5 estilos de la capa sobre una captura de las cartas
/// y guarda `<estilo>.png` en la carpeta. Así las vistas previas de la página Etiquetas son la capa real.
pub fn muestras(fondo: &std::path::Path, salida: &std::path::Path) -> Result<(), String> {
    let archivo = fs::File::open(fondo).map_err(|e| e.to_string())?;
    let mut lector = png::Decoder::new(std::io::BufReader::new(archivo)).read_info().map_err(|e| e.to_string())?;
    let mut fondo = vec![0; lector.output_buffer_size().ok_or("png")?];
    let info = lector.next_frame(&mut fondo).map_err(|e| e.to_string())?;
    let canales = info.color_type.samples();
    let (ancho, alto) = (info.width as i32, info.height as i32);
    let capa = Capa::nueva(ancho, alto).map_err(|e| e.to_string())?;
    let cartas = cartas_demo(&HashMap::new(), ancho, alto);
    fs::create_dir_all(salida).map_err(|e| e.to_string())?;
    for estilo in ["placa", "insignia", "cinta", "podio", "enfoque"] {
        let config = Config { estilo: estilo.into(), ..Default::default() };
        let capa_px = capa.imagen(&cartas, "Brand", &config, &capa::ES).map_err(|e| e.to_string())?;
        // capa (BGRA premultiplicado) sobre el fondo (RGB o RGBA)
        let rgb: Vec<u8> = capa_px
            .chunks_exact(4)
            .zip(fondo.chunks_exact(canales))
            .flat_map(|(c, f)| {
                let resto = 255 - c[3] as u32;
                [2, 1, 0].map(|i| (c[i] as u32 + f[2 - i] as u32 * resto / 255).min(255) as u8)
            })
            .collect();
        let destino = fs::File::create(salida.join(format!("{estilo}.png"))).map_err(|e| e.to_string())?;
        let mut png = png::Encoder::new(std::io::BufWriter::new(destino), ancho as u32, alto as u32);
        png.set_color(png::ColorType::Rgb);
        png.write_header().and_then(|mut w| w.write_image_data(&rgb)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
