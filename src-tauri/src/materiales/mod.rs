// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Lista de materiales de un schematic: cuántos ítems de cada tipo hacen falta
//! para construirlo, desglosados en stacks y cajas de shulker.
//!
//! Es lógica pura (sin Tauri): recibe un `Schematic` ya cargado.
//!
//! Filtro de altura: `y_min` e `y_max` son coordenadas Y del schematic, las
//! mismas en las que está `Region::origen` (origen de la región + y local),
//! y el rango es inclusivo. Así el filtro significa lo mismo con varias
//! regiones a distinta altura.
//!
//! `capas` es el mismo rango pero en coordenadas del modelo (las del visor y
//! del parámetro `capas` del comando `malla`): Y del modelo 0 = la capa más
//! baja de la caja que engloba todas las regiones. Se convierte sumando
//! [`origen_modelo_y`]. Si llegan los dos filtros, cuenta la intersección.

pub mod reglas;

#[cfg(test)]
mod pruebas;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::ErrorApp;
use crate::schematic::{Region, Schematic};
use reglas::{Reglas, Resolucion};

/// Ítem con el que se agrupan los IDs antiguos sin traducción.
pub const ITEM_DESCONOCIDO: &str = "schematiclab:desconocido";

/// Huecos de una caja de shulker.
const HUECOS_SHULKER: u64 = 27;

/// Qué contar.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Opciones {
    /// Contar el agua y la lava (solo fuentes) como cubos.
    pub incluir_fluidos: bool,
    /// Solo la región con este nombre; `None`, todas.
    pub region: Option<String>,
    /// Altura mínima (inclusiva), en coordenadas del schematic.
    pub y_min: Option<i32>,
    /// Altura máxima (inclusiva), en coordenadas del schematic.
    pub y_max: Option<i32>,
    /// `[desde, hasta]` inclusivo en coordenadas del modelo (las del visor).
    pub capas: Option<[i32; 2]>,
}

/// Y del schematic que corresponde a la capa 0 del modelo: la mínima de las
/// regiones con volumen (0 si no hay ninguna). Es la misma cuenta que hace
/// `malla::Mundo::desde_schematic` para su `origen`.
pub fn origen_modelo_y(schem: &Schematic) -> i32 {
    schem
        .regiones
        .iter()
        .filter(|r| r.tamano.volumen() > 0)
        .map(|r| r.origen[1])
        .min()
        .unwrap_or(0)
}

/// Rango Y efectivo en coordenadas del schematic (`None` = sin límite).
fn rango_y(schem: &Schematic, opciones: &Opciones) -> (Option<i64>, Option<i64>) {
    let mut desde = opciones.y_min.map(i64::from);
    let mut hasta = opciones.y_max.map(i64::from);
    if let Some([a, b]) = opciones.capas {
        let base = i64::from(origen_modelo_y(schem));
        let (a, b) = (i64::from(a) + base, i64::from(b) + base);
        desde = Some(desde.map_or(a, |d| d.max(a)));
        hasta = Some(hasta.map_or(b, |h| h.min(b)));
    }
    (desde, hasta)
}

/// Un ítem de la lista con su desglose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    pub item: String,
    pub cantidad: u64,
    pub tamano_stack: u32,
    /// Stacks completos y lo que sobra (`stacks × tamano_stack + sueltos`).
    pub stacks: u64,
    pub sueltos: u64,
    /// Cajas de shulker llenas (27 stacks) y lo que queda fuera de ellas.
    pub shulkers: u64,
    pub stacks_en_shulker: u64,
    pub sueltos_en_shulker: u64,
}

/// Resultado del comando `materiales`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListaMateriales {
    /// Ordenada por cantidad (de más a menos) y, a igualdad, por ítem.
    pub materiales: Vec<Material>,
    /// Suma de todas las cantidades.
    pub total_items: u64,
    /// Huecos de inventario necesarios (cada ítem redondea su último stack).
    pub total_stacks: u64,
    /// Cajas de shulker para llevarlo todo mezclado: huecos / 27, hacia arriba.
    pub total_shulkers: u64,
    /// Bloques con ID antiguo sin traducción (también van en la lista como
    /// `schematiclab:desconocido`).
    pub desconocidos: u64,
}

/// Desglosa una cantidad en stacks y cajas de shulker.
pub fn desglosar(item: String, cantidad: u64, tamano_stack: u32) -> Material {
    let t = u64::from(tamano_stack.max(1));
    let caja = HUECOS_SHULKER * t;
    Material {
        item,
        cantidad,
        tamano_stack: tamano_stack.max(1),
        stacks: cantidad / t,
        sueltos: cantidad % t,
        shulkers: cantidad / caja,
        stacks_en_shulker: (cantidad % caja) / t,
        sueltos_en_shulker: cantidad % t,
    }
}

/// Lista de materiales con las reglas incrustadas en la aplicación.
pub fn calcular(schem: &Schematic, opciones: &Opciones) -> Result<ListaMateriales, ErrorApp> {
    let reglas = reglas::globales().map_err(|e| ErrorApp::Interno(e.to_string()))?;
    calcular_con(reglas, schem, opciones)
}

/// Lista de materiales con unas reglas concretas.
pub fn calcular_con(
    reglas: &Reglas,
    schem: &Schematic,
    opciones: &Opciones,
) -> Result<ListaMateriales, ErrorApp> {
    let regiones: Vec<&Region> = match &opciones.region {
        None => schem.regiones.iter().collect(),
        Some(nombre) => {
            let v: Vec<&Region> = schem
                .regiones
                .iter()
                .filter(|r| &r.nombre == nombre)
                .collect();
            if v.is_empty() {
                return Err(ErrorApp::RegionNoEncontrada {
                    nombre: nombre.clone(),
                });
            }
            v
        }
    };

    // La resolución se hace una vez por block state distinto, no por bloque.
    let mut cache: HashMap<&str, Resolucion> = HashMap::new();
    let mut cuentas: HashMap<String, u64> = HashMap::new();
    let mut desconocidos: u64 = 0;
    let (y_min, y_max) = rango_y(schem, opciones);
    for region in regiones {
        let por_paleta = contar_paleta(region, y_min, y_max);
        for (estado, &veces) in region.paleta.iter().zip(&por_paleta) {
            if veces == 0 {
                continue;
            }
            let res = cache
                .entry(estado.as_str())
                .or_insert_with(|| reglas.resolver(estado, opciones.incluir_fluidos));
            match res {
                Resolucion::Nada => {}
                Resolucion::Desconocido => desconocidos += veces,
                Resolucion::Items(items) => {
                    for (item, n) in items {
                        let c = cuentas.entry(item.clone()).or_default();
                        *c = c.saturating_add(veces.saturating_mul(*n));
                    }
                }
            }
        }
    }
    if desconocidos > 0 {
        *cuentas.entry(ITEM_DESCONOCIDO.to_string()).or_default() += desconocidos;
    }

    let mut materiales: Vec<Material> = cuentas
        .into_iter()
        .map(|(item, cantidad)| {
            let t = reglas.tamano_stack(&item);
            desglosar(item, cantidad, t)
        })
        .collect();
    materiales.sort_by(|a, b| {
        b.cantidad
            .cmp(&a.cantidad)
            .then_with(|| a.item.cmp(&b.item))
    });

    let total_items = materiales.iter().map(|m| m.cantidad).sum();
    let total_stacks: u64 = materiales
        .iter()
        .map(|m| m.stacks + u64::from(m.sueltos > 0))
        .sum();
    Ok(ListaMateriales {
        materiales,
        total_items,
        total_stacks,
        total_shulkers: total_stacks.div_ceil(HUECOS_SHULKER),
        desconocidos,
    })
}

/// Cuántas posiciones de la región usan cada entrada de la paleta, dentro del
/// rango de alturas. Las capas Y son tramos contiguos de `bloques`.
fn contar_paleta(region: &Region, y_min: Option<i64>, y_max: Option<i64>) -> Vec<u64> {
    let mut cuentas = vec![0u64; region.paleta.len()];
    let t = &region.tamano;
    let capa = u64::from(t.ancho) * u64::from(t.largo);
    let base = i64::from(region.origen[1]);
    let alto = i64::from(t.alto);
    let desde = y_min.map_or(0, |y| (y - base).max(0));
    let hasta = y_max.map_or(alto - 1, |y| (y - base).min(alto - 1));
    if capa == 0 || desde > hasta {
        return cuentas;
    }
    // desde >= 0 y hasta < alto, así que caben en u64.
    let limite = region.bloques.len() as u64;
    let inicio = (desde as u64).saturating_mul(capa).min(limite) as usize;
    let fin = ((hasta as u64) + 1).saturating_mul(capa).min(limite) as usize;
    for &p in &region.bloques[inicio..fin] {
        if let Some(c) = cuentas.get_mut(p as usize) {
            *c += 1;
        }
    }
    cuentas
}
