//! Llevar una build al cliente: página de runas y set de ítems para la tienda del juego.
//! Solo se usa cuando el jugador toca "Importar" (lo mismo que hacen Blitz u OP.GG); nunca por su cuenta.
use crate::{
    lol::Lcu,
    modelo::{Build, Icono},
};
use reqwest::{blocking::Client, Method};
use serde_json::{json, Value};

/// Las páginas y sets que crea Xyra empiezan así: al volver a importar se reemplazan en vez de acumularse.
const PREFIJO: &str = "Xyra · ";

fn ids(lista: &[Icono]) -> impl Iterator<Item = u32> + '_ {
    lista.iter().map(|x| x.id)
}

pub fn pagina_runas(b: &Build, campeon: &str) -> Value {
    let r = &b.runas;
    json!({
        "name": format!("{PREFIJO}{campeon}"),
        "primaryStyleId": r.principal.id,
        "subStyleId": r.secundaria.id,
        "selectedPerkIds": ids(&r.runas).chain(ids(&r.secundarias)).chain(ids(&r.fragmentos)).collect::<Vec<_>>(),
        "current": true,
    })
}

/// `bloques`: títulos de Inicio, Botas, Núcleo y Situacionales en el idioma del jugador.
pub fn set_items(b: &Build, campeon: &str, bloques: [&str; 4]) -> Value {
    let bloque = |titulo: &str, lista: &[Icono]| json!({ "type": titulo, "items": lista.iter().map(|i| json!({ "id": i.id.to_string(), "count": 1 })).collect::<Vec<_>>() });
    json!({
        "title": format!("{PREFIJO}{campeon}"),
        "associatedChampions": [b.campeon],
        "associatedMaps": [],
        "blocks": [bloque(bloques[0], &b.inicio), bloque(bloques[1], &b.botas), bloque(bloques[2], &b.nucleo), bloque(bloques[3], &b.situacionales)],
        "map": "any",
        "mode": "any",
        "preferredItemSlots": [],
        "sortrank": 0,
        "startedFrom": "blank",
        "type": "custom",
        // UUID fijo por campeón ("Xyra" en hex): reemplaza al anterior
        "uid": format!("58797261-0000-4000-8000-{:012}", b.campeon),
    })
}

/// Quita el set de Xyra anterior para ese campeón y agrega el nuevo; los sets del jugador no se tocan.
fn agregar_set(sets: &mut Value, nuevo: Value, campeon: u32) {
    if !sets["itemSets"].is_array() {
        sets["itemSets"] = json!([]);
    }
    let lista = sets["itemSets"].as_array_mut().expect("arreglo");
    lista.retain(|s| {
        let mio = s["title"].as_str().is_some_and(|t| t.starts_with(PREFIJO));
        let del_campeon = s["associatedChampions"].as_array().is_some_and(|a| a.iter().any(|c| c.as_u64() == Some(campeon as u64)));
        !(mio && del_campeon)
    });
    lista.push(nuevo);
}

/// Crea la página de runas y la deja activa. Antes borra las páginas que dejó Xyra.
/// Error "sin_espacio" si el jugador no tiene páginas libres.
pub fn runas(lcu: &Lcu, http: &Client, b: &Build, campeon: &str) -> Result<(), String> {
    for p in lcu.get(http, "/lol-perks/v1/pages")?.as_array().into_iter().flatten() {
        let mia = p["name"].as_str().is_some_and(|n| n.starts_with(PREFIJO));
        if mia && p["isDeletable"].as_bool() == Some(true) {
            let _ = lcu.pedir(http, Method::DELETE, &format!("/lol-perks/v1/pages/{}", p["id"]), None);
        }
    }
    lcu.pedir(http, Method::POST, "/lol-perks/v1/pages", Some(&pagina_runas(b, campeon)))
        .map(|_| ())
        .map_err(|e| if e.to_lowercase().contains("max") { "sin_espacio".into() } else { e })
}

/// Guarda el set de ítems: aparece en la tienda del juego al jugar ese campeón.
pub fn items(lcu: &Lcu, http: &Client, b: &Build, campeon: &str, bloques: [&str; 4]) -> Result<(), String> {
    let yo = lcu.get(http, "/lol-summoner/v1/current-summoner")?;
    let id = yo["summonerId"].as_u64().ok_or("sin invocador")?;
    let ruta = format!("/lol-item-sets/v1/item-sets/{id}/sets");
    let mut sets = lcu.get(http, &ruta)?;
    agregar_set(&mut sets, set_items(b, campeon, bloques), b.campeon);
    lcu.pedir(http, Method::PUT, &ruta, Some(&sets)).map(|_| ())
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::{catalogo::Catalogo, opgg};

    fn build() -> Build {
        let v: Value = serde_json::from_str(include_str!("muestras/opgg-aram-ahri.json")).unwrap();
        opgg::armar_build(&v["data"], 103, &Catalogo::default())
    }

    #[test]
    fn build_de_opgg() {
        let b = build();
        let n = |l: &[Icono]| l.iter().map(|x| x.id).collect::<Vec<_>>();
        assert_eq!(n(&b.nucleo), [6655, 4646, 4645]);
        assert_eq!(n(&b.botas), [3020]);
        // lo que ya está en el núcleo o en las botas no se repite en situacionales
        assert_eq!(n(&b.situacionales), [3089]);
        assert_eq!(b.runas.principal.id, 8100);
        assert_eq!(b.runas.runas.len() + b.runas.secundarias.len() + b.runas.fragmentos.len(), 9);
        assert_eq!(b.habilidades.len(), 15);
        // sin catálogo, el nombre queda como el id
        assert_eq!(b.botas[0].nombre, "#3020");
    }

    #[test]
    fn pagina_y_set() {
        let b = build();
        let p = pagina_runas(&b, "Ahri");
        assert_eq!(p["name"], "Xyra · Ahri");
        assert_eq!(p["selectedPerkIds"].as_array().unwrap().len(), 9);

        let mut sets = json!({ "itemSets": [
            { "title": "Xyra · Ahri", "associatedChampions": [103] },
            { "title": "Mi set", "associatedChampions": [103] },
            { "title": "Xyra · Lux", "associatedChampions": [99] },
        ]});
        agregar_set(&mut sets, set_items(&b, "Ahri", ["Inicio", "Botas", "Núcleo", "Situacionales"]), 103);
        let titulos: Vec<&str> = sets["itemSets"].as_array().unwrap().iter().map(|s| s["title"].as_str().unwrap()).collect();
        assert_eq!(titulos, ["Mi set", "Xyra · Lux", "Xyra · Ahri"]);
        assert_eq!(sets["itemSets"][2]["blocks"][2]["items"][0]["id"], "6655");
    }
}
