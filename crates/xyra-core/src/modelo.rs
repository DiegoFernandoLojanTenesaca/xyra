//! Datos que muestran las interfaces. Es la fuente única de estos tipos: `cargo test` los exporta a
//! TypeScript en `src/lib/generado/` (ts-rs), así Rust y la interfaz nunca se desalinean.
use crate::cartas::Carta;
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct CampeonInfo {
    pub id: u32,
    pub nombre: String,
    pub icono: String,
    /// Tier en ARAM: Caos según OP.GG: 1 = mejor … 5.
    pub tier: Option<u8>,
    pub puesto: Option<u32>,
}

/// Selección de campeones: el tuyo y los de la banca (lo mismo que ves en el cliente).
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Seleccion {
    pub campeon: Option<CampeonInfo>,
    pub banca: Vec<CampeonInfo>,
    /// Modo de la cola: "ARAM", "KIWI" (Caos), "CLASSIC" (Grieta), "CHERRY" (Arena)...
    pub modo: String,
    /// Posición asignada en la Grieta ("top", "jungle", "mid", "adc", "support"); None en ARAM o elección a ciegas.
    pub posicion: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Fase {
    SinLol,
    Cliente,
    Seleccion,
    Partida,
    Pausado,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Estado {
    pub fase: Fase,
    /// Nombre del campeón en partida.
    pub campeon: Option<String>,
    /// "KIWI" = ARAM: Caos, "CHERRY" = Arena.
    pub modo: Option<String>,
    pub cartas: Vec<Carta>,
    pub seleccion: Option<Seleccion>,
    /// None = no se pudo leer la configuración del juego.
    pub sin_bordes: Option<bool>,
    pub idioma_cliente: String,
    pub ocr: Option<String>,
    pub version: String,
}

/// Algo con nombre e ícono: un aumento, un ítem, una runa, un hechizo.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct Icono {
    pub id: u32,
    pub nombre: String,
    pub icono: String,
}

/// Página de runas: árbol principal (4 runas, la primera es la clave), secundario (2) y fragmentos (3).
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct Runas {
    pub principal: Icono,
    pub secundaria: Icono,
    pub runas: Vec<Icono>,
    pub secundarias: Vec<Icono>,
    pub fragmentos: Vec<Icono>,
    /// % de victorias y de uso de esta página.
    pub winrate: f64,
    pub uso: f64,
}

/// Build recomendada de un campeón en ARAM según OP.GG (la misma que usa OP.GG para ARAM: Caos).
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct Build {
    pub campeon: u32,
    pub runas: Runas,
    pub hechizos: Vec<Icono>,
    pub inicio: Vec<Icono>,
    pub botas: Vec<Icono>,
    pub nucleo: Vec<Icono>,
    /// Ítems que más se terminan comprando, fuera del núcleo y las botas.
    pub situacionales: Vec<Icono>,
    /// Orden de subida de habilidades, nivel por nivel ("Q", "W"...).
    pub habilidades: Vec<String>,
    /// Qué habilidad maximizar primero.
    pub prioridad: Vec<String>,
    /// Winrate del campeón y partidas analizadas.
    pub winrate: f64,
    pub partidas: u32,
    /// Grieta: la posición de esta build y las posiciones en que se juega el campeón (la más jugada primero).
    pub posicion: Option<String>,
    pub posiciones: Vec<String>,
}

/// Dónde se juega la build: ARAM (también la usa ARAM: Caos) o la Grieta del Invocador (normales y clasificatorias).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, Serialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ModoBuild {
    Aram,
    Grieta,
}

impl ModoBuild {
    /// Modo de build que corresponde a un modo de juego del cliente o de la partida.
    pub fn de_juego(modo: &str) -> ModoBuild {
        if modo == "CLASSIC" {
            ModoBuild::Grieta
        } else {
            ModoBuild::Aram
        }
    }
}

/// Fila de la tier list de aumentos de un campeón.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct AumentoFila {
    pub id: u32,
    pub nombre: String,
    pub icono: String,
    /// "kSilver" | "kGold" | "kPrismatic"
    pub rareza: String,
    pub tier: u8,
    pub perf: f64,
    pub popular: f64,
}
