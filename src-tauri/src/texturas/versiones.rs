// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Versiones de Minecraft instaladas: `<.minecraft>/versions/<v>/<v>.jar`.

use std::fs;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

/// Entrada que tiene que traer un jar para servir de fuente de texturas.
pub const ENTRADA_PRUEBA: &str = "assets/minecraft/blockstates/stone.json";

/// Versión instalada con un jar utilizable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionMinecraft {
    /// Nombre de la carpeta: `1.21.8`, `26.1-snapshot-11`…
    pub id: String,
    /// Ruta del jar.
    pub ruta: String,
    /// Versión estable vanilla: `type` = `release` en su JSON y sin
    /// `inheritsFrom` (las de OptiFine, Fabric o Forge heredan de otra y se
    /// ordenan detrás aunque se declaren `release`).
    pub es_release: bool,
    /// `releaseTime` del JSON de la versión (para ordenar; no se serializa).
    #[serde(skip)]
    pub fecha: String,
}

/// Carpetas `.minecraft` habituales que existen en esta máquina: la de
/// Windows (`%APPDATA%\.minecraft`), la de Linux (`~/.minecraft`) y la de
/// macOS (`~/Library/Application Support/minecraft`).
pub fn carpetas_por_defecto() -> Vec<PathBuf> {
    let mut c = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        c.push(PathBuf::from(appdata).join(".minecraft"));
    }
    let casa = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if let Some(casa) = casa {
        let casa = PathBuf::from(casa);
        c.push(casa.join(".minecraft"));
        c.push(casa.join("Library/Application Support/minecraft"));
    }
    c.retain(|p| p.join("versions").is_dir());
    c.dedup();
    c
}

/// Versiones con jar utilizable en estas carpetas `.minecraft`, sin repetir
/// id (gana la primera carpeta). Orden: releases vanilla primero y, dentro de
/// cada grupo, de la más reciente a la más antigua.
pub fn listar(carpetas: &[PathBuf]) -> Vec<VersionMinecraft> {
    let mut v: Vec<VersionMinecraft> = Vec::new();
    for carpeta in carpetas {
        let Ok(dir) = fs::read_dir(carpeta.join("versions")) else {
            continue;
        };
        for e in dir.flatten() {
            let Some(id) = e.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if v.iter().any(|x| x.id == id) {
                continue;
            }
            if let Some(ver) = leer_version(&e.path(), &id) {
                v.push(ver);
            }
        }
    }
    v.sort_by(|a, b| {
        b.es_release
            .cmp(&a.es_release)
            .then_with(|| b.fecha.cmp(&a.fecha))
            .then_with(|| b.id.cmp(&a.id))
    });
    v
}

/// Una carpeta de versión: necesita `<id>.jar` con los assets dentro (hay
/// perfiles, como algunos de OptiFine o Fabric, que solo traen el JSON).
fn leer_version(dir: &Path, id: &str) -> Option<VersionMinecraft> {
    let jar = dir.join(format!("{id}.jar"));
    if !jar.is_file() || !tiene_assets(&jar) {
        return None;
    }
    let json: Value = fs::read(dir.join(format!("{id}.json")))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null);
    let texto = |k: &str| {
        json.get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    Some(VersionMinecraft {
        id: id.to_string(),
        ruta: jar.to_string_lossy().into_owned(),
        es_release: texto("type") == "release" && json.get("inheritsFrom").is_none(),
        fecha: texto("releaseTime"),
    })
}

/// ¿El jar es un zip con los blockstates vanilla? Solo lee el índice del zip.
pub fn tiene_assets(jar: &Path) -> bool {
    fs::File::open(jar)
        .ok()
        .is_some_and(|f| zip_tiene(f, ENTRADA_PRUEBA))
}

fn zip_tiene<R: Read + Seek>(r: R, entrada: &str) -> bool {
    zip::ZipArchive::new(r)
        .ok()
        .is_some_and(|z| z.index_for_name(entrada).is_some())
}
