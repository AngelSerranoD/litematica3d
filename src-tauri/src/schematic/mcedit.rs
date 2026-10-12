// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Formato MCEdit / WorldEdit antiguo (.schematic, anterior a 1.13): IDs
//! numéricos y metadatos que se traducen a block states con una tabla.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use super::modelo::{Formato, Region, Schematic, caja_englobante};
use super::nbt::deserializar;
use super::{ErrorSchematic, Paleta, normalizar_estado, reservar_bloques, validar_tamano};

/// Tabla «id:meta» → block state, de PrismarineJS/minecraft-data (MIT).
static TABLA_LEGADO: LazyLock<HashMap<(u16, u8), String>> = LazyLock::new(|| {
    let texto = include_str!("../../datos/legado_bloques.json");
    let crudo: HashMap<String, String> = serde_json::from_str(texto).unwrap_or_default();
    crudo
        .into_iter()
        .filter_map(|(clave, estado)| {
            let (id, meta) = clave.split_once(':')?;
            Some((
                (id.parse().ok()?, meta.parse().ok()?),
                normalizar_estado(&estado),
            ))
        })
        .collect()
});

#[derive(Deserialize)]
struct Mcedit {
    #[serde(rename = "Width")]
    ancho: i32,
    #[serde(rename = "Height")]
    alto: i32,
    #[serde(rename = "Length")]
    largo: i32,
    #[serde(rename = "Blocks")]
    bloques: fastnbt::ByteArray,
    #[serde(rename = "Data")]
    datos: Option<fastnbt::ByteArray>,
    #[serde(rename = "AddBlocks")]
    anadidos: Option<fastnbt::ByteArray>,
}

/// Block state de un ID y metadato antiguos.
pub(crate) fn estado_legado(id: u16, meta: u8) -> String {
    let tabla = &*TABLA_LEGADO;
    tabla
        .get(&(id, meta))
        .or_else(|| tabla.get(&(id, 0)))
        .cloned()
        .unwrap_or_else(|| format!("litematica3d:desconocido_{id}_{meta}"))
}

fn lado_sin_signo(v: i32) -> i64 {
    if (-32768..0).contains(&v) {
        i64::from(v) + 65536
    } else {
        i64::from(v)
    }
}

pub(crate) fn leer(datos: &[u8]) -> Result<Schematic, ErrorSchematic> {
    let m: Mcedit = deserializar(datos)?;
    let tamano = validar_tamano(
        lado_sin_signo(m.ancho),
        lado_sin_signo(m.alto),
        lado_sin_signo(m.largo),
        "el schematic",
    )?;
    let volumen = tamano.volumen() as usize;
    let ids: &[i8] = &m.bloques;
    if ids.len() < volumen {
        return Err(ErrorSchematic::Invalido(format!(
            "faltan datos de bloques ({} de {volumen})",
            ids.len()
        )));
    }
    let metas: &[i8] = m.datos.as_deref().unwrap_or(&[]);
    if !metas.is_empty() && metas.len() < volumen {
        return Err(ErrorSchematic::Invalido(
            "faltan metadatos de bloques".into(),
        ));
    }
    let anadidos: &[i8] = m.anadidos.as_deref().unwrap_or(&[]);
    if !anadidos.is_empty() && anadidos.len() < volumen.div_ceil(2) {
        return Err(ErrorSchematic::Invalido("faltan datos de AddBlocks".into()));
    }

    let mut paleta = Paleta::nueva();
    // Caché (id << 4 | meta) → índice en la paleta: 4096 IDs × 16 metas.
    let mut cache = vec![u32::MAX; 4096 * 16];
    let mut bloques = reservar_bloques(volumen as u64)?;
    for (i, &id_bajo) in ids.iter().enumerate().take(volumen) {
        let bajo = u16::from(id_bajo as u8);
        let alto = match anadidos.get(i >> 1) {
            Some(&a) if i % 2 == 0 => u16::from(a as u8 & 0x0f),
            Some(&a) => u16::from((a as u8 & 0xf0) >> 4),
            None => 0,
        };
        let id = (alto << 8) | bajo;
        let meta = metas.get(i).map_or(0, |&d| d as u8 & 0x0f);
        let clave = (usize::from(id) << 4) | usize::from(meta);
        let mut p = cache[clave];
        if p == u32::MAX {
            p = paleta.insertar(estado_legado(id, meta));
            cache[clave] = p;
        }
        bloques.push(p);
    }

    let regiones = vec![Region {
        nombre: "Principal".into(),
        origen: [0, 0, 0],
        tamano,
        paleta: paleta.en_vec(),
        bloques,
    }];
    Ok(Schematic {
        formato: Formato::Mcedit,
        nombre: None,
        autor: None,
        descripcion: None,
        version_datos: None,
        fecha_creacion: None,
        dimensiones: caja_englobante(&regiones),
        regiones,
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn tabla_de_legado() {
        assert_eq!(estado_legado(0, 0), "minecraft:air");
        assert_eq!(estado_legado(1, 0), "minecraft:stone");
        // Meta desconocido: se usa el meta 0 del mismo ID.
        assert_eq!(estado_legado(1, 15), "minecraft:stone");
        assert_eq!(estado_legado(4095, 5), "litematica3d:desconocido_4095_5");
    }
}
