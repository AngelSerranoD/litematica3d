// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Formato .litematic (mod Litematica): varias regiones, cada una con su paleta
//! y los índices empaquetados en un LongArray sin huecos entre longs.

use serde::Deserialize;

use super::modelo::{Formato, Region, Schematic, caja_englobante};
use super::nbt::{EntradaPaleta, MapaOrdenado, deserializar};
use super::{ErrorSchematic, MAX_POSICIONES, Paleta, reservar_bloques, texto_util, validar_tamano};

#[derive(Deserialize)]
struct Archivo {
    #[serde(rename = "MinecraftDataVersion")]
    version_datos: Option<i32>,
    #[serde(rename = "Metadata")]
    metadatos: Option<Metadatos>,
    #[serde(rename = "Regions")]
    regiones: MapaOrdenado<RegionNbt>,
}

#[derive(Deserialize)]
struct Metadatos {
    #[serde(rename = "Name")]
    nombre: Option<String>,
    #[serde(rename = "Author")]
    autor: Option<String>,
    #[serde(rename = "Description")]
    descripcion: Option<String>,
    #[serde(rename = "TimeCreated")]
    creado: Option<i64>,
}

#[derive(Deserialize)]
struct Vector {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Deserialize)]
struct RegionNbt {
    #[serde(rename = "Position")]
    posicion: Vector,
    #[serde(rename = "Size")]
    tamano: Vector,
    #[serde(rename = "BlockStatePalette")]
    paleta: Vec<EntradaPaleta>,
    #[serde(rename = "BlockStates")]
    estados: fastnbt::LongArray,
}

pub(crate) fn leer(datos: &[u8]) -> Result<Schematic, ErrorSchematic> {
    let archivo: Archivo = deserializar(datos)?;
    let mut regiones = Vec::with_capacity(archivo.regiones.0.len());
    let mut acumulado: u64 = 0;
    for (nombre, r) in archivo.regiones.0 {
        let region = leer_region(nombre, r)?;
        acumulado += region.tamano.volumen();
        if acumulado > MAX_POSICIONES {
            return Err(ErrorSchematic::DemasiadoGrande(format!(
                "sus regiones suman más de {MAX_POSICIONES} posiciones"
            )));
        }
        regiones.push(region);
    }
    let meta = archivo.metadatos;
    let (nombre, autor, descripcion, creado) = match meta {
        Some(m) => (m.nombre, m.autor, m.descripcion, m.creado),
        None => (None, None, None, None),
    };
    Ok(Schematic {
        formato: Formato::Litematic,
        nombre: texto_util(nombre),
        autor: texto_util(autor),
        descripcion: texto_util(descripcion),
        version_datos: archivo.version_datos,
        fecha_creacion: creado.filter(|&t| t > 0),
        dimensiones: caja_englobante(&regiones),
        regiones,
    })
}

fn leer_region(nombre: String, r: RegionNbt) -> Result<Region, ErrorSchematic> {
    let que = format!("la región «{nombre}»");
    let t = &r.tamano;
    let tamano = validar_tamano(
        i64::from(t.x).abs(),
        i64::from(t.y).abs(),
        i64::from(t.z).abs(),
        &que,
    )?;
    // Con tamaño negativo, la posición es la esquina máxima de ese eje.
    let esquina = |p: i32, s: i32| -> Result<i32, ErrorSchematic> {
        let v = if s < 0 {
            i64::from(p) + i64::from(s) + 1
        } else {
            i64::from(p)
        };
        i32::try_from(v)
            .map_err(|_| ErrorSchematic::Invalido(format!("{que} está fuera del mundo")))
    };
    let origen = [
        esquina(r.posicion.x, t.x)?,
        esquina(r.posicion.y, t.y)?,
        esquina(r.posicion.z, t.z)?,
    ];

    if r.paleta.is_empty() {
        return Err(ErrorSchematic::Invalido(format!(
            "{que} tiene la paleta vacía"
        )));
    }
    // La paleta del archivo puede tener duplicados tras normalizar: se remapea.
    let mut paleta = Paleta::nueva();
    let remapeo: Vec<u32> = r
        .paleta
        .iter()
        .map(|e| paleta.insertar(e.estado()))
        .collect();

    let bits = bits_por_bloque(r.paleta.len());
    let volumen = tamano.volumen();
    let longs: &[i64] = &r.estados;
    let necesarios = (volumen * u64::from(bits)).div_ceil(64);
    if (longs.len() as u64) < necesarios {
        return Err(ErrorSchematic::Invalido(format!(
            "a {que} le faltan datos de bloques ({} de {necesarios})",
            longs.len()
        )));
    }

    let mut bloques = reservar_bloques(volumen)?;
    for i in 0..volumen as usize {
        let v = valor_empaquetado(longs, bits, i).unwrap_or(u64::MAX);
        let p = usize::try_from(v)
            .ok()
            .and_then(|v| remapeo.get(v))
            .ok_or_else(|| {
                ErrorSchematic::Invalido(format!("{que} usa un bloque que no está en su paleta"))
            })?;
        bloques.push(*p);
    }

    Ok(Region {
        nombre,
        origen,
        tamano,
        paleta: paleta.en_vec(),
        bloques,
    })
}

/// Bits por entrada: `max(2, ceil(log2(n)))`.
pub(crate) fn bits_por_bloque(n: usize) -> u32 {
    let log = if n <= 1 {
        0
    } else {
        usize::BITS - (n - 1).leading_zeros()
    };
    log.max(2)
}

/// Valor `indice` de un array empaquetado como `LitematicaBitArray`: los
/// valores van seguidos y uno puede repartirse entre dos longs consecutivos.
/// Devuelve `None` si el array se queda corto.
pub(crate) fn valor_empaquetado(longs: &[i64], bits: u32, indice: usize) -> Option<u64> {
    if bits == 0 || bits > 64 {
        return None;
    }
    let mascara: u64 = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    let inicio = (indice as u64).checked_mul(u64::from(bits))?;
    let fin = inicio + u64::from(bits) - 1;
    let s = usize::try_from(inicio >> 6).ok()?;
    let desp = (inicio & 63) as u32;
    let bajo = *longs.get(s)? as u64;
    if (inicio >> 6) == (fin >> 6) {
        Some((bajo >> desp) & mascara)
    } else {
        let alto = *longs.get(s + 1)? as u64;
        Some(((bajo >> desp) | (alto << (64 - desp))) & mascara)
    }
}

/// Desempaqueta `cantidad` valores (pública para los tests).
pub fn leer_empaquetado(longs: &[i64], bits: u32, cantidad: usize) -> Option<Vec<u64>> {
    (0..cantidad)
        .map(|i| valor_empaquetado(longs, bits, i))
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Empaquetado de referencia, escrito de otra forma (bit a bit).
    fn empaquetar(valores: &[u64], bits: u32) -> Vec<i64> {
        let total = (valores.len() as u64 * u64::from(bits)).div_ceil(64) as usize;
        let mut longs = vec![0u64; total];
        for (i, &v) in valores.iter().enumerate() {
            for b in 0..bits as usize {
                if (v >> b) & 1 == 1 {
                    let pos = i * bits as usize + b;
                    longs[pos / 64] |= 1 << (pos % 64);
                }
            }
        }
        longs.into_iter().map(|v| v as i64).collect()
    }

    #[test]
    fn bits_minimos() {
        assert_eq!(bits_por_bloque(1), 2);
        assert_eq!(bits_por_bloque(4), 2);
        assert_eq!(bits_por_bloque(5), 3);
        assert_eq!(bits_por_bloque(20), 5);
        assert_eq!(bits_por_bloque(256), 8);
        assert_eq!(bits_por_bloque(257), 9);
    }

    #[test]
    fn valores_que_cruzan_longs() {
        for bits in [2u32, 3, 5, 7, 13, 31, 33, 64] {
            let tope = if bits == 64 {
                u64::MAX
            } else {
                (1u64 << bits) - 1
            };
            let valores: Vec<u64> = (0..200u64)
                .map(|i| (i.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i << 7)) & tope)
                .collect();
            let longs = empaquetar(&valores, bits);
            assert_eq!(
                leer_empaquetado(&longs, bits, valores.len()).unwrap(),
                valores,
                "bits={bits}"
            );
        }
    }

    #[test]
    fn valor_partido_entre_dos_longs() {
        // Con 5 bits, el índice 12 ocupa los bits 60..65: 4 en el primer long y 1 en el segundo.
        let longs = empaquetar(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0b10111], 5);
        assert_eq!(longs.len(), 2);
        assert_eq!(valor_empaquetado(&longs, 5, 12), Some(0b10111));
    }

    #[test]
    fn array_corto_no_hace_panic() {
        assert_eq!(valor_empaquetado(&[0], 5, 12), None);
        assert_eq!(valor_empaquetado(&[], 2, 0), None);
    }
}
