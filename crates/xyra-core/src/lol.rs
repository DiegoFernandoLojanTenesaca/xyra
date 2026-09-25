//! Todo lo que se consulta al LoL: dónde está instalado, el cliente local (LCU) y la API oficial de la partida.
use base64::Engine;
use reqwest::blocking::Client;
use serde_json::Value;
use std::{fs, path::{Path, PathBuf}, time::Duration};

const RIOT_META: &str = r"C:\ProgramData\Riot Games\Metadata\league_of_legends.live\league_of_legends.live.product_settings.yaml";
const CARPETA_POR_DEFECTO: &str = r"C:\Riot Games\League of Legends";

/// El cliente y el juego usan certificados autofirmados en 127.0.0.1.
pub fn cliente_http() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .user_agent("xyra")
        .build()
        .expect("cliente http")
}

pub struct Meta {
    pub carpeta: PathBuf,
    /// Idioma del cliente, p. ej. "es_MX".
    pub idioma: String,
}

fn entre_comillas(linea: &str) -> Option<String> {
    let a = linea.find('"')? + 1;
    let b = linea[a..].find('"')?;
    Some(linea[a..a + b].to_string())
}

/// Carpeta e idioma del LoL según los metadatos que deja el Riot Client.
pub fn riot_meta() -> Meta {
    let texto = fs::read_to_string(RIOT_META).unwrap_or_default();
    let mut carpeta = None;
    let mut idioma = None;
    for l in texto.lines() {
        if l.starts_with("product_install_full_path:") {
            carpeta = entre_comillas(l);
        } else if l.starts_with(' ') && l.trim_start().starts_with("locale:") {
            idioma = entre_comillas(l);
        }
    }
    Meta {
        carpeta: PathBuf::from(carpeta.unwrap_or_else(|| CARPETA_POR_DEFECTO.into())),
        idioma: idioma.unwrap_or_else(|| "en_US".into()),
    }
}

/// Cliente local del LoL (LCU). Solo existe mientras el cliente está abierto.
pub struct Lcu {
    url: String,
    auth: String,
}

impl Lcu {
    pub fn conectar(carpeta: &Path) -> Option<Lcu> {
        let texto = fs::read_to_string(carpeta.join("lockfile")).ok()?;
        let partes: Vec<&str> = texto.split(':').collect();
        if partes.len() < 5 {
            return None;
        }
        let clave = base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", partes[3]));
        Some(Lcu { url: format!("https://127.0.0.1:{}", partes[2]), auth: format!("Basic {clave}") })
    }

    pub fn get(&self, http: &Client, ruta: &str) -> Result<Value, String> {
        self.pedir(http, reqwest::Method::GET, ruta, None)
    }

    /// Cambios en el cliente. Solo se usan cuando el jugador toca "Importar" (runas o set de ítems).
    pub fn pedir(&self, http: &Client, metodo: reqwest::Method, ruta: &str, cuerpo: Option<&Value>) -> Result<Value, String> {
        let mut r = http.request(metodo, format!("{}{}", self.url, ruta)).header("Authorization", &self.auth);
        if let Some(c) = cuerpo {
            r = r.json(c);
        }
        let r = r.send().map_err(|e| e.to_string())?;
        let estado = r.status();
        let texto = r.text().unwrap_or_default();
        if !estado.is_success() {
            return Err(format!("{estado}: {texto}"));
        }
        Ok(serde_json::from_str(&texto).unwrap_or(Value::Null))
    }
}

pub struct EnPartida {
    /// Alias interno del campeón ("MonkeyKing"), para cruzarlo con los datos del cliente.
    pub alias: String,
    /// Nombre del campeón en el idioma del jugador.
    pub campeon: String,
    /// "KIWI" = ARAM: Caos, "CHERRY" = Arena.
    pub modo: String,
}

/// Partida en curso según la API oficial del juego, o None (sin partida o pantalla de carga).
pub fn partida(http: &Client) -> Option<EnPartida> {
    let d: Value = http
        .get("https://127.0.0.1:2999/liveclientdata/allgamedata")
        .timeout(Duration::from_secs(2))
        .send()
        .ok()?
        .json()
        .ok()?;
    let yo = d["activePlayer"]["riotId"].as_str()?;
    let p = d["allPlayers"].as_array()?.iter().find(|p| p["riotId"].as_str() == Some(yo))?;
    Some(EnPartida {
        alias: p["rawChampionName"].as_str()?.rsplit('_').next()?.to_string(),
        campeon: p["championName"].as_str()?.to_string(),
        modo: d["gameData"]["gameMode"].as_str()?.to_string(),
    })
}

fn game_cfg(carpeta: &Path) -> PathBuf {
    carpeta.join("Config").join("game.cfg")
}

/// ¿El juego está en modo "Sin bordes"? (WindowMode=2). None si no se pudo leer.
pub fn sin_bordes(carpeta: &Path) -> Option<bool> {
    let texto = fs::read_to_string(game_cfg(carpeta)).ok()?;
    let valor = texto.lines().find_map(|l| l.trim().strip_prefix("WindowMode="))?;
    Some(valor.trim() == "2")
}

/// Pone el juego en "Sin bordes" editando game.cfg (con copia de respaldo). Solo con el juego cerrado.
pub fn poner_sin_bordes(carpeta: &Path, http: &Client) -> Result<(), String> {
    if partida(http).is_some() {
        return Err("partida".into());
    }
    let ruta = game_cfg(carpeta);
    let texto = fs::read_to_string(&ruta).map_err(|e| e.to_string())?;
    let respaldo = ruta.with_extension("cfg.antes-xyra");
    if !respaldo.exists() {
        fs::write(&respaldo, &texto).map_err(|e| e.to_string())?;
    }
    let nuevo: String = texto
        .split_inclusive('\n')
        .map(|l| {
            if l.trim_start().starts_with("WindowMode=") {
                let fin = if l.ends_with("\r\n") { "\r\n" } else if l.ends_with('\n') { "\n" } else { "" };
                format!("WindowMode=2{fin}")
            } else {
                l.to_string()
            }
        })
        .collect();
    fs::write(&ruta, nuevo).map_err(|e| e.to_string())
}

/// Ruta de un recurso del cliente (/lol-game-data/assets/...) servida por CommunityDragon, para usarla sin el cliente.
pub fn url_recurso(ruta: &str) -> String {
    let resto = ruta.trim_start_matches("/lol-game-data/assets/").to_lowercase();
    format!("https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/{resto}")
}
