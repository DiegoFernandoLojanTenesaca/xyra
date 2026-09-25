use super::Shared;
use std::sync::Arc;
use xyra_core::{catalog::Catalog, errors::AppError, league::Lcu};

#[derive(Default)]
pub struct CatalogLoader {
    loaded: bool,
}

impl CatalogLoader {
    pub fn reset(&mut self) {
        self.loaded = false;
    }

    pub fn load(&mut self, lcu: &Lcu, shared: &Shared) -> bool {
        if self.loaded {
            return false;
        }
        match Catalog::read(lcu) {
            Ok(catalog) => {
                self.loaded = true;
                if let Err(e) = shared.storage.save_catalog(&catalog) {
                    shared.log_error("save catalog", e);
                }
                *shared.catalog.write().unwrap() = Arc::new(catalog);
                true
            }
            Err(AppError::EmptyCatalog | AppError::Client(_)) => false,
            Err(e) => {
                shared.log_error("catalog", e);
                false
            }
        }
    }
}
