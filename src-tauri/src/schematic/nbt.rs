// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Utilidades NBT comunes a los lectores de formato.

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

use serde::Deserialize;
use serde::de::{DeserializeOwned, MapAccess, Visitor};

use super::{ErrorSchematic, construir_estado};

/// Deserializa NBT sin comprimir. Las listas no pueden declarar más elementos
/// que bytes tiene la entrada, para no reservar memoria de más con datos malos.
pub(crate) fn deserializar<T: DeserializeOwned>(datos: &[u8]) -> Result<T, ErrorSchematic> {
    let opciones = fastnbt::DeOpts::new().max_seq_len(datos.len().max(16));
    fastnbt::from_bytes_with_opts(datos, opciones).map_err(ErrorSchematic::corrupto)
}

/// Anidamiento máximo de listas y compounds. Los schematics reales no pasan
/// de unos 20 niveles; el límite evita que un archivo de pocos KB con miles
/// de niveles desborde la pila, porque la deserialización es recursiva.
pub(crate) const MAX_PROFUNDIDAD: usize = 128;

/// Recorre el NBT sin recursión y rechaza el que anida demasiado. Se llama
/// una vez antes de deserializar nada.
pub(crate) fn comprobar_profundidad(datos: &[u8]) -> Result<(), ErrorSchematic> {
    enum Marco {
        Compound,
        Lista { tipo: u8, quedan: usize },
    }

    let truncado = || ErrorSchematic::corrupto("NBT truncado");
    let mut pos = 0usize;
    let mut leer = move |n: usize| -> Result<&[u8], ErrorSchematic> {
        let fin = pos
            .checked_add(n)
            .filter(|&f| f <= datos.len())
            .ok_or_else(truncado)?;
        let trozo = &datos[pos..fin];
        pos = fin;
        Ok(trozo)
    };
    let largo = |b: &[u8]| i32::from_be_bytes([b[0], b[1], b[2], b[3]]).max(0) as usize;

    // Etiqueta raíz con nombre.
    let raiz = leer(1)?[0];
    let n = leer(2)?;
    leer(u16::from_be_bytes([n[0], n[1]]) as usize)?;

    let mut pila: Vec<Marco> = Vec::new();
    let mut pendiente = Some(raiz);
    loop {
        if let Some(tipo) = pendiente.take() {
            // Carga útil de una etiqueta del tipo `tipo`.
            if let Some(b) = ancho_fijo(tipo) {
                leer(b)?;
            } else {
                match tipo {
                    7 => {
                        let l = largo(leer(4)?);
                        leer(l)?;
                    }
                    8 => {
                        let n = leer(2)?;
                        leer(u16::from_be_bytes([n[0], n[1]]) as usize)?;
                    }
                    11 | 12 => {
                        let l = largo(leer(4)?);
                        let ancho = if tipo == 11 { 4 } else { 8 };
                        leer(l.checked_mul(ancho).ok_or_else(truncado)?)?;
                    }
                    9 => {
                        let tipo_elem = leer(1)?[0];
                        let quedan = largo(leer(4)?);
                        // Elementos de ancho fijo: se saltan de una vez. Una lista
                        // de tipo End solo puede estar vacía (fastnbt 2.6 entra
                        // en pánico si no lo está).
                        match (tipo_elem, ancho_fijo(tipo_elem)) {
                            (0, _) if quedan > 0 => {
                                return Err(ErrorSchematic::corrupto(
                                    "lista NBT de tipo End con elementos",
                                ));
                            }
                            (0, _) => {}
                            (_, Some(b)) => {
                                leer(quedan.checked_mul(b).ok_or_else(truncado)?)?;
                            }
                            _ => pila.push(Marco::Lista {
                                tipo: tipo_elem,
                                quedan,
                            }),
                        }
                    }
                    10 => pila.push(Marco::Compound),
                    0 => {}
                    _ => return Err(ErrorSchematic::corrupto("etiqueta NBT desconocida")),
                }
                if pila.len() > MAX_PROFUNDIDAD {
                    return Err(ErrorSchematic::DemasiadoGrande(format!(
                        "anida más de {MAX_PROFUNDIDAD} niveles"
                    )));
                }
            }
        }

        // Siguiente etiqueta según el contenedor abierto.
        match pila.last_mut() {
            None => return Ok(()),
            Some(Marco::Compound) => {
                let tipo = leer(1)?[0];
                if tipo == 0 {
                    pila.pop();
                } else {
                    let n = leer(2)?;
                    leer(u16::from_be_bytes([n[0], n[1]]) as usize)?;
                    pendiente = Some(tipo);
                }
            }
            Some(Marco::Lista { tipo, quedan }) => {
                if *quedan == 0 {
                    pila.pop();
                } else {
                    *quedan -= 1;
                    pendiente = Some(*tipo);
                }
            }
        }
    }
}

/// Bytes de la carga útil de los tipos NBT numéricos.
fn ancho_fijo(tipo: u8) -> Option<usize> {
    match tipo {
        1 => Some(1),
        2 => Some(2),
        3 | 5 => Some(4),
        4 | 6 => Some(8),
        _ => None,
    }
}

/// Compound que conserva el orden en que aparecen las claves en el archivo
/// (`fastnbt::Value::Compound` es un HashMap y lo perdería).
pub(crate) struct MapaOrdenado<T>(pub Vec<(String, T)>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for MapaOrdenado<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitante<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for Visitante<T> {
            type Value = MapaOrdenado<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("un compound NBT")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut mapa: A) -> Result<Self::Value, A::Error> {
                let mut pares = Vec::new();
                while let Some((clave, valor)) = mapa.next_entry::<String, T>()? {
                    pares.push((clave, valor));
                }
                Ok(MapaOrdenado(pares))
            }
        }

        d.deserialize_map(Visitante(PhantomData))
    }
}

/// Entrada de paleta con `Name` y `Properties` (litematic y estructura).
#[derive(Deserialize)]
pub(crate) struct EntradaPaleta {
    #[serde(rename = "Name")]
    pub nombre: String,
    #[serde(rename = "Properties")]
    pub propiedades: Option<BTreeMap<String, String>>,
}

impl EntradaPaleta {
    /// Block state normalizado.
    pub fn estado(&self) -> String {
        let props = self
            .propiedades
            .iter()
            .flatten()
            .map(|(k, v)| (k.as_str(), v.as_str()));
        construir_estado(&self.nombre, props)
    }
}
