// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Texturas reales del visor, sacadas del jar de Minecraft instalado en la
//! máquina del usuario. Nada de esto se redistribuye: el atlas se genera y se
//! guarda en la caché de la app (`<app_cache_dir>/texturas/<versión>/`).
//!
//! - [`versiones`]: qué versiones hay instaladas.
//! - [`modelos`]: de block state y cara a textura (blockstates + modelos).
//! - [`atlas`]: lectura del jar y montaje del PNG.
//! - [`nombres`]: nombres de los ítems en inglés (`nombres_en.json`).
//! - [`GestorTexturas`]: ajustes, caché en disco y atlas activo.

pub mod atlas;
pub mod modelos;
pub mod nombres;
pub mod versiones;

#[cfg(test)]
pub(crate) mod pruebas;

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::ErrorApp;
use crate::malla::apariencia::{ESQUEMA_COLORES, TablaColores};
use crate::malla::{Cara, DatosAtlas, FuenteTexturas, TexturaCara};

use atlas::{Atlas, ESQUEMA_ATLAS, MetaAtlas};
use modelos::Recursos;
use versiones::VersionMinecraft;

/// Tope por entrada del jar. Las texturas y el idioma ocupan como mucho unos
/// cientos de KB; un jar manipulado no puede forzar reservas mayores.
const MAX_ENTRADA_JAR: u64 = 16 * 1024 * 1024;

/// Lee una entrada del jar sin fiarse del tamaño que declara su cabecera.
pub(crate) fn leer_entrada_jar(entrada: impl Read, declarado: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(declarado.min(MAX_ENTRADA_JAR) as usize);
    entrada.take(MAX_ENTRADA_JAR + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ENTRADA_JAR {
        return Err(io::Error::other("entrada del jar demasiado grande"));
    }
    Ok(bytes)
}

/// Tema de la interfaz. `Sistema` sigue `prefers-color-scheme`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tema {
    Claro,
    Oscuro,
    #[default]
    Sistema,
}

/// `ajustes.json` (en `app_config_dir`). Todos los campos tienen valor por
/// defecto: un archivo de una versión anterior (sin `tema` ni
/// `incluirFluidos`) se sigue leyendo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ajustes {
    /// Versión de Minecraft de la que salen las texturas; `None` = la
    /// release más reciente instalada.
    #[serde(default)]
    pub version_minecraft: Option<String>,
    /// Carpeta `.minecraft` elegida a mano; sin ella se buscan las habituales.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruta_minecraft: Option<String>,
    /// Tema de la interfaz (solo lo usa el frontend).
    #[serde(default)]
    pub tema: Tema,
    /// Contar agua y lava en la lista de materiales (`Opciones::incluir_fluidos`).
    #[serde(default)]
    pub incluir_fluidos: bool,
}

impl Ajustes {
    /// ¿Cambia algo que afecte a las texturas o a los nombres de los ítems?
    fn mismo_minecraft(&self, otros: &Ajustes) -> bool {
        self.version_minecraft == otros.version_minecraft
            && self.ruta_minecraft == otros.ruta_minecraft
    }
}

/// Id de ítem → nombre en inglés.
pub type MapaNombres = HashMap<String, String>;

/// Atlas cargado y listo para resolver texturas.
pub struct Activo {
    pub meta: MetaAtlas,
    pub png: Arc<Vec<u8>>,
    pub recursos: Recursos,
    /// Resoluciones ya hechas (estado visual, cara).
    memo: Mutex<HashMap<(String, Cara), Option<TexturaCara>>>,
}

impl Activo {
    pub fn nuevo(meta: MetaAtlas, png: Vec<u8>, recursos: Recursos) -> Self {
        Activo {
            meta,
            png: Arc::new(png),
            recursos,
            memo: Mutex::new(HashMap::new()),
        }
    }
}

impl FuenteTexturas for Activo {
    fn textura(&self, estado: &str, cara: Cara) -> Option<TexturaCara> {
        let clave = (estado.to_string(), cara);
        if let Some(t) = self
            .memo
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&clave)
        {
            return *t;
        }
        let t = self.recursos.resolver(estado, cara).and_then(|r| {
            let celda = *self.meta.texturas.get(&r.textura)?;
            Some(TexturaCara {
                celda,
                rotacion: r.rotacion,
                color: self.meta.colores.get(celda as usize).copied()?,
            })
        });
        self.memo
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(clave, t);
        t
    }

    fn datos_atlas(&self) -> DatosAtlas {
        self.meta.datos()
    }
}

/// Ajustes, versiones y atlas activo. Vive en el estado de Tauri.
pub struct GestorTexturas {
    /// `<app_cache_dir>/texturas`.
    dir_cache: PathBuf,
    /// `<app_config_dir>/ajustes.json`.
    ruta_ajustes: PathBuf,
    ajustes: Mutex<Ajustes>,
    /// Atlas listo (o `None` si aún no se ha preparado o no hay jar).
    activo: Mutex<Option<Arc<Activo>>>,
    /// Solo una preparación a la vez.
    preparando: Mutex<()>,
    /// Nombres en inglés de la última versión pedida: (versión, mapa).
    nombres: Mutex<Option<(String, Arc<MapaNombres>)>>,
}

impl GestorTexturas {
    /// Lee los ajustes (si faltan o están dañados, los de por defecto).
    pub fn nuevo(dir_cache: &Path, dir_config: &Path) -> Self {
        let ruta_ajustes = dir_config.join("ajustes.json");
        let ajustes = fs::read(&ruta_ajustes)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        GestorTexturas {
            dir_cache: dir_cache.join("texturas"),
            ruta_ajustes,
            ajustes: Mutex::new(ajustes),
            activo: Mutex::new(None),
            preparando: Mutex::new(()),
            nombres: Mutex::new(None),
        }
    }

    /// `<app_cache_dir>/texturas`: aquí cuelga la carpeta de cada versión.
    pub fn dir_cache(&self) -> &Path {
        &self.dir_cache
    }

    pub fn ajustes(&self) -> Ajustes {
        self.ajustes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Guarda los ajustes y devuelve si cambian las texturas: si la versión
    /// que se usa de verdad (id y jar) ya no es la misma, se descartan el
    /// atlas activo y los nombres en memoria (la próxima preparación carga
    /// los nuevos). Cambiar el tema o los fluidos no toca nada de eso.
    ///
    /// `ruta_minecraft` vacía cuenta como `None`; si no, tiene que ser una
    /// carpeta que exista y tenga `versions/` (si no, error y no se guarda).
    pub fn guardar_ajustes(&self, mut nuevos: Ajustes) -> Result<bool, ErrorApp> {
        nuevos.ruta_minecraft = nuevos
            .ruta_minecraft
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());
        if let Some(r) = &nuevos.ruta_minecraft {
            versiones::validar_carpeta(Path::new(r)).map_err(ErrorApp::CarpetaMinecraft)?;
        }
        if let Some(dir) = self.ruta_ajustes.parent() {
            fs::create_dir_all(dir).map_err(|e| ErrorApp::io("crear la carpeta de ajustes", e))?;
        }
        let json =
            serde_json::to_vec_pretty(&nuevos).map_err(|e| ErrorApp::Interno(e.to_string()))?;
        let tmp = self.ruta_ajustes.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| ErrorApp::io("guardar los ajustes", e))?;
        fs::rename(&tmp, &self.ruta_ajustes).map_err(|e| ErrorApp::io("guardar los ajustes", e))?;

        let mismo_minecraft = self.ajustes().mismo_minecraft(&nuevos);
        if mismo_minecraft {
            *self.ajustes.lock().unwrap_or_else(|e| e.into_inner()) = nuevos;
            return Ok(false);
        }
        let antes = self.version_elegida();
        *self.ajustes.lock().unwrap_or_else(|e| e.into_inner()) = nuevos;
        let despues = self.version_elegida();
        let cambian = antes != despues;
        if cambian {
            *self.activo.lock().unwrap_or_else(|e| e.into_inner()) = None;
            *self.nombres.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
        Ok(cambian)
    }

    /// Carpetas `.minecraft` donde buscar.
    fn carpetas(&self) -> Vec<PathBuf> {
        match self.ajustes().ruta_minecraft {
            Some(r) => vec![PathBuf::from(r)],
            None => versiones::carpetas_por_defecto(),
        }
    }

    /// Versiones instaladas con jar, ordenadas (releases y recientes primero).
    pub fn versiones(&self) -> Vec<VersionMinecraft> {
        versiones::listar(&self.carpetas())
    }

    /// Versión que se usa: la de los ajustes si está instalada; si no, la
    /// primera de la lista.
    pub fn version_elegida(&self) -> Option<VersionMinecraft> {
        let lista = self.versiones();
        let pedida = self.ajustes().version_minecraft;
        pedida
            .and_then(|p| lista.iter().find(|v| v.id == p).cloned())
            .or_else(|| lista.into_iter().next())
    }

    /// Atlas activo si ya está preparado (no bloquea ni genera nada).
    pub fn listo(&self) -> Option<Arc<Activo>> {
        self.activo
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Prepara el atlas de la versión elegida: de la caché o generándolo.
    /// Bloquea (llamar desde un hilo de fondo). `None` si no hay ningún jar.
    pub fn preparar(&self) -> Result<Option<Arc<Activo>>, ErrorApp> {
        let _turno = self.preparando.lock().unwrap_or_else(|e| e.into_inner());
        // Los ajustes pueden cambiar mientras se prepara: se repite si la
        // versión elegida ya no es la misma al terminar.
        for _ in 0..3 {
            let Some(v) = self.version_elegida() else {
                return Ok(None);
            };
            if let Some(a) = self.listo().filter(|a| a.meta.version == v.id) {
                return Ok(Some(a));
            }
            let activo = Arc::new(cargar_o_generar(&self.dir_cache, &v)?);
            if self.version_elegida().is_some_and(|e| e.id == v.id) {
                *self.activo.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&activo));
                return Ok(Some(activo));
            }
        }
        Ok(self.listo())
    }
}

impl GestorTexturas {
    /// Nombres en inglés de los ítems de la versión elegida: de memoria, de
    /// la caché en disco o leídos de la instalación. Vacío si no hay
    /// Minecraft o no trae `en_us.json`. Nunca falla. Bloquea la primera vez.
    pub fn nombres_items(&self) -> Arc<MapaNombres> {
        let Some(v) = self.version_elegida() else {
            return Arc::default();
        };
        let mut memo = self.nombres.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((version, mapa)) = memo.as_ref()
            && *version == v.id
        {
            return Arc::clone(mapa);
        }
        let mapa = Arc::new(nombres_con_cache(&self.dir_cache, &v));
        *memo = Some((v.id.clone(), Arc::clone(&mapa)));
        mapa
    }
}

/// Nombres de una versión: de `<dir_cache>/<versión>/nombres_en.json` si vale;
/// si no, leídos de la instalación y guardados ahí (solo si no salen vacíos,
/// para volver a mirar cuando el launcher descargue los assets).
pub fn nombres_con_cache(dir_cache: &Path, v: &VersionMinecraft) -> MapaNombres {
    let dir = dir_cache.join(carpeta_version(&v.id));
    let ruta = dir.join(nombres::ARCHIVO_CACHE);
    let guardado = fs::read(&ruta)
        .ok()
        .and_then(|b| serde_json::from_slice::<nombres::CacheNombres>(&b).ok())
        .filter(|c| c.esquema == nombres::ESQUEMA_NOMBRES && c.version == v.id);
    if let Some(c) = guardado {
        return c.nombres;
    }
    let mapa = nombres::leer(Path::new(&v.ruta));
    if !mapa.is_empty() {
        let cache = nombres::CacheNombres {
            esquema: nombres::ESQUEMA_NOMBRES,
            version: v.id.clone(),
            nombres: mapa,
        };
        // Si no se puede guardar, se vuelve a leer la próxima vez.
        if let Ok(json) = serde_json::to_vec(&cache)
            && fs::create_dir_all(&dir).is_ok()
        {
            let _ = escribir(&ruta, &json);
        }
        return cache.nombres;
    }
    mapa
}

/// Nombre de carpeta seguro para una versión.
pub(crate) fn carpeta_version(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Carga el atlas de la caché si vale (mismo esquema, versión y tamaño de
/// jar); si no, lo genera desde el jar y lo guarda.
pub fn cargar_o_generar(dir_cache: &Path, v: &VersionMinecraft) -> Result<Activo, ErrorApp> {
    let jar = Path::new(&v.ruta);
    let corrupto = |detalle: String| ErrorApp::ArchivoCorrupto {
        archivo: jar
            .file_name()
            .map_or_else(|| v.ruta.clone(), |n| n.to_string_lossy().into_owned()),
        detalle: Some(detalle),
    };
    let bytes_jar = fs::metadata(jar)
        .map_err(|e| ErrorApp::io("leer el jar de Minecraft", e))?
        .len();
    let dir = dir_cache.join(carpeta_version(&v.id));
    let (ruta_png, ruta_json) = (dir.join("atlas.png"), dir.join("atlas.json"));

    let guardado = fs::read(&ruta_json)
        .ok()
        .and_then(|b| serde_json::from_slice::<MetaAtlas>(&b).ok())
        .filter(|m| m.esquema == ESQUEMA_ATLAS && m.version == v.id && m.bytes_jar == bytes_jar);
    if let Some(meta) = guardado
        && let Ok(png) = fs::read(&ruta_png)
    {
        let contenido = atlas::leer_jar(jar, false).map_err(corrupto)?;
        return Ok(Activo::nuevo(meta, png, contenido.recursos));
    }

    let (activo, _) = generar(jar, &v.id, bytes_jar).map_err(corrupto)?;
    fs::create_dir_all(&dir).map_err(|e| ErrorApp::io("crear la caché de texturas", e))?;
    // Primero el PNG y después el JSON: un JSON válido implica PNG completo.
    escribir(&ruta_png, &activo.png).map_err(|e| ErrorApp::io("guardar el atlas", e))?;
    let json = serde_json::to_vec(&activo.meta).map_err(|e| ErrorApp::Interno(e.to_string()))?;
    escribir(&ruta_json, &json).map_err(|e| ErrorApp::io("guardar el atlas", e))?;
    Ok(activo)
}

/// Escribe a un temporal y lo renombra.
pub(crate) fn escribir(ruta: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = ruta.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, ruta)
}

/// Genera el atlas desde el jar (sin tocar disco). Devuelve también las
/// texturas decodificadas por celda.
pub fn generar(jar: &Path, version: &str, bytes_jar: u64) -> Result<(Activo, Atlas), String> {
    let contenido = atlas::leer_jar(jar, true)?;
    let tintes = atlas::mapa_tintes(&contenido.recursos);
    let mut a = atlas::montar(version, bytes_jar, &contenido.pngs, &tintes)?;
    let activo = Activo::nuevo(
        a.meta.clone(),
        std::mem::take(&mut a.png),
        contenido.recursos,
    );
    Ok((activo, a))
}

/// Tabla de colores medios por bloque (`colores_bloques.json`): media
/// ponderada por alfa de la textura de arriba y la de un lado (norte) del
/// estado por defecto. Los bloques sin textura no entran.
pub fn tabla_colores(activo: &Activo, a: &Atlas) -> TablaColores {
    let mut colores = BTreeMap::new();
    for bloque in activo.recursos.bloques() {
        let estado = format!("minecraft:{bloque}");
        let mut caras: Vec<TexturaCara> = [Cara::Arriba, Cara::Norte]
            .into_iter()
            .filter_map(|c| activo.textura(&estado, c))
            .collect();
        if caras.is_empty() {
            caras.extend(activo.textura(&estado, Cara::Cruz));
        }
        let mut suma = [0u64; 3];
        let mut peso = 0u64;
        for t in caras {
            for px in a.texturas[t.celda as usize].as_chunks::<4>().0 {
                let al = u64::from(px[3]);
                for c in 0..3 {
                    suma[c] += u64::from(px[c]) * al;
                }
                peso += al;
            }
        }
        if peso > 0 {
            colores.insert(estado, suma.map(|s| ((s + peso / 2) / peso) as u8));
        }
    }
    TablaColores {
        esquema: ESQUEMA_COLORES,
        version: Some(activo.meta.version.clone()),
        colores,
    }
}
