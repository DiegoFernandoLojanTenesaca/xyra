//! Nombres e íconos de aumentos y campeones, sacados del cliente en su idioma.
//! Se guarda en disco para mostrar las estadísticas aunque el LoL esté cerrado.
use crate::{cartas::norm, lol};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

/// Campeones jugables. El cliente también lista variantes de modos especiales ("Jade_Ahri", id 60103)
/// que repetirían nombres en las listas.
fn es_campeon(id: i64) -> bool {
    (1..10_000).contains(&id)
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Catalogo {
    /// id -> (nombre, url del ícono)
    pub aumentos: HashMap<u32, (String, String)>,
    /// nombre normalizado -> ids (Arena y Caos repiten nombres)
    pub nombres: HashMap<String, Vec<u32>>,
    /// alias interno ("MonkeyKing") -> id
    pub alias: HashMap<String, u32>,
    /// id -> (nombre, url del ícono)
    pub campeones: HashMap<u32, (String, String)>,
    /// id de aumento -> "kSilver" | "kGold" | "kPrismatic"
    #[serde(default)]
    pub rareza: HashMap<u32, String>,
    /// id -> (nombre, url del ícono) de ítems, runas (y sus árboles) y hechizos de invocador
    #[serde(default)]
    pub items: HashMap<u32, (String, String)>,
    #[serde(default)]
    pub runas: HashMap<u32, (String, String)>,
    #[serde(default)]
    pub hechizos: HashMap<u32, (String, String)>,
}

/// id -> (nombre, ícono) de una lista del cliente (`/lol-game-data/assets/v1/...`). Si falla queda vacía:
/// las builds pierden nombres, pero las cartas siguen funcionando.
fn lista(lcu: &lol::Lcu, http: &Client, ruta: &str) -> HashMap<u32, (String, String)> {
    let v = lcu.get(http, ruta).unwrap_or_default();
    // perkstyles.json viene como {"styles": [...]}
    let arreglo = v.as_array().or_else(|| v["styles"].as_array()).cloned().unwrap_or_default();
    arreglo
        .iter()
        .filter_map(|x| {
            let id = x["id"].as_u64()? as u32;
            Some((id, (x["name"].as_str()?.to_string(), lol::url_recurso(x["iconPath"].as_str().unwrap_or_default()))))
        })
        .collect()
}

impl Catalogo {
    pub fn desde_cliente(lcu: &lol::Lcu, http: &Client) -> Result<Catalogo, String> {
        let mut c = Catalogo::default();
        for a in lcu.get(http, "/lol-game-data/assets/v1/cherry-augments.json")?.as_array().ok_or("aumentos")? {
            let (Some(id), Some(nombre)) = (a["id"].as_u64(), a["nameTRA"].as_str()) else { continue };
            if nombre.is_empty() {
                continue;
            }
            let icono = lol::url_recurso(a["augmentSmallIconPath"].as_str().unwrap_or_default());
            c.nombres.entry(norm(nombre)).or_default().push(id as u32);
            c.aumentos.insert(id as u32, (nombre.to_string(), icono));
            c.rareza.insert(id as u32, a["rarity"].as_str().unwrap_or_default().to_string());
        }
        for ch in lcu.get(http, "/lol-game-data/assets/v1/champion-summary.json")?.as_array().ok_or("campeones")? {
            let (Some(id), Some(nombre), Some(alias)) = (ch["id"].as_i64(), ch["name"].as_str(), ch["alias"].as_str()) else { continue };
            if !es_campeon(id) {
                continue;
            }
            let icono = lol::url_recurso(ch["squarePortraitPath"].as_str().unwrap_or_default());
            c.alias.insert(alias.to_string(), id as u32);
            c.campeones.insert(id as u32, (nombre.to_string(), icono));
        }
        if c.nombres.is_empty() {
            return Err("catálogo vacío".into());
        }
        c.items = lista(lcu, http, "/lol-game-data/assets/v1/items.json");
        c.runas = lista(lcu, http, "/lol-game-data/assets/v1/perks.json");
        c.runas.extend(lista(lcu, http, "/lol-game-data/assets/v1/perkstyles.json"));
        c.hechizos = lista(lcu, http, "/lol-game-data/assets/v1/summoner-spells.json");
        Ok(c)
    }

    pub fn cargar(ruta: &Path) -> Option<Catalogo> {
        let mut c: Catalogo = serde_json::from_str(&fs::read_to_string(ruta).ok()?).ok()?;
        // catálogos guardados antes de filtrar las variantes de modos especiales
        c.campeones.retain(|&id, _| es_campeon(id as i64));
        Some(c)
    }

    pub fn guardar(&self, ruta: &Path) {
        if let Ok(texto) = serde_json::to_string(self) {
            let _ = fs::write(ruta, texto);
        }
    }
}
