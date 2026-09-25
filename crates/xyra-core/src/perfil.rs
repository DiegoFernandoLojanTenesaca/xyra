//! Tu cuenta tal como la muestra el cliente (solo lectura): nombre, nivel, región, rango y maestrías.
use crate::lol::{url_recurso, Lcu};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};
use ts_rs::TS;

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Maestria {
    pub id: u32,
    pub nombre: String,
    pub icono: String,
    pub nivel: u32,
    #[ts(type = "number")]
    pub puntos: u64,
}

/// Rango de Solo/Dúo.
#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Rango {
    /// "IRON" … "CHALLENGER" (la interfaz lo traduce y busca su emblema).
    pub liga: String,
    /// "I" … "IV"; vacío en Maestro o más.
    pub division: String,
    pub lp: i64,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Perfil {
    pub nombre: String,
    pub tag: String,
    pub nivel: u32,
    pub icono: String,
    /// "LA1", "NA1"…
    pub region: String,
    /// None si no tiene rango en Solo/Dúo.
    pub rango: Option<Rango>,
    /// Los 5 campeones con más maestría.
    pub maestrias: Vec<Maestria>,
}

/// `campeones`: id -> (nombre, ícono), del catálogo.
pub fn leer(lcu: &Lcu, http: &Client, campeones: &HashMap<u32, (String, String)>) -> Result<Perfil, String> {
    let s = lcu.get(http, "/lol-summoner/v1/current-summoner")?;
    let region = lcu.get(http, "/riotclient/region-locale").ok().and_then(|r| r["region"].as_str().map(String::from));
    let rango = lcu.get(http, "/lol-ranked/v1/current-ranked-stats").ok().and_then(|r| {
        let q = &r["queueMap"]["RANKED_SOLO_5x5"];
        let liga = q["tier"].as_str().filter(|t| !t.is_empty() && *t != "NONE")?;
        // Maestro o más no tiene división ("NA")
        let division = q["division"].as_str().filter(|d| *d != "NA").unwrap_or_default();
        Some(Rango { liga: liga.into(), division: division.into(), lp: q["leaguePoints"].as_i64().unwrap_or(0) })
    });
    let maestrias = lcu
        .get(http, "/lol-champion-mastery/v1/local-player/champion-mastery")
        .ok()
        .and_then(|m| m.as_array().cloned())
        .unwrap_or_default();
    let mut maestrias: Vec<Maestria> = maestrias
        .iter()
        .filter_map(|m| {
            let id = m["championId"].as_u64()? as u32;
            let (nombre, icono) = campeones.get(&id).cloned().unwrap_or_else(|| (format!("#{id}"), String::new()));
            Some(Maestria { id, nombre, icono, nivel: m["championLevel"].as_u64().unwrap_or(0) as u32, puntos: m["championPoints"].as_u64().unwrap_or(0) })
        })
        .collect();
    maestrias.sort_by(|a, b| b.puntos.cmp(&a.puntos));
    maestrias.truncate(5);
    Ok(Perfil {
        nombre: s["gameName"].as_str().or(s["displayName"].as_str()).unwrap_or_default().into(),
        tag: s["tagLine"].as_str().unwrap_or_default().into(),
        nivel: s["summonerLevel"].as_u64().unwrap_or(0) as u32,
        icono: url_recurso(&format!("/lol-game-data/assets/v1/profile-icons/{}.jpg", s["profileIconId"].as_u64().unwrap_or(0))),
        region: region.unwrap_or_default(),
        rango,
        maestrias,
    })
}

/// El último perfil leído se guarda para mostrarlo con el LoL cerrado.
pub fn cargar(ruta: &Path) -> Option<Perfil> {
    serde_json::from_str(&fs::read_to_string(ruta).ok()?).ok()
}

pub fn guardar(ruta: &Path, p: &Perfil) {
    if let Ok(texto) = serde_json::to_string(p) {
        let _ = fs::write(ruta, texto);
    }
}
