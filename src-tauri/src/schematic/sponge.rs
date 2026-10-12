// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Formato Sponge (.schem de WorldEdit) v1, v2 y v3: una caja con paleta
//! nombre → índice y los índices codificados como varints.

use std::collections::HashMap;

use serde::Deserialize;

use super::modelo::{Formato, Region, Schematic, caja_englobante};
use super::nbt::deserializar;
use super::{
    ErrorSchematic, Paleta, normalizar_estado, reservar_bloques, texto_util, validar_tamano,
};

/// Índice de paleta máximo admitido (evita tablas enormes con datos malos).
const MAX_INDICE_PALETA: i32 = 1 << 24;

#[derive(Deserialize)]
struct Metadatos {
    #[serde(rename = "Name")]
    nombre: Option<String>,
    #[serde(rename = "Author")]
    autor: Option<String>,
    #[serde(rename = "Date")]
    fecha: Option<i64>,
}

#[derive(Deserialize)]
struct SpongeV2 {
    #[serde(rename = "DataVersion")]
    version_datos: Option<i32>,
    #[serde(rename = "Width")]
    ancho: i32,
    #[serde(rename = "Height")]
    alto: i32,
    #[serde(rename = "Length")]
    largo: i32,
    #[serde(rename = "Palette")]
    paleta: HashMap<String, i32>,
    #[serde(rename = "BlockData")]
    datos: fastnbt::ByteArray,
    #[serde(rename = "Metadata")]
    metadatos: Option<Metadatos>,
}

#[derive(Deserialize)]
struct BloquesV3 {
    #[serde(rename = "Palette")]
    paleta: HashMap<String, i32>,
    #[serde(rename = "Data")]
    datos: fastnbt::ByteArray,
}

#[derive(Deserialize)]
struct SpongeV3 {
    #[serde(rename = "DataVersion")]
    version_datos: Option<i32>,
    #[serde(rename = "Width")]
    ancho: i32,
    #[serde(rename = "Height")]
    alto: i32,
    #[serde(rename = "Length")]
    largo: i32,
    #[serde(rename = "Blocks")]
    bloques: Option<BloquesV3>,
    #[serde(rename = "Metadata")]
    metadatos: Option<Metadatos>,
}

#[derive(Deserialize)]
struct Envoltorio {
    #[serde(rename = "Schematic")]
    schematic: SpongeV3,
}

pub(crate) fn leer_v1_v2(datos: &[u8]) -> Result<Schematic, ErrorSchematic> {
    let s: SpongeV2 = deserializar(datos)?;
    construir(
        s.version_datos,
        [s.ancho, s.alto, s.largo],
        Some((&s.paleta, &s.datos)),
        s.metadatos,
    )
}

pub(crate) fn leer_v3_envuelto(datos: &[u8]) -> Result<Schematic, ErrorSchematic> {
    let e: Envoltorio = deserializar(datos)?;
    leer_v3(e.schematic)
}

pub(crate) fn leer_v3_plano(datos: &[u8]) -> Result<Schematic, ErrorSchematic> {
    let s: SpongeV3 = deserializar(datos)?;
    leer_v3(s)
}

fn leer_v3(s: SpongeV3) -> Result<Schematic, ErrorSchematic> {
    construir(
        s.version_datos,
        [s.ancho, s.alto, s.largo],
        s.bloques.as_ref().map(|b| (&b.paleta, &b.datos)),
        s.metadatos,
    )
}

/// Los lados son Short sin signo: un valor negativo es en realidad > 32767.
fn lado_sin_signo(v: i32) -> i64 {
    if (-32768..0).contains(&v) {
        i64::from(v) + 65536
    } else {
        i64::from(v)
    }
}

fn construir(
    version_datos: Option<i32>,
    lados: [i32; 3],
    bloques: Option<(&HashMap<String, i32>, &fastnbt::ByteArray)>,
    metadatos: Option<Metadatos>,
) -> Result<Schematic, ErrorSchematic> {
    let tamano = validar_tamano(
        lado_sin_signo(lados[0]),
        lado_sin_signo(lados[1]),
        lado_sin_signo(lados[2]),
        "el schematic",
    )?;
    let volumen = tamano.volumen();
    let mut paleta = Paleta::nueva();
    let mut indices = reservar_bloques(volumen)?;

    match bloques {
        // v3 sin bloques (solo biomas o entidades): todo aire.
        None => {
            let aire = paleta.insertar("minecraft:air".into());
            indices.resize(volumen as usize, aire);
        }
        Some((paleta_nbt, datos)) => {
            let tabla = tabla_paleta(paleta_nbt, &mut paleta)?;
            let bytes: &[i8] = datos;
            // Cada varint ocupa al menos un byte.
            if (bytes.len() as u64) < volumen {
                return Err(ErrorSchematic::Invalido(format!(
                    "faltan datos de bloques ({} bytes para {volumen} posiciones)",
                    bytes.len()
                )));
            }
            let bytes: Vec<u8> = bytes.iter().map(|&b| b as u8).collect();
            let mut pos = 0usize;
            for _ in 0..volumen {
                let v = siguiente_varint(&bytes, &mut pos).ok_or_else(|| {
                    ErrorSchematic::Invalido("los datos de bloques están incompletos".into())
                })?;
                let p = tabla
                    .get(v as usize)
                    .copied()
                    .filter(|&p| p != u32::MAX)
                    .ok_or_else(|| {
                        ErrorSchematic::Invalido("usa un bloque que no está en su paleta".into())
                    })?;
                indices.push(p);
            }
        }
    }

    let (nombre, autor, fecha) = match metadatos {
        Some(m) => (m.nombre, m.autor, m.fecha),
        None => (None, None, None),
    };
    let regiones = vec![Region {
        nombre: "Principal".into(),
        origen: [0, 0, 0],
        tamano,
        paleta: paleta.en_vec(),
        bloques: indices,
    }];
    Ok(Schematic {
        formato: Formato::Sponge,
        nombre: texto_util(nombre),
        autor: texto_util(autor),
        descripcion: None,
        version_datos,
        fecha_creacion: fecha.filter(|&t| t > 0),
        dimensiones: caja_englobante(&regiones),
        regiones,
    })
}

/// Tabla índice del archivo → índice en la paleta normalizada (`u32::MAX` = sin uso).
fn tabla_paleta(
    paleta_nbt: &HashMap<String, i32>,
    paleta: &mut Paleta,
) -> Result<Vec<u32>, ErrorSchematic> {
    if paleta_nbt.is_empty() {
        return Err(ErrorSchematic::Invalido("la paleta está vacía".into()));
    }
    let mut entradas: Vec<(&String, i32)> = paleta_nbt.iter().map(|(k, &v)| (k, v)).collect();
    entradas.sort_by_key(|&(_, v)| v);
    let maximo = entradas.last().map(|&(_, v)| v).unwrap_or(0);
    if entradas[0].1 < 0 || maximo > MAX_INDICE_PALETA {
        return Err(ErrorSchematic::Invalido(
            "la paleta tiene índices no válidos".into(),
        ));
    }
    let mut tabla = vec![u32::MAX; maximo as usize + 1];
    for (estado, i) in entradas {
        let celda = &mut tabla[i as usize];
        if *celda != u32::MAX {
            return Err(ErrorSchematic::Invalido(
                "la paleta repite un índice".into(),
            ));
        }
        *celda = paleta.insertar(normalizar_estado(estado));
    }
    Ok(tabla)
}

/// Lee un varint (7 bits por byte, el bit alto indica que sigue otro) que
/// quepa en 32 bits. `None` si se acaba la entrada o el valor es demasiado largo.
fn siguiente_varint(bytes: &[u8], pos: &mut usize) -> Option<u32> {
    let mut valor: u32 = 0;
    for desp in (0..35).step_by(7) {
        let b = *bytes.get(*pos)?;
        *pos += 1;
        let parte = u32::from(b & 0x7f);
        if desp == 28 && parte > 0x0f {
            return None;
        }
        valor |= parte << desp;
        if b & 0x80 == 0 {
            return Some(valor);
        }
    }
    None
}

/// Decodifica una secuencia completa de varints (pública para los tests).
pub fn leer_varints(bytes: &[u8]) -> Option<Vec<u32>> {
    let mut pos = 0;
    let mut salida = Vec::new();
    while pos < bytes.len() {
        salida.push(siguiente_varint(bytes, &mut pos)?);
    }
    Some(salida)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn codificar(mut v: u32) -> Vec<u8> {
        let mut s = Vec::new();
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                s.push(b);
                return s;
            }
            s.push(b | 0x80);
        }
    }

    #[test]
    fn varints_de_uno_y_varios_bytes() {
        assert_eq!(leer_varints(&[0x00, 0x7f]), Some(vec![0, 127]));
        assert_eq!(leer_varints(&[0x80, 0x01]), Some(vec![128]));
        assert_eq!(leer_varints(&[0x86, 0x01, 0x05]), Some(vec![134, 5]));
        let valores = [0u32, 1, 127, 128, 300, 16_383, 16_384, 2_097_151, u32::MAX];
        let bytes: Vec<u8> = valores.iter().flat_map(|&v| codificar(v)).collect();
        assert_eq!(leer_varints(&bytes).unwrap(), valores);
    }

    #[test]
    fn varints_malos_no_hacen_panic() {
        assert_eq!(leer_varints(&[0x80]), None);
        assert_eq!(leer_varints(&[0xff, 0xff, 0xff, 0xff, 0x7f]), None);
        assert_eq!(leer_varints(&[0xff; 6]), None);
    }
}
