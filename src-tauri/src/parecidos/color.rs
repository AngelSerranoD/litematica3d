// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Color de cada block state y distancia entre colores en OKLab.
//!
//! - **Media de una textura**: media aritmética (en sRGB, como
//!   `colores_bloques.json`) de los píxeles con alfa ≥ [`UMBRAL_ALFA`]; los
//!   transparentes o casi no cuentan. Se guarda también cuántos píxeles
//!   contaron, para ponderar.
//! - **Color de un estado**: la cara de arriba pesa [`PESO_ARRIBA`] y cada
//!   lado (norte, sur, este, oeste) [`PESO_LADO`]: la de arriba es la que más
//!   se ve en una construcción mirada desde fuera, pero los cuatro lados juntos
//!   pesan el doble. Cada cara aporta sus píxeles opacos multiplicados por su
//!   peso (una cara medio transparente pesa la mitad). La de abajo no cuenta.
//!   Si no hay ninguna de esas caras (plantas), la del plano cruzado.
//! - **OKLab** (Björn Ottosson, 2020): espacio perceptual en el que la
//!   distancia euclídea se parece a la diferencia que ve el ojo. `L` va de 0
//!   (negro) a 1 (blanco); `a` y `b`, más o menos entre -0,4 y 0,4.
//!
//! Las medias por textura salen del atlas activo (que ya lleva los tintes de
//! bioma) y se guardan en `<cache>/texturas/<versión>/colores.json`.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::malla::Cara;
use crate::texturas::atlas::MetaAtlas;
use crate::texturas::{Activo, carpeta_version, escribir};

/// Alfa mínimo para que un píxel cuente en la media (0-255).
pub const UMBRAL_ALFA: u8 = 128;
/// Peso de la cara de arriba en el color de un estado.
pub const PESO_ARRIBA: u64 = 2;
/// Peso de cada cara lateral en el color de un estado.
pub const PESO_LADO: u64 = 1;
/// Nombre del archivo de caché dentro de la carpeta de la versión.
pub const ARCHIVO_CACHE: &str = "colores.json";
/// Versión del esquema de `colores.json`. Subirla obliga a recalcularlo.
pub const ESQUEMA_CACHE: u32 = 1;

/// Color en OKLab.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Oklab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

impl Oklab {
    /// Convierte un color sRGB de 8 bits.
    pub fn desde_rgb(rgb: [u8; 3]) -> Self {
        let [r, g, b] = rgb.map(|c| a_lineal(f64::from(c) / 255.0));
        let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
        let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
        let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
        let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
        Oklab {
            l: 0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
            a: 1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
            b: 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
        }
    }

    /// Distancia euclídea en OKLab (ΔE_ok). Diferencias por debajo de ~0,02
    /// apenas se notan; por encima de ~0,1 son colores claramente distintos.
    pub fn distancia(self, otro: Oklab) -> f64 {
        ((self.l - otro.l).powi(2) + (self.a - otro.a).powi(2) + (self.b - otro.b).powi(2)).sqrt()
    }
}

/// Distancia OKLab entre dos colores sRGB.
pub fn distancia_rgb(a: [u8; 3], b: [u8; 3]) -> f64 {
    Oklab::desde_rgb(a).distancia(Oklab::desde_rgb(b))
}

/// sRGB (0-1) → lineal.
fn a_lineal(c: f64) -> f64 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Media de una textura: color medio de los píxeles opacos y cuántos son.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaTextura {
    pub rgb: [u8; 3],
    /// Píxeles con alfa ≥ [`UMBRAL_ALFA`] (0 = textura transparente).
    pub n: u32,
}

/// Media de una imagen RGBA (cualquier tamaño) ignorando los píxeles con
/// alfa < [`UMBRAL_ALFA`].
pub fn media_textura(rgba: &[u8]) -> MediaTextura {
    let mut acc = Acumulador::default();
    for px in rgba.as_chunks::<4>().0 {
        if px[3] >= UMBRAL_ALFA {
            acc.sumar_pixel([px[0], px[1], px[2]], 1);
        }
    }
    MediaTextura {
        rgb: acc.color().unwrap_or_default(),
        n: acc.n as u32,
    }
}

/// Suma ponderada de colores.
#[derive(Debug, Default, Clone, Copy)]
struct Acumulador {
    suma: [u64; 3],
    n: u64,
}

impl Acumulador {
    fn sumar_pixel(&mut self, rgb: [u8; 3], peso: u64) {
        for (s, c) in self.suma.iter_mut().zip(rgb) {
            *s += u64::from(c) * peso;
        }
        self.n += peso;
    }

    /// Suma una textura entera: cada uno de sus píxeles opacos con `peso`.
    fn sumar_media(&mut self, m: MediaTextura, peso: u64) {
        self.sumar_pixel(m.rgb, u64::from(m.n) * peso);
    }

    fn color(&self) -> Option<[u8; 3]> {
        (self.n > 0).then(|| self.suma.map(|s| ((s + self.n / 2) / self.n) as u8))
    }
}

/// Medias de todas las texturas del atlas (por nombre: `block/stone`),
/// leyendo la parte central de cada celda del PNG.
pub fn medias_atlas(png: &[u8], meta: &MetaAtlas) -> Result<HashMap<String, MediaTextura>, String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(png));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut lector = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![
        0;
        lector
            .output_buffer_size()
            .ok_or("atlas demasiado grande")?
    ];
    let info = lector.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if info.color_type != png::ColorType::Rgba || info.width != meta.ancho {
        return Err("el atlas no es RGBA del tamaño esperado".into());
    }
    let (fila, t) = (info.line_size, meta.tam_textura as usize);
    let mut medias = HashMap::with_capacity(meta.texturas.len());
    let mut celda = Vec::with_capacity(t * t * 4);
    for (nombre, &i) in &meta.texturas {
        let x0 = (i % meta.celdas_por_fila * meta.tam_celda + meta.relleno) as usize;
        let y0 = (i / meta.celdas_por_fila * meta.tam_celda + meta.relleno) as usize;
        celda.clear();
        for y in y0..y0 + t {
            let ini = y * fila + x0 * 4;
            celda.extend_from_slice(buf.get(ini..ini + t * 4).ok_or("celda fuera del atlas")?);
        }
        medias.insert(nombre.clone(), media_textura(&celda));
    }
    Ok(medias)
}

/// Color de un block state (`minecraft:oak_log[axis=x]`) con las texturas
/// del jar: caras de arriba y laterales ponderadas (ver el módulo). `None` si
/// el bloque no está en el jar o no tiene ninguna textura opaca.
pub fn color_de_estado(
    activo: &Activo,
    medias: &HashMap<String, MediaTextura>,
    estado: &str,
) -> Option<[u8; 3]> {
    let media = |cara: Cara| -> Option<MediaTextura> {
        let r = activo.recursos.resolver(estado, cara)?;
        medias.get(&r.textura).copied()
    };
    let mut acc = Acumulador::default();
    let caras = [
        (Cara::Arriba, PESO_ARRIBA),
        (Cara::Norte, PESO_LADO),
        (Cara::Sur, PESO_LADO),
        (Cara::Este, PESO_LADO),
        (Cara::Oeste, PESO_LADO),
    ];
    for (cara, peso) in caras {
        if let Some(m) = media(cara) {
            acc.sumar_media(m, peso);
        }
    }
    if acc.n == 0
        && let Some(m) = media(Cara::Cruz)
    {
        acc.sumar_media(m, 1);
    }
    acc.color()
}

/// `colores.json`: medias por textura y color del estado por defecto de cada
/// bloque de una versión.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheColores {
    pub esquema: u32,
    /// Versión de Minecraft.
    pub version: String,
    /// Tamaño del jar del que salió (si cambia, se recalcula).
    pub bytes_jar: u64,
    /// Umbral de alfa con que se calculó (si cambia, se recalcula).
    pub umbral_alfa: u8,
    /// `block/stone` → media.
    pub texturas: BTreeMap<String, MediaTextura>,
    /// `minecraft:stone` → color de su estado por defecto (solo los que
    /// tienen alguna textura opaca).
    pub bloques: BTreeMap<String, [u8; 3]>,
}

/// Ruta de `colores.json` de una versión dentro de `<cache>/texturas`.
pub fn ruta_cache(dir_cache: &Path, version: &str) -> PathBuf {
    dir_cache.join(carpeta_version(version)).join(ARCHIVO_CACHE)
}

/// Lee `colores.json` si vale para este atlas (mismo esquema, versión, jar y
/// umbral).
pub fn leer_cache(dir_cache: &Path, meta: &MetaAtlas) -> Option<CacheColores> {
    let bytes = fs::read(ruta_cache(dir_cache, &meta.version)).ok()?;
    serde_json::from_slice::<CacheColores>(&bytes)
        .ok()
        .filter(|c| {
            c.esquema == ESQUEMA_CACHE
                && c.version == meta.version
                && c.bytes_jar == meta.bytes_jar
                && c.umbral_alfa == UMBRAL_ALFA
        })
}

/// Guarda `colores.json` (a un temporal y renombrando). Si falla, se
/// recalcula la próxima vez: no es un error para quien lo usa.
pub fn guardar_cache(dir_cache: &Path, cache: &CacheColores) -> std::io::Result<()> {
    let ruta = ruta_cache(dir_cache, &cache.version);
    if let Some(dir) = ruta.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec(cache).map_err(std::io::Error::other)?;
    escribir(&ruta, &json)
}
