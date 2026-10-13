// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Blockstates y modelos de bloque de Minecraft: de un block state y una cara
//! del mundo a la textura que se ve en ella.
//!
//! Aproximación (suficiente para el visor):
//! - Blockstate: la variante cuyas propiedades encajan (si es una lista con
//!   pesos, la primera); en `multipart`, todas las partes cuyo `when` se
//!   cumple, en orden.
//! - Modelo: herencia por `parent` (las texturas del hijo mandan; los
//!   `elements` son los del primero de la cadena que los tenga) y variables
//!   `#nombre` resueltas.
//! - Cara: la del primer elemento que tenga esa cara, deshaciendo antes los
//!   giros `x`/`y` de la variante. Si ningún elemento la tiene (agua, lava,
//!   cofres, carteles…: bloques que el juego no dibuja con elementos), la
//!   textura `particle`.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;

use crate::malla::Cara;

/// Prefijo de los recursos vanilla.
const NS: &str = "minecraft:";

/// Profundidad máxima de herencia y de variables (evita ciclos).
const MAX_SALTOS: usize = 32;

/// Direcciones de las caras de un modelo, en el orden de [`Cara::CUBO`].
const NOMBRES_CARA: [&str; 6] = ["up", "down", "north", "south", "west", "east"];

/// Vector de cada dirección (índice como en [`NOMBRES_CARA`]).
const VECTORES: [[i32; 3]; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];

/// Una cara de un elemento de modelo.
#[derive(Debug, Clone)]
struct CaraModelo {
    /// Textura tal cual en el JSON (`#side` o `block/stone`).
    textura: String,
    /// Giro de la textura en grados (campo `rotation` de la cara).
    rotacion: i32,
    /// Lleva `tintindex` (color de bioma).
    tintada: bool,
}

/// Elemento (caja) de un modelo: sus límites y sus caras, por dirección.
#[derive(Debug, Clone, Default)]
struct Elemento {
    /// Esquina mínima (`from`), en 1/16 de bloque.
    desde: [f64; 3],
    /// Esquina máxima (`to`), en 1/16 de bloque.
    hasta: [f64; 3],
    caras: [Option<CaraModelo>; 6],
}

impl Elemento {
    /// Lados del bloque (índices como en [`NOMBRES_CARA`], en el espacio del
    /// modelo) que este elemento tapa por completo: tiene la cara de ese lado,
    /// llega hasta el borde del bloque y ocupa de 0 a 16 en los otros dos
    /// ejes. Sirve igual un cubo entero que una lámina sin grosor (los
    /// bloques de champiñón y la nieve en polvo se montan así). `None` si el
    /// elemento se sale del bloque. Tolerancia de 0,01 (hay modelos con
    /// 0,002 para evitar parpadeos).
    fn lados_tapados(&self) -> Option<Vec<usize>> {
        const TOL: f64 = 0.01;
        let min: [f64; 3] = std::array::from_fn(|e| self.desde[e].min(self.hasta[e]));
        let max: [f64; 3] = std::array::from_fn(|e| self.desde[e].max(self.hasta[e]));
        if (0..3).any(|e| min[e] < -TOL || max[e] > 16.0 + TOL) {
            return None;
        }
        let entero = |e: usize| min[e] <= TOL && max[e] >= 16.0 - TOL;
        let mut lados = Vec::new();
        for (i, cara) in self.caras.iter().enumerate() {
            if cara.is_none() {
                continue;
            }
            let eje = VECTORES[i].iter().position(|&c| c != 0).unwrap_or(0);
            let positiva = VECTORES[i][eje] > 0;
            let en_borde = if positiva {
                max[eje] >= 16.0 - TOL
            } else {
                min[eje] <= TOL
            };
            if en_borde && (0..3).filter(|&e| e != eje).all(entero) {
                lados.push(i);
            }
        }
        Some(lados)
    }
}

/// Modelo con la herencia ya aplicada.
#[derive(Debug, Clone, Default)]
pub struct ModeloPlano {
    texturas: HashMap<String, String>,
    elementos: Vec<Elemento>,
}

impl ModeloPlano {
    /// Resuelve `#variable` siguiendo la cadena. Devuelve la ruta sin
    /// espacio de nombres (`block/stone`) o `None` si no termina en una ruta.
    fn textura(&self, referencia: &str) -> Option<String> {
        let mut r = referencia;
        for _ in 0..MAX_SALTOS {
            match r.strip_prefix('#') {
                Some(var) => r = self.texturas.get(var)?,
                None => return Some(sin_ns(r).to_string()),
            }
        }
        None
    }
}

/// Modelo aplicado por una variante o parte, con sus giros.
#[derive(Debug, Clone, PartialEq)]
pub struct Aplicado {
    pub modelo: String,
    pub x: i32,
    pub y: i32,
    pub uvlock: bool,
}

/// Textura de una cara ya resuelta (antes de buscarla en el atlas).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaraResuelta {
    /// Ruta de la textura sin espacio de nombres: `block/oak_log`.
    pub textura: String,
    /// Giro en grados, horario mirando la cara desde fuera (ver
    /// `docs/visor.md`).
    pub rotacion: u16,
}

/// Blockstates y modelos de bloque de una versión (JSON ya parseado).
#[derive(Debug, Default)]
pub struct Recursos {
    /// `oak_log` → contenido de `blockstates/oak_log.json`.
    blockstates: HashMap<String, Value>,
    /// `block/oak_log` → contenido de `models/block/oak_log.json`.
    modelos: HashMap<String, Value>,
    /// Modelos ya aplanados.
    planos: Mutex<HashMap<String, Option<std::sync::Arc<ModeloPlano>>>>,
}

fn sin_ns(s: &str) -> &str {
    s.strip_prefix(NS).unwrap_or(s)
}

impl Recursos {
    /// Recursos a partir de los textos JSON. `blockstates`: (nombre del
    /// bloque, JSON); `modelos`: (ruta `block/x`, JSON). Los JSON que no se
    /// pueden leer se ignoran.
    pub fn desde_json<'a>(
        blockstates: impl IntoIterator<Item = (String, &'a [u8])>,
        modelos: impl IntoIterator<Item = (String, &'a [u8])>,
    ) -> Self {
        let parsear = |(k, v): (String, &[u8])| Some((k, serde_json::from_slice(v).ok()?));
        Recursos {
            blockstates: blockstates.into_iter().filter_map(parsear).collect(),
            modelos: modelos.into_iter().filter_map(parsear).collect(),
            planos: Mutex::new(HashMap::new()),
        }
    }

    /// Nombres de los bloques con blockstate (sin espacio de nombres).
    pub fn bloques(&self) -> impl Iterator<Item = &str> {
        self.blockstates.keys().map(String::as_str)
    }

    /// JSON del blockstate de `nombre` (sin espacio de nombres), tal cual.
    pub fn blockstate(&self, nombre: &str) -> Option<&Value> {
        self.blockstates.get(nombre)
    }

    /// ¿El bloque `nombre` con estas propiedades es un cubo completo? Sí
    /// cuando ningún elemento de los modelos que aplica se sale del bloque y,
    /// entre todos y ya girados por la variante, tapan los seis lados (ver
    /// `Elemento::lados_tapados`). La hierba, los bloques de champiñón o la
    /// estantería cincelada cuentan; un farol, un cactus, un caldero, un
    /// cofre o el agua, no.
    pub fn es_cubo_completo(&self, nombre: &str, props: &[(&str, &str)]) -> bool {
        let mut tapados = [false; 6];
        for a in self.aplicados(nombre, props) {
            let Some(p) = self.plano(&a.modelo) else {
                return false;
            };
            for e in &p.elementos {
                let Some(lados) = e.lados_tapados() else {
                    return false;
                };
                for l in lados {
                    tapados[direccion(girar(VECTORES[l], a.x, a.y))] = true;
                }
            }
        }
        tapados.iter().all(|&t| t)
    }

    pub fn n_blockstates(&self) -> usize {
        self.blockstates.len()
    }

    pub fn n_modelos(&self) -> usize {
        self.modelos.len()
    }

    /// Modelo con la herencia aplicada (memorizado).
    pub fn plano(&self, ruta: &str) -> Option<std::sync::Arc<ModeloPlano>> {
        let ruta = sin_ns(ruta);
        if let Some(m) = self
            .planos
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(ruta)
        {
            return m.clone();
        }
        let m = self.aplanar(ruta).map(std::sync::Arc::new);
        self.planos
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(ruta.to_string(), m.clone());
        m
    }

    fn aplanar(&self, ruta: &str) -> Option<ModeloPlano> {
        let mut plano = ModeloPlano::default();
        let mut con_elementos = false;
        let mut actual = self.modelos.get(ruta)?;
        for _ in 0..MAX_SALTOS {
            if let Some(t) = actual.get("textures").and_then(Value::as_object) {
                for (k, v) in t {
                    if let Some(v) = v.as_str() {
                        plano
                            .texturas
                            .entry(k.clone())
                            .or_insert_with(|| v.to_string());
                    }
                }
            }
            if !con_elementos && let Some(els) = actual.get("elements").and_then(Value::as_array) {
                con_elementos = true;
                plano.elementos = els.iter().map(elemento).collect();
            }
            let Some(padre) = actual.get("parent").and_then(Value::as_str) else {
                break;
            };
            match self.modelos.get(sin_ns(padre)) {
                Some(p) => actual = p,
                // `builtin/generated`, `builtin/entity` o un padre que falta.
                None => break,
            }
        }
        Some(plano)
    }

    /// Modelos que aplica el blockstate de `nombre` (sin espacio de nombres)
    /// con estas propiedades.
    pub fn aplicados(&self, nombre: &str, props: &[(&str, &str)]) -> Vec<Aplicado> {
        let Some(bs) = self.blockstates.get(nombre) else {
            return Vec::new();
        };
        if let Some(variantes) = bs.get("variants").and_then(Value::as_object) {
            // La que encaja con más propiedades sin contradecir ninguna; si
            // ninguna encaja, la primera.
            let mut mejor: Option<(usize, &Value)> = None;
            for (clave, v) in variantes {
                if let Some(n) = encaje_variante(clave, props)
                    && mejor.is_none_or(|(m, _)| n > m)
                {
                    mejor = Some((n, v));
                }
            }
            let elegida = mejor.map(|(_, v)| v).or_else(|| variantes.values().next());
            return elegida.and_then(aplicado).into_iter().collect();
        }
        if let Some(partes) = bs.get("multipart").and_then(Value::as_array) {
            return partes
                .iter()
                .filter(|p| p.get("when").is_none_or(|w| cumple(w, props)))
                .filter_map(|p| p.get("apply").and_then(aplicado))
                .collect();
        }
        Vec::new()
    }

    /// Todos los modelos que puede usar el blockstate de `nombre` (cualquier
    /// variante, parte o entrada con peso). Para buscar texturas tintadas.
    pub fn todos_los_modelos(&self, nombre: &str) -> Vec<String> {
        let Some(bs) = self.blockstates.get(nombre) else {
            return Vec::new();
        };
        let mut aplicaciones: Vec<&Value> = Vec::new();
        if let Some(v) = bs.get("variants").and_then(Value::as_object) {
            aplicaciones.extend(v.values());
        }
        if let Some(p) = bs.get("multipart").and_then(Value::as_array) {
            aplicaciones.extend(p.iter().filter_map(|p| p.get("apply")));
        }
        let mut modelos = Vec::new();
        for a in aplicaciones {
            let lista = a.as_array().map_or_else(|| vec![a], |l| l.iter().collect());
            for e in lista {
                if let Some(m) = e.get("model").and_then(Value::as_str) {
                    modelos.push(sin_ns(m).to_string());
                }
            }
        }
        modelos
    }

    /// Texturas que algún elemento de este modelo pinta con `tintindex`.
    pub fn texturas_tintadas(&self, modelo: &str) -> Vec<String> {
        let Some(p) = self.plano(modelo) else {
            return Vec::new();
        };
        p.elementos
            .iter()
            .flat_map(|e| e.caras.iter().flatten())
            .filter(|c| c.tintada)
            .filter_map(|c| p.textura(&c.textura))
            .collect()
    }

    /// Textura que se ve en la cara `cara` (del mundo) del block state
    /// `estado` (`minecraft:oak_log[axis=x]`). `None` si el bloque no es
    /// vanilla, no tiene blockstate o no tiene textura en esa cara.
    pub fn resolver(&self, estado: &str, cara: Cara) -> Option<CaraResuelta> {
        let (nombre, resto) = match estado.split_once('[') {
            Some((n, r)) => (n, r.strip_suffix(']').unwrap_or(r)),
            None => (estado, ""),
        };
        let nombre = nombre.strip_prefix(NS)?;
        let props: Vec<(&str, &str)> = resto
            .split(',')
            .filter_map(|par| par.split_once('='))
            .collect();
        let aplicados = self.aplicados(nombre, &props);

        for a in &aplicados {
            let Some(plano) = self.plano(&a.modelo) else {
                continue;
            };
            if cara == Cara::Cruz {
                // Planos cruzados: la primera cara del primer elemento.
                let c = plano
                    .elementos
                    .iter()
                    .find_map(|e| [2, 3, 5, 4, 0, 1].iter().find_map(|&i| e.caras[i].as_ref()));
                if let Some(c) = c.and_then(|c| plano.textura(&c.textura)) {
                    return Some(CaraResuelta {
                        textura: c,
                        rotacion: 0,
                    });
                }
                continue;
            }
            let w = cara as usize;
            let m = direccion(deshacer_giro(VECTORES[w], a.x, a.y));
            let Some(c) = plano.elementos.iter().find_map(|e| e.caras[m].as_ref()) else {
                continue;
            };
            let Some(textura) = plano.textura(&c.textura) else {
                continue;
            };
            let rotacion = if a.uvlock {
                0
            } else {
                giro_en_mundo(m, c.rotacion, a.x, a.y, w)
            };
            return Some(CaraResuelta { textura, rotacion });
        }
        // Sin elementos: la textura de partícula del primer modelo.
        let plano = self.plano(&aplicados.first()?.modelo)?;
        let textura = plano.textura("#particle")?;
        Some(CaraResuelta {
            textura,
            rotacion: 0,
        })
    }
}

/// Cuántas propiedades de la clave de variante (`a=b,c=d`) encajan, o `None`
/// si alguna contradice a las del estado. Las que el estado no trae no
/// cuentan ni descartan.
fn encaje_variante(clave: &str, props: &[(&str, &str)]) -> Option<usize> {
    let mut n = 0;
    for par in clave.split(',').filter(|p| !p.is_empty()) {
        let (k, v) = par.split_once('=')?;
        match props.iter().find(|(pk, _)| *pk == k) {
            Some((_, pv)) if *pv == v => n += 1,
            Some(_) => return None,
            None => {}
        }
    }
    Some(n)
}

/// ¿Se cumple la condición `when` de una parte de multipart?
fn cumple(when: &Value, props: &[(&str, &str)]) -> bool {
    let Some(obj) = when.as_object() else {
        return false;
    };
    if let Some(o) = obj.get("OR").and_then(Value::as_array) {
        return o.iter().any(|w| cumple(w, props));
    }
    if let Some(y) = obj.get("AND").and_then(Value::as_array) {
        return y.iter().all(|w| cumple(w, props));
    }
    obj.iter().all(|(k, v)| {
        let esperado = match v {
            Value::String(s) => s.clone(),
            otro => otro.to_string(),
        };
        props
            .iter()
            .find(|(pk, _)| pk == k)
            .is_some_and(|(_, pv)| esperado.split('|').any(|e| e == *pv))
    })
}

/// Variante o parte (`{"model": ..., "x": 90}` o lista con pesos → la
/// primera) a [`Aplicado`].
fn aplicado(v: &Value) -> Option<Aplicado> {
    let v = match v.as_array() {
        Some(lista) => lista.first()?,
        None => v,
    };
    let giro = |k: &str| v.get(k).and_then(Value::as_i64).unwrap_or(0) as i32;
    Some(Aplicado {
        modelo: sin_ns(v.get("model")?.as_str()?).to_string(),
        x: giro("x"),
        y: giro("y"),
        uvlock: v.get("uvlock").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// Lee un elemento de modelo (límites y caras).
fn elemento(v: &Value) -> Elemento {
    let esquina = |k: &str| -> [f64; 3] {
        let mut r = [0.0; 3];
        if let Some(l) = v.get(k).and_then(Value::as_array) {
            for (i, x) in l.iter().take(3).enumerate() {
                r[i] = x.as_f64().unwrap_or(0.0);
            }
        }
        r
    };
    let mut e = Elemento {
        desde: esquina("from"),
        hasta: esquina("to"),
        ..Elemento::default()
    };
    let Some(caras) = v.get("faces").and_then(Value::as_object) else {
        return e;
    };
    for (i, nombre) in NOMBRES_CARA.iter().enumerate() {
        let Some(c) = caras.get(*nombre) else {
            continue;
        };
        let Some(textura) = c.get("texture").and_then(Value::as_str) else {
            continue;
        };
        e.caras[i] = Some(CaraModelo {
            textura: textura.to_string(),
            rotacion: c.get("rotation").and_then(Value::as_i64).unwrap_or(0) as i32,
            tintada: c
                .get("tintindex")
                .and_then(Value::as_i64)
                .is_some_and(|t| t >= 0),
        });
    }
    e
}

/// Giro de 90° alrededor de X como el `x: 90` de Minecraft
/// (arriba → norte → abajo → sur → arriba).
fn giro_x([x, y, z]: [i32; 3]) -> [i32; 3] {
    [x, z, -y]
}

/// Giro de 90° alrededor de Y como el `y: 90` de Minecraft
/// (norte → este → sur → oeste → norte).
fn giro_y([x, y, z]: [i32; 3]) -> [i32; 3] {
    [-z, y, x]
}

/// Cuartos de vuelta (0..4) de un ángulo en grados.
fn cuartos(grados: i32) -> usize {
    (grados.rem_euclid(360) / 90) as usize
}

/// Aplica el giro de una variante: primero `x`, después `y`.
fn girar(mut v: [i32; 3], x: i32, y: i32) -> [i32; 3] {
    for _ in 0..cuartos(x) {
        v = giro_x(v);
    }
    for _ in 0..cuartos(y) {
        v = giro_y(v);
    }
    v
}

/// Deshace el giro de una variante: `y` hacia atrás y después `x`.
fn deshacer_giro(mut v: [i32; 3], x: i32, y: i32) -> [i32; 3] {
    for _ in 0..(4 - cuartos(y)) % 4 {
        v = giro_y(v);
    }
    for _ in 0..(4 - cuartos(x)) % 4 {
        v = giro_x(v);
    }
    v
}

/// Índice de dirección de un vector unitario de eje.
fn direccion(v: [i32; 3]) -> usize {
    VECTORES.iter().position(|d| *d == v).unwrap_or(0)
}

fn cruz([a, b, c]: [i32; 3], [d, e, f]: [i32; 3]) -> [i32; 3] {
    [b * f - c * e, c * d - a * f, a * e - b * d]
}

fn neg([a, b, c]: [i32; 3]) -> [i32; 3] {
    [-a, -b, -c]
}

/// Hacia dónde apunta el borde superior de la imagen en la cara `i` sin
/// giros (convenio de `docs/visor.md`): en los lados, arriba; en la cara de
/// arriba, al norte; en la de abajo, al sur.
fn arriba_imagen(i: usize) -> [i32; 3] {
    match i {
        0 => [0, 0, -1],
        1 => [0, 0, 1],
        _ => [0, 1, 0],
    }
}

/// Derecha de la imagen mirando la cara `i` desde fuera.
fn derecha_imagen(i: usize) -> [i32; 3] {
    cruz(arriba_imagen(i), VECTORES[i])
}

/// Giro (grados, horario mirando desde fuera) con que se ve en la cara del
/// mundo `w` la textura de la cara `m` del modelo, que ya lleva un giro
/// propio `rot_cara`, tras girar el modelo `x`, `y`.
fn giro_en_mundo(m: usize, rot_cara: i32, x: i32, y: i32, w: usize) -> u16 {
    // Borde superior de la imagen en el modelo, con el giro propio de la cara
    // (girar en sentido horario un cuarto lleva «arriba» a «derecha»).
    let mut arriba = arriba_imagen(m);
    for _ in 0..cuartos(rot_cara) {
        arriba = cruz(arriba, VECTORES[m]);
    }
    let arriba = girar(arriba, x, y);
    let (a, d) = (arriba_imagen(w), derecha_imagen(w));
    if arriba == a {
        0
    } else if arriba == d {
        90
    } else if arriba == neg(a) {
        180
    } else {
        270
    }
}

#[cfg(test)]
mod giros {
    use super::*;

    #[test]
    fn giros_como_minecraft() {
        // Pistón: la cabeza del modelo está al norte; `x: 270` la pone arriba.
        assert_eq!(girar([0, 0, -1], 270, 0), [0, 1, 0]);
        // Horno: frente al norte; `y: 90` lo lleva al este.
        assert_eq!(girar([0, 0, -1], 0, 90), [1, 0, 0]);
        for v in VECTORES {
            for (x, y) in [(90, 0), (0, 270), (90, 90), (180, 270)] {
                assert_eq!(deshacer_giro(girar(v, x, y), x, y), v);
            }
        }
    }

    #[test]
    fn derecha_de_la_cara_norte_es_el_oeste() {
        assert_eq!(derecha_imagen(2), [-1, 0, 0]);
        assert_eq!(derecha_imagen(0), [1, 0, 0]);
    }
}
