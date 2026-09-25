//! Xyra: lógica sin interfaz. Todo lo que no depende de Windows vive aquí para poder reutilizarlo
//! (la app de escritorio hoy; un compañero en el celular mañana).
//!
//! - `modelo`: los datos que ven las interfaces (fuente única: de aquí salen los tipos de TypeScript).
//! - `cartas`: reconocer y calificar las cartas de aumento.
//! - `lol`, `opgg`, `catalogo`: fuentes de datos (cliente del LoL, API de la partida, OP.GG).
//! - `stats`, `perfil`: tus partidas y tu cuenta, solo lectura.
//! - `importar`: llevar runas e ítems al cliente, solo cuando el jugador lo pide.
//! - `juego`: opciones oficiales del juego (rango de ataque, cronómetros del minimapa...) por la API del cliente.
//! - `config`: los ajustes del usuario.

pub mod cartas;
pub mod catalogo;
pub mod config;
pub mod importar;
pub mod juego;
pub mod lol;
pub mod modelo;
pub mod opgg;
pub mod perfil;
pub mod stats;
