use include_dir::{Dir, include_dir};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, sync::OnceLock};

static LOCALES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../locales");
pub const BASE_LANGUAGE: &str = "en";

/// "language/namespace" -> parsed namespace file.
fn namespaces() -> &'static HashMap<String, Value> {
    static CACHE: OnceLock<HashMap<String, Value>> = OnceLock::new();
    CACHE.get_or_init(|| {
        LOCALES
            .dirs()
            .flat_map(|language| language.files())
            .filter_map(|file| {
                let path = file.path();
                let key = format!("{}/{}", path.parent()?.to_str()?, path.file_stem()?.to_str()?);
                Some((key, serde_json::from_slice(file.contents()).ok()?))
            })
            .collect()
    })
}

pub fn languages() -> impl Iterator<Item = &'static str> {
    LOCALES.dirs().filter_map(|d| d.path().file_name()?.to_str())
}

/// Available language for a locale code ("es_MX" -> "es"), or the base language.
pub fn resolve(locale: &str) -> &'static str {
    let language = locale.split(['_', '-']).next().unwrap_or_default().to_lowercase();
    languages().find(|l| *l == language).unwrap_or(BASE_LANGUAGE)
}

fn lookup(language: &str, key: &str) -> Option<String> {
    let (namespace, path) = key.split_once(':')?;
    let mut node = namespaces().get(&format!("{language}/{namespace}"))?;
    for part in path.split('.') {
        node = node.get(part)?;
    }
    node.as_str().map(String::from)
}

/// `key` is "namespace:path.to.key"; falls back to the base language, then to the key itself.
pub fn t(language: &str, key: &str) -> String {
    lookup(language, key).or_else(|| lookup(BASE_LANGUAGE, key)).unwrap_or_else(|| key.to_string())
}

pub fn t_with(language: &str, key: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(t(language, key), |text, (name, value)| text.replace(&format!("{{{{{name}}}}}"), value))
}

/// Serialized name of an enum variant, which is also its i18n key.
pub fn variant_key(variant: impl Serialize) -> String {
    serde_json::to_value(variant).ok().and_then(|value| value.as_str().map(String::from)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_locales_and_falls_back() {
        assert_eq!(resolve("es_MX"), "es");
        assert_eq!(resolve("xx_YY"), BASE_LANGUAGE);
        assert_eq!(t("es", "stats:victory"), "Victoria");
        assert_eq!(t("xx", "stats:victory"), "Victory");
        assert_eq!(t("es", "missing:key"), "missing:key");
        assert_eq!(t_with("en", "overlay:forChampion", &[("champion", "Brand")]), "for Brand");
        assert_eq!(variant_key(crate::model::GameMode::SummonersRift), "summonersRift");
    }

    #[test]
    fn every_language_has_the_base_keys() {
        fn keys(value: &Value, prefix: &str, out: &mut Vec<String>) {
            if let Some(map) = value.as_object() {
                for (k, v) in map {
                    keys(v, &format!("{prefix}{k}."), out);
                }
            } else {
                out.push(prefix.trim_end_matches('.').to_string());
            }
        }
        for (name, base) in namespaces().iter().filter(|(n, _)| n.starts_with("en/")) {
            let mut expected = Vec::new();
            keys(base, "", &mut expected);
            for language in languages().filter(|l| *l != BASE_LANGUAGE) {
                let other = &namespaces()[&name.replacen("en/", &format!("{language}/"), 1)];
                let mut found = Vec::new();
                keys(other, "", &mut found);
                for key in &expected {
                    assert!(found.contains(key), "{language} is missing {name}:{key}");
                }
            }
        }
    }
}
