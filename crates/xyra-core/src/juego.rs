//! Opciones oficiales del juego (las mismas del menú de Opciones del LoL), cambiadas por la API del cliente
//! (`/lol-game-settings`). Solo las de esta lista: nada que el juego no ofrezca ya.
use crate::lol::Lcu;
use reqwest::{blocking::Client, Method};
use serde::Serialize;
use serde_json::{json, Value};
use ts_rs::TS;

/// (sección, clave) de las opciones que Xyra deja cambiar.
pub const PERMITIDAS: [(&str, &str); 6] = [
    ("General", "WindowMode"),
    ("HUD", "ShowAttackRadius"),
    ("HUD", "MinimapEnableAllTimers"),
    ("General", "ShowTurretRangeIndicators"),
    ("HUD", "MinimapScale"),
    ("HUD", "FlipMiniMap"),
];

const RUTA: &str = "/lol-game-settings/v1/game-settings";

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct AjusteJuego {
    pub seccion: String,
    pub clave: String,
    /// true/false, o un número (WindowMode: 0 pantalla completa, 1 ventana, 2 sin bordes; MinimapScale).
    #[ts(type = "boolean | number")]
    pub valor: Value,
}

/// Valor actual de cada opción permitida. Error si el cliente está cerrado.
pub fn leer(lcu: &Lcu, http: &Client) -> Result<Vec<AjusteJuego>, String> {
    let v = lcu.get(http, RUTA)?;
    Ok(PERMITIDAS
        .iter()
        .filter(|(s, c)| !v[*s][*c].is_null())
        .map(|(s, c)| AjusteJuego { seccion: s.to_string(), clave: c.to_string(), valor: v[*s][*c].clone() })
        .collect())
}

pub fn cambiar(lcu: &Lcu, http: &Client, seccion: &str, clave: &str, valor: Value) -> Result<(), String> {
    if !PERMITIDAS.contains(&(seccion, clave)) {
        return Err("opción no permitida".into());
    }
    lcu.pedir(http, Method::PATCH, RUTA, Some(&json!({ seccion: { clave: valor } }))).map(|_| ())
}
