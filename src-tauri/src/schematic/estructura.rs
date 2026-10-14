// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Formato de estructura de Minecraft (.nbt del Structure Block): lista de
//! bloques con posición; las posiciones que no aparecen son aire.

use serde::Deserialize;

use super::modelo::{Formato, Region, Schematic, caja_englobante};
use super::nbt::{EntradaPaleta, deserializar};
use super::{ErrorSchematic, Paleta, limitar_implicito, reservar_bloques, validar_tamano};

#[derive(Deserialize)]
struct Estructura {
    #[serde(rename = "DataVersion")]
    version_datos: Option<i32>,
    size: Vec<i32>,
    palette: Option<Vec<EntradaPaleta>>,
    /// Variantes aleatorias (naufragios…): se usa la primera.
    palettes: Option<Vec<Vec<EntradaPaleta>>>,
    blocks: Vec<BloqueNbt>,
}

#[derive(Deserialize)]
struct BloqueNbt {
    pos: Vec<i32>,
    state: i32,
}

pub(crate) fn leer(datos: &[u8]) -> Result<Schematic, ErrorSchematic> {
    let e: Estructura = deserializar(datos)?;
    let [ancho, alto, largo] = e.size[..] else {
        return Err(ErrorSchematic::Invalido(
            "el tamaño no tiene tres valores".into(),
        ));
    };
    let tamano = validar_tamano(ancho.into(), alto.into(), largo.into(), "la estructura")?;

    let paleta_nbt = match (e.palette, e.palettes) {
        (Some(p), _) => p,
        (None, Some(ps)) => ps.into_iter().next().unwrap_or_default(),
        (None, None) => Vec::new(),
    };
    if paleta_nbt.is_empty() && !e.blocks.is_empty() {
        return Err(ErrorSchematic::Invalido("la paleta está vacía".into()));
    }
    // El índice 0 es aire: las posiciones sin declarar lo usan.
    let mut paleta = Paleta::con_aire();
    let remapeo: Vec<u32> = paleta_nbt
        .iter()
        .map(|p| paleta.insertar(p.estado()))
        .collect();

    // La estructura es dispersa: lo no declarado es aire y no ocupa bytes.
    limitar_implicito(tamano.volumen(), e.blocks.len() as u64)?;
    let volumen = tamano.volumen() as usize;
    let mut bloques = reservar_bloques(volumen as u64)?;
    bloques.resize(volumen, 0);
    let fuera = || ErrorSchematic::Invalido("hay un bloque fuera de la estructura".into());
    for b in &e.blocks {
        let [x, y, z] = b.pos[..] else {
            return Err(ErrorSchematic::Invalido(
                "un bloque no tiene posición válida".into(),
            ));
        };
        let coord = |v: i32, lado: u32| u32::try_from(v).ok().filter(|&v| v < lado);
        let (x, y, z) = match (
            coord(x, tamano.ancho),
            coord(y, tamano.alto),
            coord(z, tamano.largo),
        ) {
            (Some(x), Some(y), Some(z)) => (x as usize, y as usize, z as usize),
            _ => return Err(fuera()),
        };
        let p = usize::try_from(b.state)
            .ok()
            .and_then(|s| remapeo.get(s))
            .ok_or_else(|| {
                ErrorSchematic::Invalido("usa un bloque que no está en su paleta".into())
            })?;
        let i = (y * tamano.largo as usize + z) * tamano.ancho as usize + x;
        bloques[i] = *p;
    }

    let regiones = vec![Region {
        nombre: "Principal".into(),
        origen: [0, 0, 0],
        tamano,
        paleta: paleta.en_vec(),
        bloques,
        // Las entidades y entidades de bloque de este formato no se conservan.
        extras: Default::default(),
    }];
    Ok(Schematic {
        formato: Formato::Estructura,
        nombre: None,
        autor: None,
        descripcion: None,
        version_datos: e.version_datos,
        fecha_creacion: None,
        dimensiones: caja_englobante(&regiones),
        regiones,
    })
}
