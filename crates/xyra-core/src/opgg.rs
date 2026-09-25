//! Estadísticas de aumentos por campeón desde OP.GG.
use crate::{
    catalogo::Catalogo,
    modelo::{Build, Icono, ModoBuild, Runas},
};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct Dato {
    /// 0 = S … 6 = F.
    pub tier: u8,
    /// Sirve para desempatar entre cartas del mismo tier: más alto es mejor.
    pub perf: f64,
    /// % de partidas en que se elige.
    pub popular: f64,
}

fn get(http: &Client, url: &str) -> Result<Value, String> {
    http.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.json()).map_err(|e| e.to_string())
}

/// ARAM: Caos. OP.GG ya da el tier de cada aumento para el campeón.
pub fn caos(http: &Client, campeon: u32) -> Result<HashMap<u32, Dato>, String> {
    let v = get(http, &format!("https://lol-api-champion.op.gg/api/contents/stats/champions/{campeon}/aram-augments"))?;
    Ok(v["data"]
        .as_array()
        .ok_or("sin datos")?
        .iter()
        .filter_map(|a| {
            let dato = Dato {
                tier: a["tier"].as_u64()? as u8,
                perf: a["performance"].as_f64().unwrap_or(0.0),
                popular: a["popular"].as_f64().unwrap_or(0.0),
            };
            Some((a["id"].as_u64()? as u32, dato))
        })
        .collect())
}

/// Arena. OP.GG da el puesto promedio de cada aumento con el campeón; el tier sale del percentil
/// (10 % mejores = S, siguiente 20 % = A, 30 % = B, 20 % = C, resto = D). Con menos de 20 partidas no se califica.
pub fn arena(http: &Client, campeon: u32) -> Result<HashMap<u32, Dato>, String> {
    let v = get(http, &format!("https://lol-api-champion.op.gg/api/global/champions/arena/{campeon}"))?;
    let mut lista: Vec<(u32, f64, f64)> = v["data"]["augment_group"]
        .as_array()
        .ok_or("sin datos")?
        .iter()
        .flat_map(|g| g["augments"].as_array().cloned().unwrap_or_default())
        .filter_map(|a| {
            let jugadas = a["play"].as_f64()?;
            if jugadas < 20.0 {
                return None;
            }
            Some((a["id"].as_u64()? as u32, a["total_place"].as_f64()? / jugadas, a["pick_rate"].as_f64().unwrap_or(0.0) * 100.0))
        })
        .collect();
    lista.sort_by(|a, b| a.1.total_cmp(&b.1));
    Ok(tiers_por_percentil(&lista))
}

fn tiers_por_percentil(ordenados: &[(u32, f64, f64)]) -> HashMap<u32, Dato> {
    let n = ordenados.len().max(1) as f64;
    ordenados
        .iter()
        .enumerate()
        .map(|(i, &(id, puesto, popular))| {
            let p = i as f64 / n;
            let tier = match p {
                p if p < 0.1 => 0,
                p if p < 0.3 => 1,
                p if p < 0.6 => 2,
                p if p < 0.8 => 3,
                _ => 4,
            };
            (id, Dato { tier, perf: 100.0 - puesto * 10.0, popular })
        })
        .collect()
}

/// Tier list de campeones de ARAM: Caos: id -> (tier 1 = mejor … 5, puesto).
pub fn tiers_campeones(http: &Client) -> Result<HashMap<u32, (u8, u32)>, String> {
    let v = get(http, "https://lol-api-champion.op.gg/api/contents/tiers?type=aram_mayhem")?;
    Ok(v["data"]
        .as_array()
        .ok_or("sin datos")?
        .iter()
        .filter_map(|c| Some((c["id"].as_u64()? as u32, (c["tier"].as_u64()? as u8, c["rank"].as_u64()? as u32))))
        .collect())
}

/// Build de un campeón según OP.GG. En ARAM es la que OP.GG muestra también para ARAM: Caos; en la Grieta es la de
/// la posición pedida o, sin posición, la más jugada. Nombres e íconos salen del catálogo.
pub fn build(http: &Client, campeon: u32, modo: ModoBuild, posicion: Option<&str>, cat: &Catalogo) -> Result<Build, String> {
    const BASE: &str = "https://lol-api-champion.op.gg/api/global/champions";
    if modo == ModoBuild::Aram {
        return Ok(armar_build(&get(http, &format!("{BASE}/aram/{campeon}/none"))?["data"], campeon, cat));
    }
    let pedida = posicion.unwrap_or("mid");
    let v = get(http, &format!("{BASE}/ranked/{campeon}/{pedida}"))?;
    let mut b = armar_build(&v["data"], campeon, cat);
    // sin posición pedida: si el campeón casi no va a "mid", se trae la de su posición principal
    if let (None, Some(principal)) = (posicion, b.posiciones.first().cloned()) {
        if principal != pedida {
            b = armar_build(&get(http, &format!("{BASE}/ranked/{campeon}/{principal}"))?["data"], campeon, cat);
        }
    }
    b.posicion = Some(b.posicion.clone().unwrap_or_else(|| pedida.to_string()));
    Ok(b)
}

fn icono(mapa: &HashMap<u32, (String, String)>, id: u32) -> Icono {
    let (nombre, icono) = mapa.get(&id).cloned().unwrap_or_else(|| (format!("#{id}"), String::new()));
    Icono { id, nombre, icono }
}

fn ids(v: &Value) -> Vec<u32> {
    v.as_array().into_iter().flatten().filter_map(|x| x.as_u64()).map(|x| x as u32).collect()
}

/// La opción más usada de una lista de OP.GG (vienen ordenadas por partidas).
fn primera<'a>(d: &'a Value, clave: &str) -> &'a Value {
    &d[clave][0]
}

pub(crate) fn armar_build(d: &Value, campeon: u32, cat: &Catalogo) -> Build {
    let item = |id| icono(&cat.items, id);
    let runa = |id| icono(&cat.runas, id);
    let r = primera(d, "runes");
    let pct = |v: &Value| {
        let (w, p) = (v["win"].as_f64().unwrap_or(0.0), v["play"].as_f64().unwrap_or(0.0));
        if p > 0.0 { 100.0 * w / p } else { 0.0 }
    };
    let inicio = ids(&primera(d, "starter_items")["ids"]);
    let botas = ids(&primera(d, "boots")["ids"]);
    let nucleo = ids(&primera(d, "core_items")["ids"]);
    let mut situacionales: Vec<u32> = Vec::new();
    for x in d["last_items"].as_array().into_iter().flatten() {
        for id in ids(&x["ids"]) {
            if !nucleo.contains(&id) && !botas.contains(&id) && !situacionales.contains(&id) {
                situacionales.push(id);
            }
        }
    }
    situacionales.truncate(6);
    let texto = |v: &Value| v.as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>();
    let stats = &d["summary"]["average_stats"];
    Build {
        campeon,
        runas: Runas {
            principal: runa(r["primary_page_id"].as_u64().unwrap_or(0) as u32),
            secundaria: runa(r["secondary_page_id"].as_u64().unwrap_or(0) as u32),
            runas: ids(&r["primary_rune_ids"]).into_iter().map(runa).collect(),
            secundarias: ids(&r["secondary_rune_ids"]).into_iter().map(runa).collect(),
            fragmentos: ids(&r["stat_mod_ids"]).into_iter().map(runa).collect(),
            winrate: pct(r),
            uso: r["pick_rate"].as_f64().unwrap_or(0.0) * 100.0,
        },
        hechizos: ids(&primera(d, "summoner_spells")["ids"]).into_iter().map(|id| icono(&cat.hechizos, id)).collect(),
        inicio: inicio.into_iter().map(item).collect(),
        botas: botas.into_iter().map(item).collect(),
        nucleo: nucleo.into_iter().map(item).collect(),
        situacionales: situacionales.into_iter().map(item).collect(),
        habilidades: texto(&primera(d, "skills")["order"]),
        prioridad: texto(&primera(d, "skill_masteries")["ids"]),
        winrate: stats["win_rate"].as_f64().unwrap_or(0.0) * 100.0,
        partidas: stats["play"].as_u64().unwrap_or(0) as u32,
        posicion: None,
        // OP.GG las nombra "TOP", "JUNGLE", "MID", "ADC", "SUPPORT"; ya vienen de la más a la menos jugada
        posiciones: d["summary"]["positions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p["name"].as_str().map(str::to_lowercase))
            .collect(),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn percentiles() {
        let lista: Vec<(u32, f64, f64)> = (0..10).map(|i| (i, 2.0 + i as f64 * 0.3, 1.0)).collect();
        let t = tiers_por_percentil(&lista);
        assert_eq!(t[&0].tier, 0);
        assert_eq!(t[&2].tier, 1);
        assert_eq!(t[&5].tier, 2);
        assert_eq!(t[&9].tier, 4);
        assert!(t[&0].perf > t[&9].perf);
    }
}
