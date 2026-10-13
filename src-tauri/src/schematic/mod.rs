// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Lectura de schematics de Minecraft (.litematic, .schem, .schematic y .nbt)
//! a un modelo común. El formato se detecta por el contenido, nunca por la
//! extensión del archivo.

mod escritor_litematic;
mod estructura;
mod litematic;
mod mcedit;
pub mod modelo;
mod nbt;
mod sponge;

use std::borrow::Cow;
use std::io::Read;

use serde::Deserialize;
use serde::de::IgnoredAny;

pub use escritor_litematic::{
    ErrorEscritura, OpcionesLitematic, VERSION_DATOS_POR_DEFECTO, VERSION_LITEMATIC,
    escribir_empaquetado, escribir_litematic, escribir_litematic_en,
};
pub use litematic::leer_empaquetado;
pub use modelo::{Dimensiones, ExtrasRegion, Formato, Region, Schematic, es_aire};
pub use sponge::leer_varints;

/// Máximo de posiciones (sumando regiones) que se aceptan: 512 Mi.
pub const MAX_POSICIONES: u64 = 512 * 1024 * 1024;

/// Máximo de posiciones sin datos que las respalden (aire implícito de las
/// estructuras y de Sponge v3 sin bloques): 16 Mi, 64 MiB de índices. Sin este
/// tope, un archivo de pocos bytes podría declarar un volumen enorme y obligar
/// a reservar gigas antes de validar nada.
pub const MAX_POSICIONES_IMPLICITAS: u64 = 16 * 1024 * 1024;

/// Comprueba que un volumen sin datos densos detrás no se dispare: se admite
/// hasta [`MAX_POSICIONES_IMPLICITAS`] o 8 posiciones por bloque declarado, lo
/// que sea mayor (así el gasto de memoria crece con el tamaño del archivo).
pub(crate) fn limitar_implicito(volumen: u64, declarados: u64) -> Result<(), ErrorSchematic> {
    let tope = MAX_POSICIONES_IMPLICITAS.max(declarados.saturating_mul(8));
    if volumen > tope {
        return Err(ErrorSchematic::DemasiadoGrande(format!(
            "declara {volumen} posiciones pero solo describe {declarados} bloques"
        )));
    }
    Ok(())
}

/// Máximo de bytes NBT tras descomprimir (protege de «bombas» de compresión).
pub const MAX_BYTES_DESCOMPRIMIDOS: u64 = 1024 * 1024 * 1024;

/// Errores al leer un schematic. Nunca se produce un panic con datos malos.
#[derive(Debug, thiserror::Error)]
pub enum ErrorSchematic {
    /// Datos binarios dañados: compresión rota, NBT truncado o tipos inesperados.
    /// El detalle es técnico (en inglés, del decodificador) y no se enseña al usuario.
    #[error("datos dañados o incompletos ({detalle})")]
    Corrupto { detalle: String },
    /// El NBT es legible pero su contenido no es coherente (texto para el usuario).
    #[error("{0}")]
    Invalido(String),
    /// Dimensiones o datos desproporcionados.
    #[error("{0}")]
    DemasiadoGrande(String),
    /// No se reconoce ninguno de los cuatro formatos.
    #[error("no es un schematic reconocible")]
    FormatoDesconocido,
}

impl ErrorSchematic {
    pub(crate) fn corrupto(e: impl std::fmt::Display) -> Self {
        ErrorSchematic::Corrupto {
            detalle: e.to_string(),
        }
    }
}

/// Lee un schematic desde los bytes del archivo (comprimido o no).
pub fn cargar(bytes: &[u8]) -> Result<Schematic, ErrorSchematic> {
    let datos = descomprimir(bytes)?;
    let datos: &[u8] = &datos;
    let sondeo: Sondeo = nbt::deserializar(datos)?;

    let schematic = if sondeo.regions.is_some() {
        litematic::leer(datos)?
    } else if sondeo.schematic.is_some() {
        sponge::leer_v3_envuelto(datos)?
    } else if sondeo.palette.is_some() && sondeo.block_data.is_some() {
        sponge::leer_v1_v2(datos)?
    } else if sondeo.blocks.is_some() && sondeo.width.is_some() {
        if sondeo.materials.is_none() && sondeo.version.is_some_and(|v| v >= 3) {
            sponge::leer_v3_plano(datos)?
        } else {
            mcedit::leer(datos)?
        }
    } else if sondeo.size.is_some()
        && sondeo.blocks_min.is_some()
        && (sondeo.palette_min.is_some() || sondeo.palettes.is_some())
    {
        estructura::leer(datos)?
    } else {
        return Err(ErrorSchematic::FormatoDesconocido);
    };

    validar_final(&schematic)?;
    Ok(schematic)
}

/// Campos de la raíz que bastan para decidir el formato. `IgnoredAny` salta
/// los datos sin copiarlos.
#[derive(Deserialize)]
struct Sondeo {
    #[serde(rename = "Regions")]
    regions: Option<IgnoredAny>,
    #[serde(rename = "Schematic")]
    schematic: Option<IgnoredAny>,
    #[serde(rename = "Version")]
    version: Option<i32>,
    #[serde(rename = "Palette")]
    palette: Option<IgnoredAny>,
    #[serde(rename = "BlockData")]
    block_data: Option<IgnoredAny>,
    #[serde(rename = "Blocks")]
    blocks: Option<IgnoredAny>,
    #[serde(rename = "Materials")]
    materials: Option<IgnoredAny>,
    #[serde(rename = "Width")]
    width: Option<IgnoredAny>,
    size: Option<IgnoredAny>,
    #[serde(rename = "blocks")]
    blocks_min: Option<IgnoredAny>,
    #[serde(rename = "palette")]
    palette_min: Option<IgnoredAny>,
    palettes: Option<IgnoredAny>,
}

/// Quita la compresión según la cabecera: gzip (1f 8b), zlib (78 xx) o NBT crudo.
fn descomprimir(bytes: &[u8]) -> Result<Cow<'_, [u8]>, ErrorSchematic> {
    match bytes {
        [0x1f, 0x8b, ..] => leer_limitado(flate2::read::MultiGzDecoder::new(bytes)),
        [0x78, b, ..] if ((0x78u16 << 8) | u16::from(*b)).is_multiple_of(31) => {
            leer_limitado(flate2::read::ZlibDecoder::new(bytes))
        }
        // Un NBT válido empieza con la etiqueta Compound (10).
        [0x0a, ..] => Ok(Cow::Borrowed(bytes)),
        _ => Err(ErrorSchematic::FormatoDesconocido),
    }
}

fn leer_limitado(lector: impl Read) -> Result<Cow<'static, [u8]>, ErrorSchematic> {
    let mut salida = Vec::new();
    lector
        .take(MAX_BYTES_DESCOMPRIMIDOS + 1)
        .read_to_end(&mut salida)
        .map_err(ErrorSchematic::corrupto)?;
    if salida.len() as u64 > MAX_BYTES_DESCOMPRIMIDOS {
        return Err(ErrorSchematic::DemasiadoGrande(
            "el archivo ocupa más de 1 GiB al descomprimirlo".into(),
        ));
    }
    Ok(Cow::Owned(salida))
}

/// Comprobaciones comunes a todos los formatos tras traducirlos.
fn validar_final(s: &Schematic) -> Result<(), ErrorSchematic> {
    if s.regiones.is_empty() {
        return Err(ErrorSchematic::Invalido(
            "no contiene ninguna región".into(),
        ));
    }
    let total: u64 = s.regiones.iter().map(|r| r.tamano.volumen()).sum();
    if total > MAX_POSICIONES {
        return Err(demasiado_grande(total));
    }
    for r in &s.regiones {
        if r.bloques.len() as u64 != r.tamano.volumen() {
            return Err(ErrorSchematic::Invalido(format!(
                "la región «{}» no tiene un bloque por posición",
                r.nombre
            )));
        }
        let n = r.paleta.len();
        if r.bloques.iter().any(|&p| p as usize >= n) {
            return Err(ErrorSchematic::Invalido(format!(
                "la región «{}» usa un bloque que no está en su paleta",
                r.nombre
            )));
        }
    }
    Ok(())
}

fn demasiado_grande(posiciones: u64) -> ErrorSchematic {
    ErrorSchematic::DemasiadoGrande(format!(
        "tiene {posiciones} posiciones y el máximo admitido es {MAX_POSICIONES}"
    ))
}

/// Valida un tamaño leído del archivo y lo convierte en `Dimensiones`.
pub(crate) fn validar_tamano(
    ancho: i64,
    alto: i64,
    largo: i64,
    que: &str,
) -> Result<Dimensiones, ErrorSchematic> {
    if ancho <= 0 || alto <= 0 || largo <= 0 {
        return Err(ErrorSchematic::Invalido(format!(
            "{que} tiene un tamaño no válido ({ancho}×{alto}×{largo})"
        )));
    }
    let a = u32::try_from(ancho).map_err(|_| demasiado_grande(u64::MAX))?;
    let h = u32::try_from(alto).map_err(|_| demasiado_grande(u64::MAX))?;
    let l = u32::try_from(largo).map_err(|_| demasiado_grande(u64::MAX))?;
    let d = Dimensiones {
        ancho: a,
        alto: h,
        largo: l,
    };
    if d.volumen() > MAX_POSICIONES {
        return Err(demasiado_grande(d.volumen()));
    }
    Ok(d)
}

/// Reserva el vector de bloques sin abortar el proceso si no hay memoria.
pub(crate) fn reservar_bloques(volumen: u64) -> Result<Vec<u32>, ErrorSchematic> {
    let n = usize::try_from(volumen).map_err(|_| demasiado_grande(volumen))?;
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| {
        ErrorSchematic::DemasiadoGrande("no hay memoria suficiente para abrirlo".into())
    })?;
    Ok(v)
}

/// Paleta en construcción: asigna un índice a cada block state distinto.
pub(crate) struct Paleta {
    indices: std::collections::HashMap<String, u32>,
    estados: Vec<String>,
}

impl Paleta {
    pub fn nueva() -> Self {
        Paleta {
            indices: Default::default(),
            estados: Vec::new(),
        }
    }

    /// Paleta cuyo índice 0 es `minecraft:air` (para formatos con huecos).
    pub fn con_aire() -> Self {
        let mut p = Paleta::nueva();
        p.insertar("minecraft:air".to_string());
        p
    }

    /// Devuelve el índice del estado (ya normalizado), añadiéndolo si es nuevo.
    pub fn insertar(&mut self, estado: String) -> u32 {
        if let Some(&i) = self.indices.get(&estado) {
            return i;
        }
        let i = self.estados.len() as u32;
        self.indices.insert(estado.clone(), i);
        self.estados.push(estado);
        i
    }

    pub fn en_vec(self) -> Vec<String> {
        self.estados
    }
}

/// Añade `minecraft:` si el nombre no trae namespace.
fn con_namespace(nombre: &str) -> String {
    let nombre = nombre.trim();
    if nombre.contains(':') {
        nombre.to_string()
    } else {
        format!("minecraft:{nombre}")
    }
}

/// Construye el texto canónico `ns:nombre[a=1,b=2]` con las propiedades
/// ordenadas alfabéticamente por clave.
pub(crate) fn construir_estado<'a>(
    nombre: &str,
    propiedades: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> String {
    let mut props: Vec<(&str, &str)> = propiedades
        .into_iter()
        .map(|(k, v)| (k.trim(), v.trim()))
        .filter(|(k, _)| !k.is_empty())
        .collect();
    props.sort_unstable();
    let mut texto = con_namespace(nombre);
    if !props.is_empty() {
        texto.push('[');
        for (i, (k, v)) in props.iter().enumerate() {
            if i > 0 {
                texto.push(',');
            }
            texto.push_str(k);
            texto.push('=');
            texto.push_str(v);
        }
        texto.push(']');
    }
    texto
}

/// Normaliza un block state escrito como texto (`piedra`, `ns:x[b=2,a=1]`…).
pub fn normalizar_estado(texto: &str) -> String {
    let texto = texto.trim();
    match texto.split_once('[') {
        None => con_namespace(texto),
        Some((nombre, resto)) => {
            let resto = resto.strip_suffix(']').unwrap_or(resto);
            let props = resto.split(',').map(|par| match par.split_once('=') {
                Some((k, v)) => (k, v),
                None => (par, ""),
            });
            construir_estado(nombre, props)
        }
    }
}

/// Texto opcional sin espacios sobrantes; vacío equivale a ausente.
pub(crate) fn texto_util(t: Option<String>) -> Option<String> {
    t.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn normaliza_namespace_y_orden() {
        assert_eq!(normalizar_estado("stone"), "minecraft:stone");
        assert_eq!(
            normalizar_estado("oak_stairs[half=bottom,facing=north]"),
            "minecraft:oak_stairs[facing=north,half=bottom]"
        );
        assert_eq!(normalizar_estado("mod:cosa"), "mod:cosa");
    }

    #[test]
    fn basura_y_vacio_no_hacen_panic() {
        assert!(cargar(&[]).is_err());
        assert!(cargar(&[0x1f, 0x8b, 0, 0]).is_err());
        assert!(cargar(&[0x0a]).is_err());
        assert!(cargar(&[0x0a, 0, 0, 0]).is_err());
    }
}
