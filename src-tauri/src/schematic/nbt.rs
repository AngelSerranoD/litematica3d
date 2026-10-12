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
