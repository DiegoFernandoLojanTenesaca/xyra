pub use reqwest::blocking::Client;
use std::{sync::Once, time::Duration};

const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = concat!("xyra/", env!("CARGO_PKG_VERSION"));

/// HTTP client for internet services (OP.GG, GitHub), verified against the system's certificate authorities.
pub fn client() -> Client {
    static CRYPTO: Once = Once::new();
    CRYPTO.call_once(|| rustls::crypto::ring::default_provider().install_default().expect("first TLS crypto provider"));
    Client::builder().timeout(HTTP_TIMEOUT).user_agent(USER_AGENT).build().expect("internet HTTP client")
}
