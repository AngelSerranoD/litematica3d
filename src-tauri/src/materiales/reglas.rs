// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Reglas que traducen un block state a los ítems necesarios para colocarlo.
//! Salen de `datos/bloque_a_item.json`, incluido en el binario y leído una sola
//! vez. El esquema está descrito en el propio JSON (`_descripcion`).

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::schematic::es_aire;

/// Texto del JSON de reglas, incrustado al compilar.
pub const JSON_REGLAS: &str = include_str!("../../datos/bloque_a_item.json");

/// Prefijo de los IDs antiguos (MCEdit) que no tienen traducción.
pub const PREFIJO_DESCONOCIDO: &str = "litematica3d:desconocido_";

/// Versión del esquema del JSON que entiende este código.
const VERSION_ESQUEMA: u32 = 1;

/// Tamaño de stack de lo que no aparece en `stack`.
const STACK_POR_DEFECTO: u32 = 64;

static REGLAS: LazyLock<Result<Reglas, String>> = LazyLock::new(|| Reglas::desde_json(JSON_REGLAS));

/// Reglas del JSON incrustado. Si no se pudieran leer (no debería pasar: lo
/// comprueba un test), devuelve el motivo en vez de abortar la aplicación.
pub fn globales() -> Result<&'static Reglas, &'static str> {
    REGLAS.as_ref().map_err(String::as_str)
}

/// El JSON tal cual viene, antes de compilar los comodines.
#[derive(Deserialize)]
struct Crudo {
    version: u32,
    ignorar: Vec<String>,
    item: HashMap<String, String>,
    varios: HashMap<String, Vec<String>>,
    solo_si: HashMap<String, HashMap<String, String>>,
    cantidad_por_propiedad: HashMap<String, String>,
    multiplicador: HashMap<String, HashMap<String, HashMap<String, u32>>>,
    fluidos: HashMap<String, String>,
    stack: HashMap<String, Vec<String>>,
}

/// Clave con un comodín: `prefijo*sufijo`.
#[derive(Debug)]
struct Comodin<T> {
    prefijo: String,
    sufijo: String,
    valor: T,
}

impl<T> Comodin<T> {
    fn encaja(&self, id: &str) -> bool {
        id.len() >= self.prefijo.len() + self.sufijo.len()
            && id.starts_with(&self.prefijo)
            && id.ends_with(&self.sufijo)
    }
}

/// Tabla de bloque -> valor con claves exactas y claves con un `*`.
/// La exacta gana; entre comodines gana el más específico (más texto fijo).
#[derive(Debug)]
pub struct Tabla<T> {
    exactas: HashMap<String, T>,
    comodines: Vec<Comodin<T>>,
}

impl<T> Tabla<T> {
    fn nueva(entradas: impl IntoIterator<Item = (String, T)>) -> Result<Self, String> {
        let mut exactas = HashMap::new();
        let mut comodines = Vec::new();
        for (clave, valor) in entradas {
            if clave.contains('[') {
                return Err(format!("la clave «{clave}» no debe llevar propiedades"));
            }
            match clave.matches('*').count() {
                0 => {
                    exactas.insert(clave, valor);
                }
                1 => {
                    let (prefijo, sufijo) = clave.split_once('*').unwrap_or_default();
                    comodines.push(Comodin {
                        prefijo: prefijo.to_string(),
                        sufijo: sufijo.to_string(),
                        valor,
                    });
                }
                _ => return Err(format!("la clave «{clave}» tiene más de un comodín")),
            }
        }
        // Más texto fijo primero; a igualdad, orden alfabético para que sea estable.
        comodines.sort_by(|a, b| {
            let largo = |c: &Comodin<T>| c.prefijo.len() + c.sufijo.len();
            largo(b)
                .cmp(&largo(a))
                .then_with(|| (&a.prefijo, &a.sufijo).cmp(&(&b.prefijo, &b.sufijo)))
        });
        Ok(Tabla { exactas, comodines })
    }

    /// Valor para un ID de bloque (sin propiedades).
    pub fn buscar(&self, id: &str) -> Option<&T> {
        self.exactas.get(id).or_else(|| {
            self.comodines
                .iter()
                .find(|c| c.encaja(id))
                .map(|c| &c.valor)
        })
    }
}

/// Qué aporta un block state a la lista de materiales.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolucion {
    /// No cuenta (aire, técnico, mitad superior de una puerta, fluido excluido…).
    Nada,
    /// ID antiguo sin traducción.
    Desconocido,
    /// Ítems y cuántos de cada uno por bloque.
    Items(Vec<(String, u64)>),
}

/// Reglas ya compiladas, listas para consultar.
#[derive(Debug)]
pub struct Reglas {
    ignorar: Tabla<()>,
    item: Tabla<String>,
    varios: Tabla<Vec<String>>,
    solo_si: Tabla<Vec<(String, String)>>,
    cantidad: Tabla<String>,
    multiplicador: Tabla<Vec<(String, HashMap<String, u32>)>>,
    fluidos: Tabla<String>,
    stack: HashMap<String, u32>,
}

impl Reglas {
    /// Lee y valida un JSON con el esquema de `bloque_a_item.json`.
    pub fn desde_json(texto: &str) -> Result<Self, String> {
        let c: Crudo = serde_json::from_str(texto)
            .map_err(|e| format!("bloque_a_item.json no es válido: {e}"))?;
        if c.version != VERSION_ESQUEMA {
            return Err(format!(
                "bloque_a_item.json tiene la versión {} y se esperaba la {VERSION_ESQUEMA}",
                c.version
            ));
        }
        let mut stack = HashMap::new();
        for (tamano, items) in c.stack {
            let n: u32 = tamano
                .parse()
                .ok()
                .filter(|&n| n > 0)
                .ok_or_else(|| format!("tamaño de stack no válido: «{tamano}»"))?;
            for item in items {
                stack.insert(item, n);
            }
        }
        Ok(Reglas {
            ignorar: Tabla::nueva(c.ignorar.into_iter().map(|b| (b, ())))?,
            item: Tabla::nueva(c.item)?,
            varios: Tabla::nueva(c.varios)?,
            solo_si: Tabla::nueva(
                c.solo_si
                    .into_iter()
                    .map(|(b, props)| (b, props.into_iter().collect())),
            )?,
            cantidad: Tabla::nueva(c.cantidad_por_propiedad)?,
            multiplicador: Tabla::nueva(
                c.multiplicador
                    .into_iter()
                    .map(|(b, props)| (b, props.into_iter().collect())),
            )?,
            fluidos: Tabla::nueva(c.fluidos)?,
            stack,
        })
    }

    /// Ítems por stack (64 si no está en la tabla).
    pub fn tamano_stack(&self, item: &str) -> u32 {
        self.stack.get(item).copied().unwrap_or(STACK_POR_DEFECTO)
    }

    /// Traduce un block state normalizado (`minecraft:x[a=1,b=2]`).
    pub fn resolver(&self, estado: &str, incluir_fluidos: bool) -> Resolucion {
        let (id, props) = separar(estado);
        let prop = |nombre: &str| props.iter().find(|(k, _)| *k == nombre).map(|&(_, v)| v);

        if id.starts_with(PREFIJO_DESCONOCIDO) {
            return Resolucion::Desconocido;
        }
        if es_aire(id) || self.ignorar.buscar(id).is_some() {
            return Resolucion::Nada;
        }
        if let Some(cubo) = self.fluidos.buscar(id) {
            // Sin `level` se toma como fuente: es lo que pone el juego por defecto.
            let fuente = prop("level").is_none_or(|v| v == "0");
            return if incluir_fluidos && fuente {
                Resolucion::Items(vec![(cubo.clone(), 1)])
            } else {
                Resolucion::Nada
            };
        }
        // solo_si, cantidad y multiplicador miran el ID original, no el ítem.
        if let Some(condiciones) = self.solo_si.buscar(id)
            && !condiciones.iter().all(|(k, v)| prop(k) == Some(v.as_str()))
        {
            return Resolucion::Nada;
        }
        let mut n: u64 = 1;
        if let Some(nombre) = self.cantidad.buscar(id) {
            n = prop(nombre).and_then(|v| v.parse().ok()).unwrap_or(1);
        }
        if let Some(multiplicadores) = self.multiplicador.buscar(id) {
            for (k, tabla) in multiplicadores {
                if let Some(&m) = prop(k).and_then(|v| tabla.get(v)) {
                    n = n.saturating_mul(u64::from(m));
                }
            }
        }
        if n == 0 {
            return Resolucion::Nada;
        }
        let items = if let Some(lista) = self.varios.buscar(id) {
            lista.iter().map(|i| (i.clone(), n)).collect()
        } else if let Some(i) = self.item.buscar(id) {
            vec![(i.clone(), n)]
        } else {
            vec![(id.to_string(), n)]
        };
        Resolucion::Items(items)
    }

    /// Todos los ítems que pueden salir de la tabla (para validarla).
    #[cfg(test)]
    pub(crate) fn items_declarados(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.stack.keys().map(String::as_str).collect();
        let tabla_item = self
            .item
            .exactas
            .values()
            .chain(self.item.comodines.iter().map(|c| &c.valor));
        v.extend(tabla_item.map(String::as_str));
        let tabla_varios = self
            .varios
            .exactas
            .values()
            .chain(self.varios.comodines.iter().map(|c| &c.valor));
        v.extend(tabla_varios.flatten().map(String::as_str));
        let tabla_fluidos = self
            .fluidos
            .exactas
            .values()
            .chain(self.fluidos.comodines.iter().map(|c| &c.valor));
        v.extend(tabla_fluidos.map(String::as_str));
        v
    }
}

/// Separa `ns:id[a=1,b=2]` en el ID y sus propiedades.
pub(crate) fn separar(estado: &str) -> (&str, Vec<(&str, &str)>) {
    match estado.split_once('[') {
        None => (estado, Vec::new()),
        Some((id, resto)) => {
            let resto = resto.strip_suffix(']').unwrap_or(resto);
            let props = resto
                .split(',')
                .filter(|p| !p.is_empty())
                .map(|p| p.split_once('=').unwrap_or((p, "")))
                .collect();
            (id, props)
        }
    }
}
