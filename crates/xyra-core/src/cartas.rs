//! Reconocer las cartas en el texto del OCR y calificarlas con los datos de OP.GG.
use crate::opgg::Dato;
use serde::Serialize;
use std::collections::HashMap;
use ts_rs::TS;
use unicode_normalization::UnicodeNormalization;

/// Línea de texto leída por el OCR, en píxeles de la zona capturada.
pub struct Linea {
    pub texto: String,
    pub x0: f64,
    pub x1: f64,
    /// Pie de la línea.
    pub y1: f64,
}

/// Minúsculas, sin tildes ni signos: "¡Tirador Mágico!" -> "tirador magico".
pub fn norm(s: &str) -> String {
    let t: String = s.to_lowercase().nfkd().filter(|c| c.is_alphanumeric() || *c == ' ').collect();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Carta encontrada en pantalla. `ids` tiene varios elementos cuando Arena y Caos repiten el nombre.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidata {
    pub ids: Vec<u32>,
    /// Centro del nombre de la carta, en píxeles de pantalla.
    pub x: f64,
    /// Pie del nombre de la carta.
    pub y: f64,
}

/// Líneas del OCR que son nombres de aumentos.
pub fn buscar(lineas: &[Linea], nombres: &HashMap<String, Vec<u32>>, dx: f64, dy: f64) -> Vec<Candidata> {
    lineas
        .iter()
        .filter_map(|l| {
            let t = norm(&l.texto);
            if t.chars().count() < 4 {
                return None;
            }
            let (nombre, parecido) = nombres
                .keys()
                .map(|k| (k, strsim::normalized_levenshtein(&t, k)))
                .max_by(|a, b| a.1.total_cmp(&b.1))?;
            (parecido >= 0.8).then(|| Candidata { ids: nombres[nombre].clone(), x: dx + (l.x0 + l.x1) / 2.0, y: dy + l.y1 })
        })
        .collect()
}

/// Las cartas salen en una sola fila: se queda con el grupo más grande a la misma altura
/// (así ignora nombres sueltos en la tienda, el chat, etc.). Menos de 2 = no hay cartas.
pub fn fila(cartas: &[Candidata], alto: f64) -> Vec<Candidata> {
    let mut ordenadas = cartas.to_vec();
    ordenadas.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut mejor: Vec<Candidata> = Vec::new();
    for c in cartas {
        let mut grupo: Vec<Candidata> = Vec::new();
        for o in &ordenadas {
            if (o.y - c.y).abs() < alto * 0.03 && !grupo.iter().any(|g| g.ids == o.ids) {
                grupo.push(o.clone());
            }
        }
        if grupo.len() > mejor.len() {
            mejor = grupo;
        }
    }
    if mejor.len() >= 2 {
        mejor
    } else {
        Vec::new()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Carta {
    pub id: u32,
    pub nombre: String,
    pub icono: String,
    /// 0 = S … 6 = F; None = OP.GG no tiene partidas de ese aumento con el campeón.
    pub tier: Option<u8>,
    pub perf: f64,
    /// 1 = la mejor de las ofrecidas.
    pub puesto: u32,
    pub mejor: bool,
    /// Mala o sin datos habiendo una buena en la mesa: conviene usar su botón de cambio.
    pub cambiar: bool,
    pub x: f64,
    pub y: f64,
}

/// Califica las cartas: menor tier y, a igualdad, mayor rendimiento; las que no tienen datos van al final.
pub fn calificar(cartas: &[Candidata], datos: &HashMap<u32, Dato>, info: &HashMap<u32, (String, String)>) -> Vec<Carta> {
    let mut out: Vec<Carta> = cartas
        .iter()
        .map(|c| {
            let id = c.ids.iter().copied().find(|i| datos.contains_key(i)).unwrap_or(c.ids[0]);
            let dato = datos.get(&id);
            let (nombre, icono) = info.get(&id).cloned().unwrap_or_default();
            Carta {
                id,
                nombre,
                icono,
                tier: dato.map(|d| d.tier),
                perf: dato.map_or(0.0, |d| d.perf),
                puesto: 0,
                mejor: false,
                cambiar: false,
                x: c.x,
                y: c.y,
            }
        })
        .collect();
    let mut orden: Vec<usize> = (0..out.len()).collect();
    orden.sort_by(|&a, &b| {
        let (x, y) = (&out[a], &out[b]);
        (x.tier.is_none(), x.tier.unwrap_or(0)).cmp(&(y.tier.is_none(), y.tier.unwrap_or(0))).then(y.perf.total_cmp(&x.perf))
    });
    for (n, &i) in orden.iter().enumerate() {
        out[i].puesto = n as u32 + 1;
    }
    let hay_buena = out.iter().any(|c| c.tier.is_some_and(|t| t <= 2));
    for c in &mut out {
        c.mejor = c.puesto == 1 && c.tier.is_some();
        c.cambiar = hay_buena && c.tier.is_none_or(|t| t >= 4);
    }
    out
}

/// Qué hacer con las etiquetas tras una lectura de la pantalla.
#[derive(Debug, PartialEq)]
pub enum Accion {
    Nada,
    Mostrar(Vec<Candidata>),
    Ocultar,
}

/// Decide cuándo mostrar y ocultar las etiquetas, para que acompañen a las cartas en el momento justo:
/// - al aparecer, las cartas entran con una animación: se muestran recién cuando dos lecturas seguidas
///   las ven en el mismo lugar (así la etiqueta no sale antes ni corrida);
/// - al elegir una, se ocultan tras dos lecturas sin cartas (una sola puede ser un fallo del OCR).
#[derive(Default)]
pub struct Detector {
    /// Lo que se ve ahora en pantalla (ids por carta).
    visto: Vec<Vec<u32>>,
    /// Lectura anterior aún sin confirmar.
    pendiente: Option<Vec<Candidata>>,
    vacios: u8,
}

/// Distancia en píxeles para considerar que una carta no se movió entre lecturas.
const QUIETA: f64 = 8.0;

fn mismas(a: &[Candidata], b: &[Candidata]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.ids == b.ids && (a.x - b.x).abs() < QUIETA && (a.y - b.y).abs() < QUIETA)
}

impl Detector {
    pub fn leer(&mut self, cartas: Vec<Candidata>) -> Accion {
        if cartas.is_empty() {
            self.pendiente = None;
            if self.visto.is_empty() {
                return Accion::Nada;
            }
            self.vacios += 1;
            if self.vacios < 2 {
                return Accion::Nada;
            }
            self.visto.clear();
            return Accion::Ocultar;
        }
        self.vacios = 0;
        let ids: Vec<Vec<u32>> = cartas.iter().map(|c| c.ids.clone()).collect();
        if ids == self.visto {
            return Accion::Nada;
        }
        if self.pendiente.as_deref().is_some_and(|p| mismas(p, &cartas)) {
            self.pendiente = None;
            self.visto = ids;
            return Accion::Mostrar(cartas);
        }
        self.pendiente = Some(cartas);
        Accion::Nada
    }

    /// ¿Hay cartas en pantalla o por confirmar? Mientras tanto conviene leer más seguido.
    pub fn atento(&self) -> bool {
        !self.visto.is_empty() || self.pendiente.is_some()
    }

    /// Olvida lo visto (se ocultó la capa por otro motivo).
    pub fn reiniciar(&mut self) {
        *self = Detector::default();
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn linea(texto: &str, x: f64, y: f64) -> Linea {
        Linea { texto: texto.into(), x0: x, x1: x + 120.0, y1: y + 20.0 }
    }

    #[test]
    fn normaliza() {
        assert_eq!(norm("¡Tirador Mágico!"), "tirador magico");
        assert_eq!(norm("  Interés   en Llamas "), "interes en llamas");
    }

    #[test]
    fn reconoce_filtra_y_califica() {
        // "Tirador Mágico" existe en Arena (129) y en Caos (1129): debe elegirse el que tiene datos
        let nombres: HashMap<String, Vec<u32>> = [
            (norm("¡BONK!"), vec![2111]),
            (norm("Tirador Mágico"), vec![129, 1129]),
            (norm("Golpe Místico"), vec![1058]),
        ]
        .into();
        let ocr = [
            linea("BONK!", 100.0, 480.0),
            linea("TIRADOR MAGICO", 500.0, 482.0),
            linea("Golpe Mistico", 900.0, 481.0),
            linea("Golpe Místico", 300.0, 900.0), // fuera de la fila (tienda, chat...)
            linea("Tienda", 50.0, 480.0),
        ];
        let cartas = fila(&buscar(&ocr, &nombres, 0.0, 0.0), 1200.0);
        assert_eq!(cartas.iter().map(|c| c.ids.clone()).collect::<Vec<_>>(), vec![vec![2111], vec![129, 1129], vec![1058]]);

        let datos: HashMap<u32, Dato> =
            [(2111, Dato { tier: 1, perf: 91.0, popular: 1.0 }), (1129, Dato { tier: 0, perf: 92.0, popular: 1.0 })].into();
        let c = calificar(&cartas, &datos, &HashMap::new());
        let resumen: Vec<_> = c.iter().map(|c| (c.id, c.tier, c.puesto, c.mejor, c.cambiar)).collect();
        assert_eq!(resumen, vec![(2111, Some(1), 2, false, false), (1129, Some(0), 1, true, false), (1058, None, 3, false, true)]);
    }

    #[test]
    fn sin_fila_no_hay_cartas() {
        let nombres: HashMap<String, Vec<u32>> = [(norm("Golpe Místico"), vec![1058])].into();
        assert!(fila(&buscar(&[linea("Golpe Mistico", 0.0, 0.0)], &nombres, 0.0, 0.0), 1200.0).is_empty());
    }

    fn carta(id: u32, x: f64) -> Candidata {
        Candidata { ids: vec![id], x, y: 500.0 }
    }

    #[test]
    fn detector_espera_a_que_se_quieten_y_oculta_sin_demora() {
        let mut d = Detector::default();
        // entrando con animación: se mueven, no se muestra
        assert_eq!(d.leer(vec![carta(1, 380.0), carta(2, 900.0)]), Accion::Nada);
        assert_eq!(d.leer(vec![carta(1, 400.0), carta(2, 920.0)]), Accion::Nada);
        // quietas en dos lecturas: se muestran
        let quietas = vec![carta(1, 402.0), carta(2, 921.0)];
        assert_eq!(d.leer(quietas.clone()), Accion::Mostrar(quietas.clone()));
        assert_eq!(d.leer(quietas.clone()), Accion::Nada);
        // un fallo suelto del OCR no las oculta; dos seguidos sí
        assert_eq!(d.leer(vec![]), Accion::Nada);
        assert_eq!(d.leer(quietas.clone()), Accion::Nada);
        assert_eq!(d.leer(vec![]), Accion::Nada);
        assert_eq!(d.leer(vec![]), Accion::Ocultar);
        assert!(!d.atento());
        // cambiar una carta (reroll) se confirma y se vuelve a dibujar
        d.leer(quietas.clone());
        d.leer(quietas);
        let otra = vec![carta(1, 402.0), carta(3, 921.0)];
        assert_eq!(d.leer(otra.clone()), Accion::Nada);
        assert_eq!(d.leer(otra.clone()), Accion::Mostrar(otra));
    }
}
