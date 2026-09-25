//! Ajustes del usuario, guardados como JSON en la carpeta de configuración de la app.
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use ts_rs::TS;

#[derive(Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(default)]
pub struct Config {
    pub estilo: String,
    pub voz: bool,
    pub arena: bool,
    pub pausado: bool,
    /// "auto" (el del cliente del LoL), "es" o "en".
    pub idioma: String,
    /// Calibración: desplazamiento vertical de las marcas, en píxeles a 1200 de alto.
    pub offset_y: f64,
    /// Calibración: tamaño de las marcas.
    pub escala: f64,
    /// Guarda una captura cada vez que detecta cartas (para reportar fallos o calibrar).
    pub grabar: bool,
    pub autostart: bool,
    /// Deja el juego en "Sin bordes" solo (si alguien lo cambia, Xyra lo vuelve a poner con el juego cerrado).
    pub auto_bordes: bool,
    /// Importa las runas en cuanto te toca un campeón en la selección.
    pub auto_runas: bool,
    /// Cierra la ventana de Xyra al empezar la partida (queda en la bandeja, sin gastar memoria).
    pub cerrar_en_partida: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            estilo: "placa".into(),
            voz: false,
            arena: true,
            pausado: false,
            idioma: "auto".into(),
            offset_y: 0.0,
            escala: 1.0,
            grabar: false,
            autostart: true,
            auto_bordes: true,
            auto_runas: false,
            cerrar_en_partida: false,
        }
    }
}

impl Config {
    pub fn cargar(ruta: &Path) -> Config {
        fs::read_to_string(ruta).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn guardar(&self, ruta: &Path) {
        if let Some(carpeta) = ruta.parent() {
            let _ = fs::create_dir_all(carpeta);
        }
        if let Ok(texto) = serde_json::to_string_pretty(self) {
            let _ = fs::write(ruta, texto);
        }
    }

    /// Idioma de las etiquetas ya resuelto: "es" o "en".
    pub fn idioma_efectivo(&self, cliente: &str) -> &'static str {
        let idioma = if self.idioma == "auto" { cliente } else { &self.idioma };
        if idioma.to_lowercase().starts_with("es") {
            "es"
        } else {
            "en"
        }
    }
}
