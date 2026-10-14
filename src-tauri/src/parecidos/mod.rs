// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Bloques parecidos: cimientos para sugerir cambios de un bloque por otro
//! de color parecido y la misma forma.
//!
//! - [`color`]: color de cada block state (caras de arriba y laterales, sin
//!   píxeles transparentes), OKLab y distancia; caché `colores.json`.
//! - [`forma`]: [`FormaBloque`] de cada bloque (entero, losa, escalera…).
//! - [`propiedades`]: propiedades admitidas y valores por defecto.
//! - [`exclusiones`]: bloques que no se sugieren nunca.
//! - [`familias`]: familia de material y variante de cada bloque.
//! - [`sugerir`]: parecidos, familias parecidas y plan para cambiar una
//!   familia por otra en un schematic.
//! - [`Catalogo`]: todo lo anterior junto para los bloques conocidos de la
//!   versión de texturas activa (los `blockstates/*.json` del jar) o, sin
//!   jar, los de `datos/colores_bloques.json`.
//!
//! Con jar, el catálogo usa el atlas activo de [`crate::texturas`] (que ya
//! lleva tintes) y guarda las medias en
//! `<app_cache_dir>/texturas/<versión>/colores.json`, junto al atlas.

pub mod color;
pub mod exclusiones;
pub mod familias;
pub mod forma;
pub mod propiedades;
pub mod sugerir;

#[cfg(test)]
mod pruebas;
#[cfg(test)]
mod pruebas_sugerir;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;

use crate::malla::apariencia::{colores_integrados, nombre_de};
use crate::materiales::reglas::separar;
use crate::schematic::construir_estado;
use crate::texturas::Activo;

pub use color::{MediaTextura, Oklab, distancia_rgb};
pub use exclusiones::{Exclusion, es_sugerible, motivo_exclusion};
pub use familias::{Miembro, Pertenencia, Variante, familia_de};
pub use forma::FormaBloque;
pub use propiedades::Propiedad;
pub use sugerir::{
    EstadoPresente, FamiliaBloque, FamiliaParecida, Parecido, miembros_en_schematic,
};

use color::{CacheColores, color_de_estado};

/// Bases de los muros vanilla (`<base>_wall`), para el catálogo sin jar.
pub const MUROS_VANILLA: &[&str] = &[
    "cobblestone",
    "mossy_cobblestone",
    "stone_brick",
    "mossy_stone_brick",
    "granite",
    "diorite",
    "andesite",
    "cobbled_deepslate",
    "polished_deepslate",
    "deepslate_brick",
    "deepslate_tile",
    "brick",
    "mud_brick",
    "nether_brick",
    "red_nether_brick",
    "sandstone",
    "red_sandstone",
    "prismarine",
    "blackstone",
    "polished_blackstone",
    "polished_blackstone_brick",
    "end_stone_brick",
    "tuff",
    "polished_tuff",
    "tuff_brick",
    "resin_brick",
];

/// Lo que se sabe de un bloque.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoBloque {
    /// `minecraft:stone`.
    pub id: String,
    pub forma: FormaBloque,
    /// Color de su estado por defecto; `None` si no tiene textura opaca.
    pub color: Option<[u8; 3]>,
    /// `color` en OKLab.
    pub oklab: Option<Oklab>,
    /// Propiedades admitidas con sus valores y su defecto (vacío sin jar en
    /// las formas sin plantilla).
    pub propiedades: Vec<Propiedad>,
    /// [`es_sugerible`] con la política por defecto.
    pub sugerible: bool,
}

impl InfoBloque {
    pub fn propiedad(&self, nombre: &str) -> Option<&Propiedad> {
        self.propiedades.iter().find(|p| p.nombre == nombre)
    }

    /// Estado por defecto: todas sus propiedades con su valor por defecto,
    /// ordenadas por nombre (`minecraft:oak_stairs[facing=north,half=bottom,…]`).
    pub fn estado_por_defecto(&self) -> String {
        self.estado_con(&[])
    }

    /// Estado de este bloque conservando las propiedades de otro estado: cada
    /// propiedad que admite este bloque toma el valor de `props_origen` si lo
    /// trae y es un valor admitido; si no, su defecto. Las de `props_origen`
    /// que este bloque no tiene se descartan.
    pub fn estado_con(&self, props_origen: &[(&str, &str)]) -> String {
        let props = self.propiedades.iter().map(|p| {
            let valor = props_origen
                .iter()
                .find(|(k, v)| *k == p.nombre && p.admite(v))
                .map_or(p.defecto.as_str(), |(_, v)| *v);
            (p.nombre.as_str(), valor)
        });
        construir_estado(&self.id, props)
    }

    /// Como [`InfoBloque::estado_con`] tomando las propiedades de un estado
    /// en texto (`minecraft:stone_stairs[facing=east,half=top]`).
    pub fn estado_desde(&self, estado_origen: &str) -> String {
        self.estado_con(&separar(estado_origen).1)
    }
}

/// Bloques conocidos de una versión (o de la tabla integrada) con su forma,
/// propiedades y color.
pub struct Catalogo {
    /// Versión de Minecraft del jar; `None` = tabla integrada (sin jar).
    version: Option<String>,
    /// Tamaño del jar (para saber si el catálogo sigue valiendo).
    bytes_jar: u64,
    /// `minecraft:stone` → info.
    bloques: BTreeMap<String, InfoBloque>,
    /// Atlas activo y medias por textura (solo con jar).
    jar: Option<(Arc<Activo>, HashMap<String, MediaTextura>)>,
    /// Colores por estado ya calculados.
    memo: Mutex<HashMap<String, Option<[u8; 3]>>>,
    /// Familias de material (se calculan la primera vez que hacen falta).
    familias: OnceLock<BTreeMap<String, Vec<Miembro>>>,
}

impl Catalogo {
    /// Catálogo sin jar: los bloques de `colores_bloques.json`, con la forma
    /// deducida del id y las propiedades de la plantilla de su forma.
    ///
    /// Los muros no están en esa tabla (su blockstate no pinta nada sin
    /// `up=true`): se añaden los de [`MUROS_VANILLA`] con el color de su
    /// material (`<base>` o `<base>s`: `stone_brick` → `stone_bricks`).
    pub fn integrado() -> Self {
        let tabla = colores_integrados();
        let muros = MUROS_VANILLA.iter().filter_map(|base| {
            let rgb = [format!("minecraft:{base}"), format!("minecraft:{base}s")]
                .iter()
                .find_map(|k| tabla.get(k).copied())?;
            Some((format!("minecraft:{base}_wall"), rgb))
        });
        let todos: Vec<(String, [u8; 3])> = tabla
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .chain(muros)
            .collect();
        let bloques = todos
            .into_iter()
            .map(|(id, rgb)| {
                let forma = forma::forma_sin_jar(&id);
                let info = InfoBloque {
                    forma,
                    color: Some(rgb),
                    oklab: Some(Oklab::desde_rgb(rgb)),
                    propiedades: propiedades::plantilla(forma, &id),
                    sugerible: es_sugerible(&id),
                    id: id.clone(),
                };
                (id, info)
            })
            .collect();
        Catalogo {
            version: None,
            bytes_jar: 0,
            bloques,
            jar: None,
            memo: Mutex::new(HashMap::new()),
            familias: OnceLock::new(),
        }
    }

    /// Catálogo con el jar del atlas activo. `dir_cache` es
    /// `<app_cache_dir>/texturas` (`GestorTexturas::dir_cache`). Lee
    /// `colores.json` si vale; si no, calcula las medias desde el PNG del
    /// atlas y lo guarda. Nunca falla: si el PNG no se puede leer, usa los
    /// colores medios del atlas (ponderados por alfa) y no guarda caché.
    pub fn con_activo(activo: Arc<Activo>, dir_cache: &Path) -> Self {
        let meta = &activo.meta;
        let guardada = color::leer_cache(dir_cache, meta);
        let medias: HashMap<String, MediaTextura> = match &guardada {
            Some(c) => c.texturas.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            None => color::medias_atlas(&activo.png, meta).unwrap_or_else(|_| {
                meta.texturas
                    .iter()
                    .filter_map(|(k, &i)| {
                        let rgb = *meta.colores.get(i as usize)?;
                        Some((k.clone(), MediaTextura { rgb, n: 256 }))
                    })
                    .collect()
            }),
        };

        let r = &activo.recursos;
        let mut nombres: Vec<&str> = r.bloques().collect();
        nombres.sort_unstable();
        let mut bloques = BTreeMap::new();
        for nombre in nombres {
            let id = format!("minecraft:{nombre}");
            let por_sufijo = forma::forma_por_sufijo(nombre);
            let derivadas = r.blockstate(nombre).map_or_else(Vec::new, |bs| {
                propiedades::desde_blockstate(bs, por_sufijo.unwrap_or(FormaBloque::Otro))
            });
            let defectos: Vec<(&str, &str)> = derivadas
                .iter()
                .map(|p| (p.nombre.as_str(), p.defecto.as_str()))
                .collect();
            let forma = forma::forma_con_recursos(r, nombre, &defectos);
            let props = propiedades::combinar(derivadas, propiedades::plantilla(forma, &id));
            let mut info = InfoBloque {
                sugerible: es_sugerible(&id),
                id: id.clone(),
                forma,
                color: None,
                oklab: None,
                propiedades: props,
            };
            info.color = match &guardada {
                Some(c) => c.bloques.get(&id).copied(),
                None => color_de_estado(&activo, &medias, &info.estado_por_defecto()),
            };
            info.oklab = info.color.map(Oklab::desde_rgb);
            bloques.insert(id, info);
        }

        if guardada.is_none() && !medias.is_empty() {
            let cache = CacheColores {
                esquema: color::ESQUEMA_CACHE,
                version: meta.version.clone(),
                bytes_jar: meta.bytes_jar,
                umbral_alfa: color::UMBRAL_ALFA,
                texturas: medias.iter().map(|(k, v)| (k.clone(), *v)).collect(),
                bloques: bloques
                    .values()
                    .filter_map(|b| Some((b.id.clone(), b.color?)))
                    .collect(),
            };
            // Si no se puede guardar, se recalcula la próxima vez.
            let _ = color::guardar_cache(dir_cache, &cache);
        }

        Catalogo {
            version: Some(meta.version.clone()),
            bytes_jar: meta.bytes_jar,
            bloques,
            jar: Some((Arc::clone(&activo), medias)),
            memo: Mutex::new(HashMap::new()),
            familias: OnceLock::new(),
        }
    }

    /// Con jar si hay atlas activo; si no, el integrado.
    pub fn nuevo(activo: Option<Arc<Activo>>, dir_cache: &Path) -> Self {
        match activo {
            Some(a) => Catalogo::con_activo(a, dir_cache),
            None => Catalogo::integrado(),
        }
    }

    /// Versión de Minecraft de las texturas (`None` sin jar).
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[cfg(test)]
    pub fn con_jar(&self) -> bool {
        self.jar.is_some()
    }

    /// Todos los bloques conocidos, por id.
    pub fn bloques(&self) -> impl Iterator<Item = &InfoBloque> {
        self.bloques.values()
    }

    pub fn len(&self) -> usize {
        self.bloques.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bloques.is_empty()
    }

    /// Info de un bloque por id o por estado (se ignoran las propiedades).
    pub fn bloque(&self, id_o_estado: &str) -> Option<&InfoBloque> {
        self.bloques.get(nombre_de(id_o_estado))
    }

    /// Forma de un bloque (`Otro` si no se conoce).
    pub fn forma(&self, id_o_estado: &str) -> FormaBloque {
        self.bloque(id_o_estado)
            .map_or(FormaBloque::Otro, |b| b.forma)
    }

    /// Color de un block state concreto (`minecraft:oak_log[axis=x]`). Con
    /// jar se resuelven sus caras (si no tiene ninguna textura opaca, el color
    /// del bloque); sin jar, el color del bloque. `None` si el bloque no se
    /// conoce (mods) o no tiene color.
    pub fn color_estado(&self, estado: &str) -> Option<[u8; 3]> {
        let info = self.bloque(estado)?;
        let Some((activo, medias)) = &self.jar else {
            return info.color;
        };
        let mut memo = self.memo.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = memo.get(estado) {
            return *c;
        }
        let c = color_de_estado(activo, medias, estado).or(info.color);
        memo.insert(estado.to_string(), c);
        c
    }

    /// [`Catalogo::color_estado`] en OKLab.
    pub fn oklab_estado(&self, estado: &str) -> Option<Oklab> {
        self.color_estado(estado).map(Oklab::desde_rgb)
    }

    /// ¿Sirve este catálogo para el atlas `activo` (o la falta de él)?
    fn vale_para(&self, activo: Option<&Activo>) -> bool {
        match activo {
            Some(a) => {
                self.version.as_deref() == Some(a.meta.version.as_str())
                    && self.bytes_jar == a.meta.bytes_jar
            }
            None => self.version.is_none(),
        }
    }
}

/// Catálogo en memoria (para el estado de Tauri): se rehace solo cuando
/// cambia el atlas activo.
#[derive(Default)]
pub struct CacheCatalogo(Mutex<Option<Arc<Catalogo>>>);

impl CacheCatalogo {
    /// Catálogo para el atlas `activo` (`GestorTexturas::listo()` o
    /// `preparar()`) y `dir_cache` (`GestorTexturas::dir_cache()`). Bloquea
    /// la primera vez de cada versión: llamar desde un hilo de fondo.
    pub fn obtener(&self, activo: Option<Arc<Activo>>, dir_cache: &Path) -> Arc<Catalogo> {
        let mut actual = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = actual.as_ref()
            && c.vale_para(activo.as_deref())
        {
            return Arc::clone(c);
        }
        let c = Arc::new(Catalogo::nuevo(activo, dir_cache));
        *actual = Some(Arc::clone(&c));
        c
    }
}
