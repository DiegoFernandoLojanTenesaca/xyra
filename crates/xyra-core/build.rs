use serde_json::Value;
use std::{env, fmt::Write, fs, path::Path};

const TOKENS: &str = "../../design/tokens.json";
const MAX_ALIAS_DEPTH: usize = 8;
const HEX_DIGITS: usize = 6;

fn resolve<'a>(root: &'a Value, value: &'a str, depth: usize) -> &'a str {
    let Some(path) = value.strip_prefix('{').and_then(|v| v.strip_suffix('}')) else { return value };
    assert!(depth < MAX_ALIAS_DEPTH, "token alias loop at {path}");
    let target = path.split('.').try_fold(root, |node, part| node.get(part)).and_then(Value::as_str);
    resolve(root, target.unwrap_or_else(|| panic!("token alias {path} points to no token")), depth + 1)
}

fn constant_name(path: &[String]) -> String {
    path.iter()
        .map(|part| {
            part.chars().fold(String::new(), |mut name, c| {
                if c.is_uppercase() {
                    name.push('_');
                }
                name.push(c.to_ascii_uppercase());
                name
            })
        })
        .collect::<Vec<_>>()
        .join("_")
}

fn emit(root: &Value, node: &Value, path: &mut Vec<String>, out: &mut String) {
    for (key, value) in node.as_object().expect("token group") {
        path.push(key.clone());
        match value {
            Value::Object(_) => emit(root, value, path, out),
            Value::String(text) => {
                let (name, resolved) = (constant_name(path), resolve(root, text, 0));
                match resolved.strip_prefix('#') {
                    Some(hex) => {
                        assert!(hex.len() == HEX_DIGITS && hex.chars().all(|c| c.is_ascii_hexdigit()), "token {name} is not #rrggbb");
                        writeln!(out, "pub const {name}: u32 = 0x{hex};")
                    }
                    None => writeln!(out, "pub const {name}: &str = {resolved:?};"),
                }
                .expect("write token");
            }
            _ => panic!("token {} must be a string", path.join(".")),
        }
        path.pop();
    }
}

fn main() {
    println!("cargo:rerun-if-changed={TOKENS}");
    let root: Value = serde_json::from_str(&fs::read_to_string(TOKENS).expect("design/tokens.json")).expect("valid tokens.json");
    let mut out = String::new();
    emit(&root, &root, &mut Vec::new(), &mut out);
    fs::write(Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("tokens.rs"), out).expect("write tokens.rs");
}
