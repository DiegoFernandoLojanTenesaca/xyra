//! Tus estadísticas: aumentos elegidos y resultado de cada partida de Caos o Arena, tomados del historial del cliente.
use crate::{catalogo::Catalogo, lol::Lcu, modelo::Icono};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::HashMap, fs, path::Path};
use ts_rs::TS;

#[derive(Clone, Serialize, Deserialize)]
pub struct Guardada {
    pub game_id: u64,
    pub fecha: String,
    pub campeon: u32,
    pub modo: String,
    pub victoria: bool,
    pub aumentos: Vec<u32>,
}

pub fn cargar(ruta: &Path) -> Vec<Guardada> {
    fs::read_to_string(ruta).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn guardar(ruta: &Path, lista: &[Guardada]) {
    if let Ok(texto) = serde_json::to_string(lista) {
        let _ = fs::write(ruta, texto);
    }
}

/// Agrega las partidas de Caos y Arena del historial reciente que falten. Devuelve cuántas se agregaron.
pub fn importar(lcu: &Lcu, http: &Client, lista: &mut Vec<Guardada>) -> Result<usize, String> {
    let v = lcu.get(http, "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=19")?;
    let mut nuevas = 0;
    for g in v["games"]["games"].as_array().into_iter().flatten() {
        let modo = g["gameMode"].as_str().unwrap_or_default();
        let Some(id) = g["gameId"].as_u64() else { continue };
        if !(modo == "KIWI" || modo == "CHERRY") || lista.iter().any(|x| x.game_id == id) {
            continue;
        }
        // en este historial "participants" trae solo al jugador actual
        let p = &g["participants"][0];
        let st = &p["stats"];
        let Some(campeon) = p["championId"].as_u64() else { continue };
        lista.push(Guardada {
            game_id: id,
            fecha: g["gameCreationDate"].as_str().unwrap_or_default().chars().take(16).collect(),
            campeon: campeon as u32,
            modo: modo.into(),
            victoria: st["win"].as_bool().unwrap_or(false),
            aumentos: (1..=6).filter_map(|i| st[format!("playerAugment{i}")].as_u64()).filter(|&a| a > 0).map(|a| a as u32).collect(),
        });
        nuevas += 1;
    }
    lista.sort_by_key(|x| Reverse(x.game_id));
    Ok(nuevas)
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct Fila {
    pub id: u32,
    pub nombre: String,
    pub icono: String,
    pub partidas: u32,
    pub victorias: u32,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct Reciente {
    #[ts(type = "number")]
    pub game_id: u64,
    pub fecha: String,
    pub campeon: String,
    pub icono: String,
    pub modo: String,
    pub victoria: bool,
    pub aumentos: Vec<Icono>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct Resumen {
    pub partidas: u32,
    pub victorias: u32,
    pub campeones: Vec<Fila>,
    pub aumentos: Vec<Fila>,
    pub recientes: Vec<Reciente>,
}

fn contar(filas: HashMap<u32, (u32, u32)>, nombres: &HashMap<u32, (String, String)>) -> Vec<Fila> {
    filas
        .into_iter()
        .map(|(id, (partidas, victorias))| {
            let (nombre, icono) = nombres.get(&id).cloned().unwrap_or_else(|| (format!("#{id}"), String::new()));
            Fila { id, nombre, icono, partidas, victorias }
        })
        .collect()
}

pub fn resumen(lista: &[Guardada], cat: &Catalogo) -> Resumen {
    let mut campeones: HashMap<u32, (u32, u32)> = HashMap::new();
    let mut aumentos: HashMap<u32, (u32, u32)> = HashMap::new();
    for g in lista {
        let e = campeones.entry(g.campeon).or_default();
        e.0 += 1;
        e.1 += g.victoria as u32;
        for &a in &g.aumentos {
            let e = aumentos.entry(a).or_default();
            e.0 += 1;
            e.1 += g.victoria as u32;
        }
    }
    let mut campeones = contar(campeones, &cat.campeones);
    campeones.sort_by_key(|f| (Reverse(f.partidas), Reverse(f.victorias)));
    // mejores aumentos: con 2 o más partidas, por % de victorias y luego por partidas
    let mut aumentos: Vec<Fila> = contar(aumentos, &cat.aumentos).into_iter().filter(|f| f.partidas >= 2).collect();
    aumentos.sort_by(|a, b| {
        (b.victorias as f64 / b.partidas as f64).total_cmp(&(a.victorias as f64 / a.partidas as f64)).then(b.partidas.cmp(&a.partidas))
    });
    aumentos.truncate(15);
    let icono = |id: &u32, m: &HashMap<u32, (String, String)>| m.get(id).cloned().unwrap_or_else(|| (format!("#{id}"), String::new()));
    Resumen {
        partidas: lista.len() as u32,
        victorias: lista.iter().filter(|g| g.victoria).count() as u32,
        campeones,
        aumentos,
        recientes: lista
            .iter()
            .take(10)
            .map(|g| {
                let (campeon, ic) = icono(&g.campeon, &cat.campeones);
                Reciente {
                    game_id: g.game_id,
                    fecha: g.fecha.replace('T', " "),
                    campeon,
                    icono: ic,
                    modo: g.modo.clone(),
                    victoria: g.victoria,
                    aumentos: g
                        .aumentos
                        .iter()
                        .map(|a| {
                            let (nombre, icono) = icono(a, &cat.aumentos);
                            Icono { id: *a, nombre, icono }
                        })
                        .collect(),
                }
            })
            .collect(),
    }
}

/// Tus partidas en CSV (se abre con Excel): fecha, modo, campeón, resultado y aumentos.
pub fn csv(lista: &[Guardada], cat: &Catalogo) -> String {
    let nombre = |m: &HashMap<u32, (String, String)>, id: &u32| m.get(id).map_or_else(|| format!("#{id}"), |x| x.0.clone());
    // comillas dobles escapadas; los nombres pueden llevar comas
    let campo = |s: String| format!("\"{}\"", s.replace('"', "\"\""));
    let mut texto = String::from("fecha,modo,campeon,resultado,aumentos\r\n");
    for g in lista {
        let modo = if g.modo == "CHERRY" { "Arena" } else { "ARAM: Caos" };
        let aumentos: Vec<String> = g.aumentos.iter().map(|a| nombre(&cat.aumentos, a)).collect();
        let fila = [g.fecha.replace('T', " "), modo.into(), nombre(&cat.campeones, &g.campeon), (if g.victoria { "victoria" } else { "derrota" }).into(), aumentos.join(" | ")];
        texto += &fila.map(campo).join(",");
        texto += "\r\n";
    }
    texto
}

#[cfg(test)]
mod pruebas_csv {
    use super::*;

    #[test]
    fn csv_escapa_comillas_y_nombra() {
        let mut cat = Catalogo::default();
        cat.campeones.insert(1, ("Annie".into(), String::new()));
        cat.aumentos.insert(7, ("Ojo \"de\" halcón, raro".into(), String::new()));
        let g = Guardada { game_id: 1, fecha: "2026-09-25T10:00".into(), campeon: 1, modo: "KIWI".into(), victoria: true, aumentos: vec![7, 9] };
        let t = csv(&[g], &cat);
        assert_eq!(t.lines().nth(1).unwrap(), r#""2026-09-25 10:00","ARAM: Caos","Annie","victoria","Ojo ""de"" halcón, raro | #9""#);
    }
}
