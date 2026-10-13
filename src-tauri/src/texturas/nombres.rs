// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Nombres en inglés de los ítems, sacados del `en_us.json` del jar del
//! cliente de Minecraft del usuario (no se redistribuye nada).
//!
//! En las versiones modernas (1.13+) el idioma está siempre dentro del jar
//! (`assets/minecraft/lang/en_us.json`). El formato antiguo `.lang` (≤1.12) no
//! se lee: sus claves (`tile.stonebrick.name`) no coinciden con los ids
//! modernos, así que daría nombres equivocados.
//!
//! Si no se encuentra, el mapa sale vacío (el frontend humaniza el id).

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Versión del formato de `nombres_en.json` en la caché.
pub const ESQUEMA_NOMBRES: u32 = 1;

/// Archivo de la caché, junto al atlas de la versión.
pub const ARCHIVO_CACHE: &str = "nombres_en.json";

/// Entrada del idioma dentro del jar.
const LANG_JAR: &str = "assets/minecraft/lang/en_us.json";

/// Contenido de `nombres_en.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheNombres {
    pub esquema: u32,
    pub version: String,
    pub nombres: HashMap<String, String>,
}

/// Nombres de una versión a partir de la ruta de su jar
/// (`<.minecraft>/versions/<v>/<v>.jar`). Vacío si no hay `en_us.json`.
pub fn leer(jar: &Path) -> HashMap<String, String> {
    desde_jar(jar)
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
