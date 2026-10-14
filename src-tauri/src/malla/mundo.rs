// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! «Mundo»: todas las regiones del schematic combinadas en una sola rejilla de
//! chunks de 16³ con una paleta común. Es lo que se malla y lo que se consulta
//! al seleccionar un bloque.
//!
//! Coordenadas del modelo: relativas a la esquina mínima de la caja que
//! engloba todas las regiones (`origen`), así que siempre son ≥ 0. Son las
//! mismas que usan los vértices de la malla.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use super::apariencia::nombre_de;
use crate::schematic::{Schematic, es_aire};

/// Lado de un chunk, en bloques.
pub const LADO: i32 = 16;
/// Posiciones de un chunk.
pub const VOLUMEN: usize = 16 * 16 * 16;

/// Índice dentro de un chunk: mismo orden que las regiones, `(y*16 + z)*16 + x`.
#[inline]
pub fn indice_local(x: usize, y: usize, z: usize) -> usize {
    (y * 16 + z) * 16 + x
}

/// Caja de una región en coordenadas del modelo (para decir a cuál pertenece
/// un bloque al seleccionarlo).
#[derive(Debug, Clone)]
struct CajaRegion {
    nombre: String,
    min: [i64; 3],
    max: [i64; 3],
}

/// Schematic combinado y troceado en chunks.
#[derive(Debug)]
pub struct Mundo {
    /// Esquina mínima de la caja englobante, en coordenadas del schematic.
    pub origen: [i32; 3],
    /// Ancho, alto y largo de la caja englobante.
    pub tamano: [u32; 3],
    /// Paleta común. El índice 0 es siempre `minecraft:air`.
    pub paleta: Vec<String>,
    /// Chunks con algún bloque, por coordenada de chunk (modelo / 16).
    chunks: HashMap<[i32; 3], Box<[u32]>>,
    regiones: Vec<CajaRegion>,
    total_bloques: u64,
}

/// Lo que se devuelve al seleccionar un bloque.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BloqueSeleccionado {
    /// Block state completo: `minecraft:oak_stairs[facing=north,half=bottom]`.
    pub estado: String,
    /// Sin propiedades: `minecraft:oak_stairs`.
    pub nombre: String,
    /// Propiedades ordenadas por clave.
    pub propiedades: BTreeMap<String, String>,
    /// Posición en coordenadas del schematic (las del juego relativas al
    /// origen del archivo).
    pub posicion: [i32; 3],
    /// Región que contiene el bloque (la última, si se solapan).
    pub region: Option<String>,
}

impl Mundo {
    /// Combina las regiones. Si dos regiones se solapan, gana el bloque no
    /// aire de la que va después. Error (texto para el usuario) si las
    /// coordenadas no caben en i32.
    pub fn desde_schematic(s: &Schematic) -> Result<Self, String> {
        let mut paleta = vec!["minecraft:air".to_string()];
        let mut indices: HashMap<&str, u32> = HashMap::new();
        let mut min = [i64::MAX; 3];
        let mut max = [i64::MIN; 3];
        for r in s.regiones.iter().filter(|r| r.tamano.volumen() > 0) {
            let t = [r.tamano.ancho, r.tamano.alto, r.tamano.largo];
            for e in 0..3 {
                min[e] = min[e].min(i64::from(r.origen[e]));
                max[e] = max[e].max(i64::from(r.origen[e]) + i64::from(t[e]));
            }
        }
        if min[0] > max[0] {
            min = [0; 3];
            max = [0; 3];
        }
        let fuera = || "las regiones están demasiado lejos entre sí".to_string();
        let mut tamano = [0u32; 3];
        let mut origen = [0i32; 3];
        for e in 0..3 {
            tamano[e] = u32::try_from(max[e] - min[e]).map_err(|_| fuera())?;
            if max[e] - min[e] > i64::from(i32::MAX) - 16 {
                return Err(fuera());
            }
            origen[e] = i32::try_from(min[e]).map_err(|_| fuera())?;
        }

        let mut chunks: HashMap<[i32; 3], Box<[u32]>> = HashMap::new();
        let mut regiones = Vec::with_capacity(s.regiones.len());
        for r in &s.regiones {
            let t = [r.tamano.ancho, r.tamano.alto, r.tamano.largo].map(i64::from);
            let rel: [i64; 3] = std::array::from_fn(|e| i64::from(r.origen[e]) - min[e]);
            regiones.push(CajaRegion {
                nombre: r.nombre.clone(),
                min: rel,
                max: std::array::from_fn(|e| rel[e] + t[e]),
            });
            if t.contains(&0) {
                continue;
            }
            // Paleta de la región → paleta común (aire → 0).
            let remap: Vec<u32> = r
                .paleta
                .iter()
                .map(|estado| {
                    if es_aire(estado) {
                        return 0;
                    }
                    *indices.entry(estado.as_str()).or_insert_with(|| {
                        paleta.push(estado.clone());
                        (paleta.len() - 1) as u32
                    })
                })
                .collect();

            // Recorre los chunks que toca la región y copia la intersección.
            let c0: [i64; 3] = std::array::from_fn(|e| rel[e] >> 4);
            let c1: [i64; 3] = std::array::from_fn(|e| (rel[e] + t[e] - 1) >> 4);
            let (ancho, largo) = (t[0] as usize, t[2] as usize);
            for cy in c0[1]..=c1[1] {
                for cz in c0[2]..=c1[2] {
                    for cx in c0[0]..=c1[0] {
                        let c = [cx, cy, cz];
                        let lo: [i64; 3] = std::array::from_fn(|e| (c[e] * 16).max(rel[e]));
                        let hi: [i64; 3] =
                            std::array::from_fn(|e| (c[e] * 16 + 16).min(rel[e] + t[e]));
                        let clave = [cx as i32, cy as i32, cz as i32];
                        // El chunk se reserva al escribir su primer bloque no aire:
                        // un archivo con regiones enormes y vacías no pide memoria.
                        let mut chunk = chunks.remove(&clave);
                        for y in lo[1]..hi[1] {
                            for z in lo[2]..hi[2] {
                                let ry = (y - rel[1]) as usize;
                                let rz = (z - rel[2]) as usize;
                                let base_r = (ry * largo + rz) * ancho;
                                let ly = (y - c[1] * 16) as usize;
                                let lz = (z - c[2] * 16) as usize;
                                for x in lo[0]..hi[0] {
                                    let i = base_r + (x - rel[0]) as usize;
                                    let p = r
                                        .bloques
                                        .get(i)
                                        .and_then(|&p| remap.get(p as usize))
                                        .copied()
                                        .unwrap_or(0);
                                    if p != 0 {
                                        let lx = (x - c[0] * 16) as usize;
                                        let chunk = chunk.get_or_insert_with(|| {
                                            vec![0u32; VOLUMEN].into_boxed_slice()
                                        });
                                        chunk[indice_local(lx, ly, lz)] = p;
                                    }
                                }
                            }
                        }
                        if let Some(chunk) = chunk {
                            chunks.insert(clave, chunk);
                        }
                    }
                }
            }
        }
        let total_bloques = chunks
            .values()
            .map(|c| c.iter().filter(|&&p| p != 0).count() as u64)
            .sum();
        Ok(Mundo {
            origen,
            tamano,
            paleta,
            chunks,
            regiones,
            total_bloques,
        })
    }

    /// Posiciones que no son aire.
    pub fn total_bloques(&self) -> u64 {
        self.total_bloques
    }

    /// Coordenadas de los chunks con bloques, ordenadas por (y, z, x).
    pub fn claves_chunks(&self) -> Vec<[i32; 3]> {
        let mut v: Vec<[i32; 3]> = self.chunks.keys().copied().collect();
        v.sort_unstable_by_key(|c| (c[1], c[2], c[0]));
        v
    }

    /// Datos de un chunk (índices de la paleta común), si tiene bloques.
    pub fn chunk(&self, clave: [i32; 3]) -> Option<&[u32]> {
        self.chunks.get(&clave).map(|c| &c[..])
    }

    /// Índice de paleta en una posición del modelo (0 = aire o fuera).
    pub fn indice_en(&self, x: i32, y: i32, z: i32) -> u32 {
        let (Some(c), lx, ly, lz) = (
            self.chunks
                .get(&[x.div_euclid(LADO), y.div_euclid(LADO), z.div_euclid(LADO)]),
            x.rem_euclid(LADO) as usize,
            y.rem_euclid(LADO) as usize,
            z.rem_euclid(LADO) as usize,
        ) else {
            return 0;
        };
        c[indice_local(lx, ly, lz)]
    }

    /// Bloque en una posición del modelo, o `None` si es aire o está fuera.
    pub fn bloque_en(&self, x: i32, y: i32, z: i32) -> Option<BloqueSeleccionado> {
        let p = self.indice_en(x, y, z);
        if p == 0 {
            return None;
        }
        let estado = self.paleta.get(p as usize)?.clone();
        let propiedades = estado
            .split_once('[')
            .map(|(_, resto)| {
                resto
                    .strip_suffix(']')
                    .unwrap_or(resto)
                    .split(',')
                    .filter_map(|par| par.split_once('='))
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let pos = [i64::from(x), i64::from(y), i64::from(z)];
        let region = self
            .regiones
            .iter()
            .rev()
            .find(|r| (0..3).all(|e| r.min[e] <= pos[e] && pos[e] < r.max[e]))
            .map(|r| r.nombre.clone());
        Some(BloqueSeleccionado {
            nombre: nombre_de(&estado).to_string(),
            posicion: [
                self.origen[0].saturating_add(x),
                self.origen[1].saturating_add(y),
                self.origen[2].saturating_add(z),
            ],
            estado,
            propiedades,
            region,
        })
    }
}
