// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Lectura del jar de Minecraft y montaje del atlas de texturas de bloque.
//!
//! El atlas lleva **todas** las texturas de `textures/block/` (unas 1100 en
//! 1.21), no solo las del schematic abierto: así se genera una sola vez por
//! versión y cualquier schematic lo reutiliza.
//!
//! Cada celda mide [`TAM_CELDA`] píxeles: la textura de [`TAM_TEXTURA`] en el
//! centro y [`RELLENO`] píxeles a cada lado con la misma textura repetida
//! (como `RepeatWrapping`). Así el filtrado bilineal y los mipmaps hasta el
//! nivel `log2(RELLENO)` = 3 no mezclan celdas vecinas y el borde de `fract`
//! empalma con el lado opuesto de la textura, que es lo que toca en los quads
//! fusionados por el greedy meshing.
//!
//! Las texturas animadas (más altas que anchas) se quedan con el primer
//! fotograma; las que no miden 16 × 16 se escalan por vecino más cercano. Las
//! tintadas por bioma (`tintindex`) se multiplican por un color fijo (ver
//! [`tinte_de`]).

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::modelos::Recursos;
use crate::malla::DatosAtlas;

/// Versión del esquema de `atlas.json`/`atlas.png`. Subirla obliga a
/// regenerar los atlas guardados en la caché.
pub const ESQUEMA_ATLAS: u32 = 1;
/// Lado de cada textura dentro de la celda.
pub const TAM_TEXTURA: u32 = 16;
/// Relleno a cada lado de la textura.
pub const RELLENO: u32 = 8;
/// Lado de la celda.
pub const TAM_CELDA: u32 = TAM_TEXTURA + 2 * RELLENO;

const PREFIJO_BLOCKSTATES: &str = "assets/minecraft/blockstates/";
const PREFIJO_MODELOS: &str = "assets/minecraft/models/";
const PREFIJO_TEXTURAS: &str = "assets/minecraft/textures/";

/// Tintes fijos (RGB) de las texturas con `tintindex`.
pub mod tintes {
    /// Hierba de llanura (`plains`): hierba, helechos, caña de azúcar.
    pub const HIERBA: [u8; 3] = [0x91, 0xBD, 0x59];
    /// Follaje de llanura: hojas de roble, jungla, acacia, roble oscuro, enredaderas.
    pub const FOLLAJE: [u8; 3] = [0x77, 0xAB, 0x2F];
    pub const ABEDUL: [u8; 3] = [0x80, 0xA7, 0x55];
    pub const ABETO: [u8; 3] = [0x61, 0x99, 0x61];
    pub const MANGLE: [u8; 3] = [0x92, 0xC6, 0x48];
    /// Agua de llanura.
    pub const AGUA: [u8; 3] = [0x3F, 0x76, 0xE4];
    pub const NENUFAR: [u8; 3] = [0x20, 0x80, 0x30];
    /// Hojarasca (follaje seco).
    pub const SECO: [u8; 3] = [0xA0, 0x82, 0x50];
    /// Redstone a media potencia.
    pub const REDSTONE: [u8; 3] = [0xB8, 0x00, 0x00];
    /// Tallos de melón y calabaza maduros.
    pub const TALLO: [u8; 3] = [0xE0, 0xC7, 0x1C];
}

/// Tinte de las texturas con `tintindex` de un bloque (nombre sin espacio de
/// nombres), o `None` si el juego no lo tinta: solo se tintan los bloques con
/// un proveedor de color en Minecraft (`BlockColors`). Un `tintindex`
/// heredado no basta (el caldero de lava hereda la cara tintada del de agua).
pub fn tinte_de(bloque: &str) -> Option<[u8; 3]> {
    use tintes::*;
    Some(match bloque {
        "birch_leaves" => ABEDUL,
        "spruce_leaves" => ABETO,
        "mangrove_leaves" => MANGLE,
        "oak_leaves" | "jungle_leaves" | "acacia_leaves" | "dark_oak_leaves" | "vine" => FOLLAJE,
        "water" | "bubble_column" | "water_cauldron" => AGUA,
        "lily_pad" => NENUFAR,
        "leaf_litter" => SECO,
        "redstone_wire" => REDSTONE,
        "melon_stem" | "pumpkin_stem" | "attached_melon_stem" | "attached_pumpkin_stem" => TALLO,
        "grass_block" | "short_grass" | "tall_grass" | "fern" | "large_fern" | "potted_fern"
        | "sugar_cane" | "pink_petals" | "wildflowers" | "bush" => HIERBA,
        _ => return None,
    })
}

/// Lo que se lee del jar.
pub struct ContenidoJar {
    pub recursos: Recursos,
    /// `block/stone` → bytes del PNG (vacío si se pidió solo el JSON).
    pub pngs: BTreeMap<String, Vec<u8>>,
}

/// Lee blockstates y modelos de bloque del jar y, si `con_png`, también las
/// texturas de bloque.
pub fn leer_jar(ruta: &Path, con_png: bool) -> Result<ContenidoJar, String> {
    let f = fs::File::open(ruta).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(f)).map_err(|e| e.to_string())?;
    let mut blockstates = Vec::new();
    let mut modelos = Vec::new();
    let mut pngs = BTreeMap::new();
    for i in 0..zip.len() {
        let mut e = zip.by_index(i).map_err(|e| e.to_string())?;
        if !e.is_file() {
            continue;
        }
        let Ok(nombre) = e.name().map(|n| n.into_owned()) else {
            continue;
        };
        let destino = if let Some(r) = nombre.strip_prefix(PREFIJO_BLOCKSTATES) {
            r.strip_suffix(".json").map(|k| (0, k.to_string()))
        } else if let Some(r) = nombre.strip_prefix(PREFIJO_MODELOS) {
            r.strip_prefix("block/")
                .and_then(|_| r.strip_suffix(".json"))
                .map(|k| (1, k.to_string()))
        } else if con_png && let Some(r) = nombre.strip_prefix(PREFIJO_TEXTURAS) {
            r.strip_prefix("block/")
                .and_then(|_| r.strip_suffix(".png"))
                .map(|k| (2, k.to_string()))
        } else {
            None
        };
        let Some((tipo, clave)) = destino else {
            continue;
        };
        let declarado = e.size();
        let bytes = super::leer_entrada_jar(&mut e, declarado).map_err(|e| e.to_string())?;
        match tipo {
            0 => blockstates.push((clave, bytes)),
            1 => modelos.push((clave, bytes)),
            _ => {
                pngs.insert(clave, bytes);
            }
        }
    }
    if blockstates.is_empty() {
        return Err("el jar no trae blockstates de Minecraft".into());
    }
    let recursos = Recursos::desde_json(
        blockstates.iter().map(|(k, v)| (k.clone(), v.as_slice())),
        modelos.iter().map(|(k, v)| (k.clone(), v.as_slice())),
    );
    Ok(ContenidoJar { recursos, pngs })
}

/// Imagen RGBA de 16 × 16 (1024 bytes).
pub type Textura = Vec<u8>;

/// Decodifica un PNG a RGBA 16 × 16: primer fotograma si es animada y escala
/// por vecino más cercano si no mide 16.
pub fn decodificar(png_bytes: &[u8]) -> Option<Textura> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(png_bytes));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut lector = dec.read_info().ok()?;
    let mut buf = vec![0; lector.output_buffer_size()?];
    let info = lector.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let canales = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };
    let fila = info.line_size;
    // Fotograma: el cuadrado de arriba (las animadas son tiras verticales).
    let lado_y = w.min(h);
    let t = TAM_TEXTURA as usize;
    let mut out = vec![0u8; t * t * 4];
    for y in 0..t {
        for x in 0..t {
            let sx = x * w / t;
            let sy = y * lado_y / t;
            let p = &buf[sy * fila + sx * canales..][..canales];
            let rgba = match canales {
                4 => [p[0], p[1], p[2], p[3]],
                3 => [p[0], p[1], p[2], 255],
                2 => [p[0], p[0], p[0], p[1]],
                _ => [p[0], p[0], p[0], 255],
            };
            out[(y * t + x) * 4..][..4].copy_from_slice(&rgba);
        }
    }
    Some(out)
}

/// Multiplica el color por un tinte.
pub fn tintar(t: &mut Textura, tinte: [u8; 3]) {
    for px in t.as_chunks_mut::<4>().0 {
        for c in 0..3 {
            px[c] = ((u16::from(px[c]) * u16::from(tinte[c]) + 127) / 255) as u8;
        }
    }
}

/// Color medio ponderado por alfa. `None` si es totalmente transparente.
pub fn color_medio(t: &Textura) -> Option<[u8; 3]> {
    let mut suma = [0u64; 3];
    let mut peso = 0u64;
    for px in t.as_chunks::<4>().0 {
        let a = u64::from(px[3]);
        for c in 0..3 {
            suma[c] += u64::from(px[c]) * a;
        }
        peso += a;
    }
    (peso > 0).then(|| suma.map(|s| ((s + peso / 2) / peso) as u8))
}

/// Tinte de cada textura tintada: se recorren todos los modelos de todos los
/// blockstates; si una textura la tintan varios bloques, gana el primero en
/// orden alfabético.
pub fn mapa_tintes(r: &Recursos) -> HashMap<String, [u8; 3]> {
    let mut bloques: Vec<&str> = r.bloques().collect();
    bloques.sort_unstable();
    let mut mapa = HashMap::new();
    for b in bloques {
        let Some(tinte) = tinte_de(b) else {
            continue;
        };
        for m in r.todos_los_modelos(b) {
            for t in r.texturas_tintadas(&m) {
                mapa.entry(t).or_insert(tinte);
            }
        }
    }
    // El agua y la lava no tienen elementos (las pinta el juego aparte); su
    // textura llega por `particle` y el agua lleva el tinte de bioma.
    for t in ["block/water_still", "block/water_flow"] {
        mapa.insert(t.to_string(), tintes::AGUA);
    }
    mapa
}

/// Atlas montado.
pub struct Atlas {
    pub meta: MetaAtlas,
    /// PNG codificado.
    pub png: Vec<u8>,
    /// Texturas ya decodificadas y tintadas, por celda (para la tabla de
    /// colores; no se guardan en la caché).
    pub texturas: Vec<Textura>,
}

/// `atlas.json`: lo que hay que saber del PNG.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaAtlas {
    pub esquema: u32,
    /// Versión de Minecraft.
    pub version: String,
    /// Tamaño del jar del que salió (si cambia, se regenera).
    pub bytes_jar: u64,
    pub ancho: u32,
    pub alto: u32,
    pub celdas_por_fila: u32,
    pub tam_celda: u32,
    pub tam_textura: u32,
    pub relleno: u32,
    /// `block/stone` → celda.
    pub texturas: BTreeMap<String, u32>,
    /// Color medio (con tinte) de cada celda.
    pub colores: Vec<[u8; 3]>,
}

impl MetaAtlas {
    pub fn datos(&self) -> DatosAtlas {
        DatosAtlas {
            version: self.version.clone(),
            ancho: self.ancho,
            alto: self.alto,
            celdas_por_fila: self.celdas_por_fila,
            tam_celda: self.tam_celda,
            tam_textura: self.tam_textura,
            relleno: self.relleno,
            celda_uv: [
                self.tam_celda as f32 / self.ancho as f32,
                self.tam_celda as f32 / self.alto as f32,
            ],
            niveles_mip: self.relleno.max(1).ilog2(),
        }
    }
}

/// Tamaño en píxeles (potencias de dos) para `n` celdas: el lado más pequeño
/// cuyo cuadrado cabe y, si sobra la mitad de abajo, se recorta.
pub fn dimensiones(n: usize) -> (u32, u32, u32) {
    let mut ancho = TAM_CELDA;
    while ((ancho / TAM_CELDA) as usize).pow(2) < n.max(1) {
        ancho *= 2;
    }
    let por_fila = ancho / TAM_CELDA;
    let filas = (n.max(1) as u32).div_ceil(por_fila);
    let alto = (filas * TAM_CELDA).next_power_of_two();
    (ancho, alto, por_fila)
}

/// Monta el atlas con todas las texturas que se puedan decodificar, en orden
/// alfabético de nombre (estable entre ejecuciones).
pub fn montar(
    version: &str,
    bytes_jar: u64,
    pngs: &BTreeMap<String, Vec<u8>>,
    tintes: &HashMap<String, [u8; 3]>,
) -> Result<Atlas, String> {
    let mut nombres = Vec::new();
    let mut texturas = Vec::new();
    for (nombre, bytes) in pngs {
        let Some(mut t) = decodificar(bytes) else {
            continue;
        };
        if let Some(&tinte) = tintes.get(nombre) {
            tintar(&mut t, tinte);
        }
        nombres.push(nombre.clone());
        texturas.push(t);
    }
    let (ancho, alto, por_fila) = dimensiones(texturas.len());
    let mut px = vec![0u8; (ancho * alto * 4) as usize];
    let (c, t, r) = (TAM_CELDA as usize, TAM_TEXTURA as usize, RELLENO as usize);
    for (i, tex) in texturas.iter().enumerate() {
        let x0 = (i as u32 % por_fila) as usize * c;
        let y0 = (i as u32 / por_fila) as usize * c;
        for y in 0..c {
            let sy = (y + t - r) % t;
            for x in 0..c {
                let sx = (x + t - r) % t;
                let d = ((y0 + y) * ancho as usize + x0 + x) * 4;
                px[d..d + 4].copy_from_slice(&tex[(sy * t + sx) * 4..][..4]);
            }
        }
    }
    let png = codificar(&px, ancho, alto)?;
    let colores = texturas
        .iter()
        .map(|t| color_medio(t).unwrap_or([0, 0, 0]))
        .collect();
    let meta = MetaAtlas {
        esquema: ESQUEMA_ATLAS,
        version: version.to_string(),
        bytes_jar,
        ancho,
        alto,
        celdas_por_fila: por_fila,
        tam_celda: TAM_CELDA,
        tam_textura: TAM_TEXTURA,
        relleno: RELLENO,
        texturas: nombres
            .into_iter()
            .enumerate()
            .map(|(i, n)| (n, i as u32))
            .collect(),
        colores,
    };
    Ok(Atlas {
        meta,
        png,
        texturas,
    })
}

fn codificar(px: &[u8], ancho: u32, alto: u32) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, ancho, alto);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(px).map_err(|e| e.to_string())?;
    }
    Ok(out)
}
