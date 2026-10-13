// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Nombres en español de los ítems, sacados del `es_es.json` de la instalación
//! de Minecraft del usuario (no se redistribuye nada).
//!
//! Dónde está ese archivo:
//! 1. Dentro del jar (`assets/minecraft/lang/es_es.json`): solo en versiones
//!    antiguas; las modernas solo traen `en_us.json`.
//! 2. En los assets descargados por el launcher: `versions/<v>/<v>.json` dice
//!    el índice (`assetIndex.id`, o el de la versión de la que hereda con
//!    `inheritsFrom`); `assets/indexes/<índice>.json` da el hash de
//!    `minecraft/lang/es_es.json`, y el archivo está en
//!    `assets/objects/<2 primeros>/<hash>`.
//!
//! Si no se encuentra, el mapa sale vacío (el frontend humaniza el id).

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Versión del formato de `nombres_es.json` en la caché.
pub const ESQUEMA_NOMBRES: u32 = 1;

/// Archivo de la caché, junto al atlas de la versión.
pub const ARCHIVO_CACHE: &str = "nombres_es.json";

/// Entrada del idioma dentro del jar y del índice de assets.
const LANG_JAR: &str = "assets/minecraft/lang/es_es.json";
const LANG_INDICE: &str = "minecraft/lang/es_es.json";

/// Saltos de `inheritsFrom` que se siguen como mucho (evita ciclos).
const MAX_HERENCIA: usize = 4;

/// Contenido de `nombres_es.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheNombres {
    pub esquema: u32,
    pub version: String,
    pub nombres: HashMap<String, String>,
}

/// Nombres de una versión a partir de la ruta de su jar
/// (`<.minecraft>/versions/<v>/<v>.jar`). Vacío si no hay `es_es.json`.
pub fn leer(jar: &Path) -> HashMap<String, String> {
    let bytes = desde_jar(jar).or_else(|| desde_assets(jar));
    bytes
        .and_then(|b| serde_json::from_slice::<Map<String, Value>>(&b).ok())
        .map(|lang| mapa(&lang))
        .unwrap_or_default()
}

/// `minecraft:<x>` → nombre, desde las claves `item.minecraft.<x>` y
/// `block.minecraft.<x>` (si están las dos, gana la de ítem). Las claves con
/// más puntos (`block.minecraft.banner.base.black`…) no son ítems y se saltan.
pub fn mapa(lang: &Map<String, Value>) -> HashMap<String, String> {
    let mut nombres = HashMap::new();
    for prefijo in ["block.minecraft.", "item.minecraft."] {
        for (clave, valor) in lang {
            let (Some(id), Some(texto)) = (clave.strip_prefix(prefijo), valor.as_str()) else {
                continue;
            };
            if id.is_empty() || id.contains('.') || texto.is_empty() {
                continue;
            }
            // Los de ítem van después y pisan a los de bloque.
            nombres.insert(format!("minecraft:{id}"), texto.to_string());
        }
    }
    nombres
}

fn desde_jar(jar: &Path) -> Option<Vec<u8>> {
    let f = fs::File::open(jar).ok()?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(f)).ok()?;
    let mut e = zip.by_name(LANG_JAR).ok()?;
    let mut bytes = Vec::with_capacity(e.size() as usize);
    e.read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

fn desde_assets(jar: &Path) -> Option<Vec<u8>> {
    let dir_version = jar.parent()?;
    let dir_versiones = dir_version.parent()?;
    let raiz = dir_versiones.parent()?;
    let id = dir_version.file_name()?.to_str()?;
    let indice = id_indice(dir_versiones, id)?;
    if !nombre_seguro(&indice) {
        return None;
    }
    let ruta_indice = raiz
        .join("assets")
        .join("indexes")
        .join(format!("{indice}.json"));
    let json: Value = serde_json::from_slice(&fs::read(ruta_indice).ok()?).ok()?;
    let hash = json
        .get("objects")?
        .get(LANG_INDICE)?
        .get("hash")?
        .as_str()?
        .to_ascii_lowercase();
    // Un SHA-1 en hexadecimal: así no se sale nunca de `assets/objects`.
    if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    fs::read(
        raiz.join("assets")
            .join("objects")
            .join(&hash[..2])
            .join(&hash),
    )
    .ok()
}

/// `assetIndex.id` de la versión, o el de la versión de la que hereda.
fn id_indice(dir_versiones: &Path, id: &str) -> Option<String> {
    let mut actual = id.to_string();
    for _ in 0..=MAX_HERENCIA {
        if !nombre_seguro(&actual) {
            return None;
        }
        let ruta = dir_versiones.join(&actual).join(format!("{actual}.json"));
        let json: Value = serde_json::from_slice(&fs::read(ruta).ok()?).ok()?;
        if let Some(indice) = json
            .get("assetIndex")
            .and_then(|a| a.get("id"))
            .and_then(Value::as_str)
        {
            return Some(indice.to_string());
        }
        // Las versiones antiguas solo traen `assets` con el id del índice.
        if let Some(indice) = json.get("assets").and_then(Value::as_str) {
            return Some(indice.to_string());
        }
        actual = json.get("inheritsFrom")?.as_str()?.to_string();
    }
    None
}

/// Un nombre de archivo sin separadores ni `..` (viene de JSON ajeno).
fn nombre_seguro(n: &str) -> bool {
    !n.is_empty() && n != "." && n != ".." && !n.contains(['/', '\\', ':'])
}
