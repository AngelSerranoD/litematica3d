// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Sugerencias sobre el [`Catalogo`]: bloques parecidos a uno, familias
//! parecidas a otra y el plan para cambiar una familia entera de un
//! schematic por otra (una lista de [`Reemplazo`] que `aplicar_cambios`
//! consume tal cual). El contrato, en `docs/edicion.md` («Parecidos»).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Serialize;

use super::familias::{Miembro, Variante, base_otro, familia_de};
use super::forma::{FormaBloque, forma_por_sufijo};
use super::{Catalogo, InfoBloque, Oklab};
use crate::edicion::Reemplazo;
use crate::schematic::{Schematic, es_aire};

/// Peso de las formas que le faltan a la familia destino en
/// [`Catalogo::familias_parecidas`] (0 = todas las de la origen; 1 = ninguna).
pub const PESO_FORMAS: f64 = 0.20;
/// Peso de lo poco equivalentes que son sus variantes (0 = todas tienen
/// equivalente exacto).
pub const PESO_VARIANTES: f64 = 0.05;

/// Un bloque parecido a otro, listo para usar como destino.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parecido {
    /// `minecraft:polished_blackstone_brick_stairs`.
    pub id: String,
    /// Block state destino con las propiedades del origen que admite
    /// (`…[facing=east,half=top,shape=straight,waterlogged=false]`).
    pub estado: String,
    pub forma: FormaBloque,
    pub familia: String,
    /// Color del estado destino.
    pub color: [u8; 3],
    /// Distancia OKLab al color del estado origen.
    pub distancia: f64,
}

/// Familia y variante de un block state, con su forma.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FamiliaBloque {
    pub familia: String,
    /// Componentes de la variante (`["pulido", "ladrillo"]`; vacío = base).
    pub variante: Variante,
    /// `base` o `pulido+ladrillo`.
    pub nombre_variante: String,
    pub forma: FormaBloque,
    /// ¿Se puede sugerir como destino?
    pub sugerible: bool,
}

/// Una familia parecida a otra.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FamiliaParecida {
    pub familia: String,
    /// Color medio de sus miembros sugeribles.
    pub color: [u8; 3],
    /// Distancia OKLab entre los colores medios.
    pub distancia: f64,
    /// Formas de la familia origen que también tiene esta.
    pub formas_comunes: usize,
    /// De 0 a 1: lo equivalentes que son sus variantes a las de la origen.
    pub variantes_comunes: f64,
    /// Orden final (menor = más parecida): distancia + penalizaciones.
    pub puntuacion: f64,
    /// Miembros sugeribles.
    pub miembros: Vec<Miembro>,
}

/// Un block state presente en un schematic y cuántas posiciones ocupa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstadoPresente {
    pub estado: String,
    pub cantidad: u64,
}

/// Compara distancias (los NaN no aparecen: los colores son finitos).
fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// Media de varios colores en OKLab.
fn media(colores: &[Oklab]) -> Option<Oklab> {
    if colores.is_empty() {
        return None;
    }
    let n = colores.len() as f64;
    let suma = colores
        .iter()
        .fold((0.0, 0.0, 0.0), |s, c| (s.0 + c.l, s.1 + c.a, s.2 + c.b));
    Some(Oklab {
        l: suma.0 / n,
        a: suma.1 / n,
        b: suma.2 / n,
    })
}

/// Media sRGB de varios colores (para mostrar la familia).
fn media_rgb(colores: &[[u8; 3]]) -> [u8; 3] {
    let n = colores.len().max(1) as u32;
    let mut s = [0u32; 3];
    for c in colores {
        for i in 0..3 {
            s[i] += u32::from(c[i]);
        }
    }
    s.map(|v| (v / n) as u8)
}

impl Catalogo {
    /// Familias del catálogo: familia → miembros (todos, también los que no
    /// se sugieren), por id.
    pub fn familias(&self) -> &BTreeMap<String, Vec<Miembro>> {
        self.familias.get_or_init(|| {
            let mut f: BTreeMap<String, Vec<Miembro>> = BTreeMap::new();
            for b in self.bloques() {
                let p = familia_de(&b.id);
                f.entry(p.familia).or_default().push(Miembro {
                    id: b.id.clone(),
                    forma: b.forma,
                    variante: p.variante,
                    sugerible: b.sugerible,
                });
            }
            f
        })
    }

    /// Forma de un estado aunque el bloque no esté en el catálogo (por su
    /// sufijo; si no, `Otro`).
    fn forma_de_estado(&self, estado: &str) -> FormaBloque {
        self.bloque(estado)
            .map(|b| b.forma)
            .or_else(|| {
                let id = estado.split_once('[').map_or(estado, |(n, _)| n);
                forma_por_sufijo(id.rsplit(':').next().unwrap_or(id))
            })
            .unwrap_or(FormaBloque::Otro)
    }

    /// Familia, variante y forma de un block state.
    pub fn familia_bloque(&self, estado: &str) -> FamiliaBloque {
        let p = familia_de(estado);
        FamiliaBloque {
            familia: p.familia,
            nombre_variante: p.variante.nombre(),
            variante: p.variante,
            forma: self.forma_de_estado(estado),
            sugerible: self.bloque(estado).is_some_and(|b| b.sugerible),
        }
    }

    /// Estado destino de `info` tomando las propiedades de `estado_origen`, y
    /// su color.
    fn destino(&self, info: &InfoBloque, estado_origen: &str) -> Option<(String, [u8; 3])> {
        let estado = info.estado_desde(estado_origen);
        let rgb = self.color_estado(&estado)?;
        Some((estado, rgb))
    }

    /// Los `n` bloques sugeribles de la misma forma más cercanos en color
    /// OKLab a `estado` (sin el propio bloque), cada uno con su estado destino
    /// ya construido (conserva `facing`, `half`, `shape`, `type`, `axis`,
    /// `waterlogged`… si el destino las admite). Una losa solo sugiere losas,
    /// una escalera escaleras… Los de forma `Otro` solo sugieren bloques con
    /// su mismo «id base» ([`base_otro`]: `white_candle` → `red_candle`,
    /// `torch` → `soul_torch`); si no hay ninguno, nada. Vacío si el bloque no
    /// está en el catálogo o no tiene color.
    pub fn parecidos(&self, estado: &str, n: usize) -> Vec<Parecido> {
        let Some(info) = self.bloque(estado) else {
            return Vec::new();
        };
        let Some(origen) = self.oklab_estado(estado).or(info.oklab) else {
            return Vec::new();
        };
        let base = (info.forma == FormaBloque::Otro).then(|| base_otro(&info.id));
        let mut v: Vec<Parecido> = self
            .bloques()
            .filter(|b| {
                b.sugerible && b.forma == info.forma && b.id != info.id && b.oklab.is_some()
            })
            .filter(|b| base.as_ref().is_none_or(|base| base_otro(&b.id) == *base))
            .filter_map(|b| {
                let (estado, color) = self.destino(b, estado)?;
                Some(Parecido {
                    id: b.id.clone(),
                    distancia: origen.distancia(Oklab::desde_rgb(color)),
                    estado,
                    forma: b.forma,
                    familia: familia_de(&b.id).familia,
                    color,
                })
            })
            .collect();
        v.sort_by(|a, b| cmp_f64(a.distancia, b.distancia).then_with(|| a.id.cmp(&b.id)));
        v.truncate(n);
        v
    }

    /// Familias parecidas a `familia`, de más a menos: color medio de los
    /// miembros (OKLab) + [`PESO_FORMAS`] × formas que le faltan +
    /// [`PESO_VARIANTES`] × lo poco equivalentes que son sus variantes. Solo
    /// familias con algún miembro sugerible con color y alguna forma en común.
    /// Vacío si `familia` no está en el catálogo.
    pub fn familias_parecidas(&self, familia: &str, n: usize) -> Vec<FamiliaParecida> {
        let familias = self.familias();
        let Some(origen) = familias.get(familia) else {
            return Vec::new();
        };
        // Del origen cuentan los sugeribles; si no hay, todos.
        let utiles: Vec<&Miembro> = {
            let s: Vec<&Miembro> = origen.iter().filter(|m| m.sugerible).collect();
            if s.is_empty() {
                origen.iter().collect()
            } else {
                s
            }
        };
        let color = |m: &&Miembro| self.bloque(&m.id).and_then(|b| b.oklab);
        let Some(color_o) = media(&utiles.iter().filter_map(color).collect::<Vec<_>>()) else {
            return Vec::new();
        };
        let formas_o: BTreeSet<FormaBloque> = utiles.iter().map(|m| m.forma).collect();
        let variantes_o: BTreeSet<&Variante> = utiles.iter().map(|m| &m.variante).collect();

        let mut v: Vec<FamiliaParecida> = familias
            .iter()
            .filter(|(nombre, _)| nombre.as_str() != familia)
            .filter_map(|(nombre, miembros)| {
                let miembros: Vec<Miembro> = miembros
                    .iter()
                    .filter(|m| m.sugerible && color(m).is_some())
                    .cloned()
                    .collect();
                let rgbs: Vec<[u8; 3]> = miembros
                    .iter()
                    .filter_map(|m| self.bloque(&m.id)?.color)
                    .collect();
                let colores: Vec<Oklab> = rgbs.iter().map(|c| Oklab::desde_rgb(*c)).collect();
                let distancia = color_o.distancia(media(&colores)?);
                let formas: BTreeSet<FormaBloque> = miembros.iter().map(|m| m.forma).collect();
                let formas_comunes = formas_o.intersection(&formas).count();
                // Sin ninguna forma en común no sirve para cambiar nada.
                if formas_comunes == 0 {
                    return None;
                }
                let variantes_comunes = variantes_o
                    .iter()
                    .map(|vo| {
                        miembros
                            .iter()
                            .map(|m| vo.parecido(&m.variante))
                            .fold(0.0, f64::max)
                    })
                    .sum::<f64>()
                    / variantes_o.len() as f64;
                let faltan = 1.0 - formas_comunes as f64 / formas_o.len() as f64;
                Some(FamiliaParecida {
                    familia: nombre.clone(),
                    color: media_rgb(&rgbs),
                    distancia,
                    formas_comunes,
                    variantes_comunes,
                    puntuacion: distancia
                        + PESO_FORMAS * faltan
                        + PESO_VARIANTES * (1.0 - variantes_comunes),
                    miembros,
                })
            })
            .collect();
        v.sort_by(|a, b| {
            cmp_f64(a.puntuacion, b.puntuacion).then_with(|| a.familia.cmp(&b.familia))
        });
        v.truncate(n);
        v
    }

    /// Destino de un estado de otra familia dentro de la familia `destino`:
    /// 1. Entre sus miembros sugeribles de la misma forma, el de variante más
    ///    equivalente ([`Variante::coste`]: la exacta si existe;
    ///    `deepslate_brick_stairs` → `polished_blackstone_brick_stairs`);
    ///    a igual coste, el más cercano en color; a igual color, por id.
    /// 2. Si la familia destino no tiene esa forma: el bloque sugerible de esa
    ///    forma más parecido en color de todo el catálogo ([`Catalogo::parecidos`]).
    /// 3. Si tampoco hay (un `Otro` sin equivalentes): `None`.
    pub fn equivalente(&self, estado: &str, destino: &str) -> Option<String> {
        let forma = self.forma_de_estado(estado);
        let variante = familia_de(estado).variante;
        let color = self.oklab_estado(estado);
        let candidatos = self
            .familias()
            .get(destino)
            .into_iter()
            .flatten()
            .filter(|m| m.sugerible && m.forma == forma)
            .filter_map(|m| {
                let info = self.bloque(&m.id)?;
                let estado_d = info.estado_desde(estado);
                let d = match (color, self.oklab_estado(&estado_d)) {
                    (Some(a), Some(b)) => a.distancia(b),
                    _ => f64::INFINITY,
                };
                Some((variante.coste(&m.variante), d, estado_d))
            })
            .min_by(|a, b| {
                cmp_f64(a.0, b.0)
                    .then_with(|| cmp_f64(a.1, b.1))
                    .then_with(|| a.2.cmp(&b.2))
            });
        match candidatos {
            Some((_, _, e)) => Some(e),
            None => self.parecidos(estado, 1).pop().map(|p| p.estado),
        }
    }

    /// Plan para cambiar la familia `origen` por la `destino` en `s`: un
    /// [`Reemplazo`] por cada block state de la familia origen presente (el
    /// destino, el de [`Catalogo::equivalente`]). Los estados sin
    /// equivalente se quedan fuera, igual que los que no cambiarían. Por
    /// estado origen, para que sea estable.
    pub fn plan_familia(&self, s: &Schematic, origen: &str, destino: &str) -> Vec<Reemplazo> {
        miembros_en_schematic(s, origen)
            .into_iter()
            .filter_map(|p| {
                let d = self.equivalente(&p.estado, destino)?;
                (d != p.estado).then_some(Reemplazo {
                    estado_origen: p.estado,
                    estado_destino: d,
                })
            })
            .collect()
    }
}

/// Block states de `s` (todas las regiones, sin aire) con las posiciones
/// que ocupa cada uno.
pub fn estados_presentes(s: &Schematic) -> HashMap<&str, u64> {
    let mut cuenta: HashMap<&str, u64> = HashMap::new();
    for r in &s.regiones {
        let mut usos = vec![0u64; r.paleta.len()];
        for &i in &r.bloques {
            if let Some(u) = usos.get_mut(i as usize) {
                *u += 1;
            }
        }
        for (estado, u) in r.paleta.iter().zip(usos) {
            if u > 0 && !es_aire(estado) {
                *cuenta.entry(estado.as_str()).or_default() += u;
            }
        }
    }
    cuenta
}

/// Block states de la familia `familia` presentes en `s`, con cuántas
/// posiciones ocupa cada uno; ordenados por estado.
pub fn miembros_en_schematic(s: &Schematic, familia: &str) -> Vec<EstadoPresente> {
    let mut v: Vec<EstadoPresente> = estados_presentes(s)
        .into_iter()
        .filter(|(e, _)| familia_de(e).familia == familia)
        .map(|(e, cantidad)| EstadoPresente {
            estado: e.to_string(),
            cantidad,
        })
        .collect();
    v.sort_by(|a, b| a.estado.cmp(&b.estado));
    v
}
