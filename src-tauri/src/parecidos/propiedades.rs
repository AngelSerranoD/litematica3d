// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Propiedades que admite cada bloque y su valor por defecto, para construir
//! el estado destino de un cambio conservando `facing`, `half`, `shape`,
//! `type`, `axis`, `waterlogged`…
//!
//! **De dónde salen**:
//! - Con jar, del blockstate: las claves de `variants` (`facing=east,half=top`)
//!   y las condiciones `when` de `multipart` (también dentro de `OR`/`AND`,
//!   con valores alternativos `a|b`). En `multipart` solo aparecen los
//!   valores que pintan algo: si una propiedad solo muestra `true` se le
//!   añade `false`, y si solo muestra `low`/`tall`, `none`.
//! - Además, la plantilla de su forma ([`plantilla`]): las propiedades que el
//!   juego tiene pero que no cambian el modelo y por eso no salen en el
//!   blockstate (sobre todo `waterlogged`; también `powered` en puertas y
//!   trampillas). Sin jar, la plantilla es todo lo que se sabe.
//!
//! **Valor por defecto**: los blockstates no lo dicen. Se usa la tabla
//! [`DEFECTOS`] (los defectos reales del juego para las propiedades
//! habituales: `facing=north`, `half=bottom`/`lower`, `type=bottom`,
//! `shape=straight`, `axis=y`, `face=wall`, `hinge=left`, lados de muros
//! `none`, booleanos `false`, salvo `up=true` en muros); si ninguno de sus
//! candidatos es un valor admitido, `false` si lo admite; si no, el primero
//! de la lista de valores. Los valores van en el orden en que aparecen, salvo
//! los numéricos, que se ordenan de menor a mayor (así el primero es el
//! menor: `layers=1`, `age=0`).

use serde::Serialize;
use serde_json::Value;

use super::forma::FormaBloque;

/// Una propiedad de un bloque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Propiedad {
    pub nombre: String,
    /// Valores admitidos.
    pub valores: Vec<String>,
    /// Valor por defecto (siempre uno de `valores`).
    pub defecto: String,
}

impl Propiedad {
    pub fn admite(&self, valor: &str) -> bool {
        self.valores.iter().any(|v| v == valor)
    }
}

/// Candidatos a valor por defecto de cada propiedad, en orden de preferencia.
pub const DEFECTOS: &[(&str, &[&str])] = &[
    ("facing", &["north", "down"]),
    ("horizontal_facing", &["north"]),
    ("half", &["bottom", "lower"]),
    ("type", &["bottom", "single", "normal"]),
    ("shape", &["straight", "north_south"]),
    ("axis", &["y"]),
    ("face", &["wall"]),
    ("hinge", &["left"]),
    ("attachment", &["floor"]),
    ("orientation", &["north_up"]),
    ("north", &["none", "false"]),
    ("south", &["none", "false"]),
    ("east", &["none", "false"]),
    ("west", &["none", "false"]),
    ("down", &["false"]),
    ("mode", &["compare", "save"]),
];

/// Valor por defecto de `nombre` entre `valores` (ver el módulo). `forma`
/// solo importa para `up`: en los muros es `true` (el poste central).
pub fn defecto(nombre: &str, valores: &[String], forma: FormaBloque) -> String {
    let admite = |v: &str| valores.iter().any(|x| x == v);
    let candidatos: &[&str] = if nombre == "up" {
        if forma == FormaBloque::Muro {
            &["true"]
        } else {
            &["false"]
        }
    } else {
        DEFECTOS
            .iter()
            .find(|(n, _)| *n == nombre)
            .map_or(&[], |(_, c)| *c)
    };
    candidatos
        .iter()
        .chain(&["false"])
        .find(|c| admite(c))
        .map(|c| (*c).to_string())
        .or_else(|| valores.first().cloned())
        .unwrap_or_default()
}

/// Propiedades (nombre → valores en orden de aparición) de un blockstate.
fn valores_blockstate(bs: &Value) -> Vec<(String, Vec<String>)> {
    let mut lista: Vec<(String, Vec<String>)> = Vec::new();
    let mut anotar = |k: &str, v: &str| {
        let i = match lista.iter().position(|(n, _)| n == k) {
            Some(i) => i,
            None => {
                lista.push((k.to_string(), Vec::new()));
                lista.len() - 1
            }
        };
        if !lista[i].1.iter().any(|x| x == v) {
            lista[i].1.push(v.to_string());
        }
    };
    if let Some(variantes) = bs.get("variants").and_then(Value::as_object) {
        for clave in variantes.keys() {
            for par in clave.split(',').filter(|p| !p.is_empty()) {
                if let Some((k, v)) = par.split_once('=') {
                    anotar(k, v);
                }
            }
        }
    }
    if let Some(partes) = bs.get("multipart").and_then(Value::as_array) {
        for p in partes {
            if let Some(w) = p.get("when") {
                condiciones(w, &mut anotar);
            }
        }
    }
    lista
}

/// Recorre una condición `when` (con `OR`/`AND` anidados) anotando cada
/// propiedad y valor.
fn condiciones(when: &Value, anotar: &mut impl FnMut(&str, &str)) {
    let Some(obj) = when.as_object() else {
        return;
    };
    for (k, v) in obj {
        if k == "OR" || k == "AND" {
            for w in v.as_array().into_iter().flatten() {
                condiciones(w, anotar);
            }
            continue;
        }
        let texto = match v {
            Value::String(s) => s.clone(),
            otro => otro.to_string(),
        };
        for alternativa in texto.split('|') {
            anotar(k, alternativa);
        }
    }
}

/// Completa y ordena los valores: `true` sin `false` → añade `false`;
/// `low`/`tall` sin `none` → añade `none` delante; todos numéricos → de
/// menor a mayor.
fn completar(valores: &mut Vec<String>) {
    let tiene = |v: &[String], x: &str| v.iter().any(|y| y == x);
    if tiene(valores, "true") && !tiene(valores, "false") {
        valores.push("false".into());
    }
    if (tiene(valores, "low") || tiene(valores, "tall")) && !tiene(valores, "none") {
        valores.insert(0, "none".into());
    }
    if valores.iter().all(|v| v.parse::<i64>().is_ok()) {
        valores.sort_by_key(|v| v.parse::<i64>().unwrap_or(0));
    }
}

/// Propiedades de un blockstate del jar (sin la plantilla de la forma).
pub fn desde_blockstate(bs: &Value, forma: FormaBloque) -> Vec<Propiedad> {
    valores_blockstate(bs)
        .into_iter()
        .map(|(nombre, mut valores)| {
            completar(&mut valores);
            let defecto = defecto(&nombre, &valores, forma);
            Propiedad {
                nombre,
                valores,
                defecto,
            }
        })
        .collect()
}

const FACING: &[&str] = &["north", "south", "west", "east"];
const BOOL: &[&str] = &["false", "true"];
const LADOS: [&str; 4] = ["north", "east", "south", "west"];

/// Propiedades que el juego da a cada forma, las cambie o no el modelo.
/// `id` solo importa en las placas de presión con peso (`power` 0-15 en vez
/// de `powered`). `Entero`, `Alfombra` y `Otro` no tienen plantilla.
pub fn plantilla(forma: FormaBloque, id: &str) -> Vec<Propiedad> {
    let mut v: Vec<(&str, Vec<&str>)> = Vec::new();
    let lados = |v: &mut Vec<(&str, Vec<&str>)>, valores: &[&'static str]| {
        for l in LADOS {
            v.push((l, valores.to_vec()));
        }
    };
    match forma {
        FormaBloque::Losa => {
            v.push(("type", vec!["bottom", "top", "double"]));
            v.push(("waterlogged", BOOL.to_vec()));
        }
        FormaBloque::Escalera => {
            v.push(("facing", FACING.to_vec()));
            v.push(("half", vec!["bottom", "top"]));
            v.push((
                "shape",
                vec![
                    "straight",
                    "inner_left",
                    "inner_right",
                    "outer_left",
                    "outer_right",
                ],
            ));
            v.push(("waterlogged", BOOL.to_vec()));
        }
        FormaBloque::Muro => {
            v.push(("up", vec!["true", "false"]));
            lados(&mut v, &["none", "low", "tall"]);
            v.push(("waterlogged", BOOL.to_vec()));
        }
        FormaBloque::Valla | FormaBloque::Panel => {
            lados(&mut v, BOOL);
            v.push(("waterlogged", BOOL.to_vec()));
        }
        FormaBloque::PuertaValla => {
            v.push(("facing", FACING.to_vec()));
            v.push(("in_wall", BOOL.to_vec()));
            v.push(("open", BOOL.to_vec()));
            v.push(("powered", BOOL.to_vec()));
        }
        FormaBloque::Puerta => {
            v.push(("facing", FACING.to_vec()));
            v.push(("half", vec!["lower", "upper"]));
            v.push(("hinge", vec!["left", "right"]));
            v.push(("open", BOOL.to_vec()));
            v.push(("powered", BOOL.to_vec()));
        }
        FormaBloque::Trampilla => {
            v.push(("facing", FACING.to_vec()));
            v.push(("half", vec!["bottom", "top"]));
            v.push(("open", BOOL.to_vec()));
            v.push(("powered", BOOL.to_vec()));
            v.push(("waterlogged", BOOL.to_vec()));
        }
        FormaBloque::Boton => {
            v.push(("face", vec!["floor", "wall", "ceiling"]));
            v.push(("facing", FACING.to_vec()));
            v.push(("powered", BOOL.to_vec()));
        }
        FormaBloque::PlacaPresion => {
            if id.contains("weighted") {
                v.push((
                    "power",
                    vec![
                        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13",
                        "14", "15",
                    ],
                ));
            } else {
                v.push(("powered", BOOL.to_vec()));
            }
        }
        FormaBloque::Entero if tiene_eje(id) => {
            v.push(("axis", vec!["x", "y", "z"]));
        }
        FormaBloque::Entero | FormaBloque::Alfombra | FormaBloque::Otro => {}
    }
    v.into_iter()
        .map(|(nombre, valores)| {
            let valores: Vec<String> = valores.into_iter().map(String::from).collect();
            Propiedad {
                nombre: nombre.to_string(),
                defecto: defecto(nombre, &valores, forma),
                valores,
            }
        })
        .collect()
}

/// Bloques enteros con `axis` (troncos, madera, tallos, pilares, basalto,
/// balas de heno…). Solo hace falta sin jar: con jar sale del blockstate.
/// Así un cambio de familia sin jar no gira los troncos.
pub fn tiene_eje(id: &str) -> bool {
    let n = id.strip_prefix("minecraft:").unwrap_or(id);
    ["_log", "_wood", "_stem", "_hyphae", "_pillar", "_froglight"]
        .iter()
        .any(|s| n.ends_with(s))
        || matches!(
            n,
            "basalt"
                | "polished_basalt"
                | "bamboo_block"
                | "stripped_bamboo_block"
                | "hay_block"
                | "bone_block"
                | "muddy_mangrove_roots"
                | "purpur_pillar"
                | "quartz_pillar"
                | "deepslate"
                | "infested_deepslate"
                | "creaking_heart"
        )
}

/// Une las propiedades del blockstate con las de la plantilla: las del
/// blockstate mandan; de la plantilla se añaden las que faltan y, en las que
/// coinciden, los valores que falten (p. ej. `false` en un booleano que el
/// blockstate solo nombra con `true`).
pub fn combinar(mut derivadas: Vec<Propiedad>, plantilla: Vec<Propiedad>) -> Vec<Propiedad> {
    for p in plantilla {
        match derivadas.iter_mut().find(|d| d.nombre == p.nombre) {
            Some(d) => {
                for v in p.valores {
                    if !d.admite(&v) {
                        d.valores.push(v);
                    }
                }
            }
            None => derivadas.push(p),
        }
    }
    derivadas
}
