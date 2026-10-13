// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos Tauri que usa el frontend. El trabajo pesado (leer, descomprimir,
//! calcular el hash, SQLite) va a un hilo bloqueante para no frenar la interfaz.
//! Las miniaturas: `docs/miniaturas.md`. La edición y los parecidos:
//! `docs/edicion.md`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, Manager, State};

use crate::biblioteca::{Biblioteca, Detalle, Edicion, Entrada, Importacion, Version};
use crate::edicion::{Cambio, Reemplazo, validar_estado};
use crate::error::ErrorApp;
use crate::malla::{self, BloqueSeleccionado, CacheVisor, DatosAtlas, FuenteTexturas, Mundo};
use crate::materiales::{self, ListaMateriales, Opciones};
use crate::parecidos::{
    CacheCatalogo, Catalogo, EstadoPresente, FamiliaBloque, FamiliaParecida, Parecido,
    miembros_en_schematic,
};
use crate::texturas::versiones::{self, VersionMinecraft};
use crate::texturas::{Ajustes, GestorTexturas};

/// Resultado de importar un archivo. El fallo de uno no aborta los demás.
#[derive(Debug, Serialize)]
#[serde(tag = "estado", rename_all = "camelCase")]
pub enum ResultadoImportacion {
    Importado { entrada: Entrada },
    Duplicado { entrada: Entrada },
    Error { ruta: String, error: ErrorApp },
}

impl ResultadoImportacion {
    fn de(ruta: String, r: Result<Importacion, ErrorApp>) -> Self {
        match r {
            Ok(Importacion::Importado(entrada)) => ResultadoImportacion::Importado { entrada },
            Ok(Importacion::Duplicado(entrada)) => ResultadoImportacion::Duplicado { entrada },
            Err(error) => ResultadoImportacion::Error { ruta, error },
        }
    }
}

/// Ejecuta `f` en el grupo de hilos bloqueantes de Tauri.
async fn en_segundo_plano<T, F>(f: F) -> Result<T, ErrorApp>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ErrorApp> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| ErrorApp::Interno(e.to_string()))?
}

/// Archivo con el que se arrancó la app (doble clic en un schematic), si lo
/// hay. Se entrega una sola vez.
#[tauri::command]
pub fn archivo_pendiente(
    estado: tauri::State<'_, crate::argumentos::ArchivoPendiente>,
) -> Option<String> {
    estado.0.lock().ok().and_then(|mut g| g.take())
}

/// Importa varios archivos; devuelve un resultado por ruta, en el mismo orden.
#[tauri::command]
pub async fn importar(app: AppHandle, rutas: Vec<String>) -> Vec<ResultadoImportacion> {
    let Some(biblioteca) = app
        .try_state::<Arc<Biblioteca>>()
        .map(|s| s.inner().clone())
    else {
        return rutas
            .into_iter()
            .map(|ruta| ResultadoImportacion::Error {
                ruta,
                error: ErrorApp::Interno("biblioteca sin abrir".into()),
            })
            .collect();
    };
    let copia = rutas.clone();
    let trabajo = tauri::async_runtime::spawn_blocking(move || {
        copia
            .into_iter()
            .map(|ruta| {
                let r = biblioteca.importar(&PathBuf::from(&ruta));
                ResultadoImportacion::de(ruta, r)
            })
            .collect::<Vec<_>>()
    });
    match trabajo.await {
        Ok(resultados) => resultados,
        Err(e) => rutas
            .into_iter()
            .map(|ruta| ResultadoImportacion::Error {
                ruta,
                error: ErrorApp::Interno(e.to_string()),
            })
            .collect(),
    }
}

#[tauri::command]
pub async fn listar(biblioteca: State<'_, Arc<Biblioteca>>) -> Result<Vec<Entrada>, ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.listar()).await
}

#[tauri::command]
pub async fn obtener(
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
) -> Result<Detalle, ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.obtener(&id)).await
}

#[tauri::command]
pub async fn renombrar(
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
    nombre: String,
) -> Result<Entrada, ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.renombrar(&id, &nombre)).await
}

#[tauri::command]
pub async fn eliminar(
    biblioteca: State<'_, Arc<Biblioteca>>,
    cache: State<'_, Arc<CacheVisor>>,
    id: String,
) -> Result<(), ErrorApp> {
    cache.olvidar(&id);
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.eliminar(&id)).await
}

/// Lista de materiales de un schematic de la biblioteca.
#[tauri::command]
pub async fn materiales(
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
    opciones: Opciones,
) -> Result<ListaMateriales, ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || {
        let (_, schem) = b.cargar(&id)?;
        materiales::calcular(&schem, &opciones)
    })
    .await
}

/// Ítems marcados como reunidos en la pantalla de materiales de un schematic.
#[tauri::command]
pub async fn comprobacion(
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
) -> Result<Vec<String>, ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.comprobacion(&id)).await
}

/// Marca o desmarca un ítem como reunido (queda guardado en la biblioteca).
#[tauri::command]
pub async fn marcar(
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
    item: String,
    reunido: bool,
) -> Result<(), ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.marcar(&id, &item, reunido)).await
}

/// Id de ítem (`minecraft:stone`) → nombre en inglés (`Stone`), del
/// `en_us.json` del jar de la versión de Minecraft elegida. Vacío si no lo hay;
/// nunca es un error.
#[tauri::command]
pub async fn nombres_items(app: AppHandle) -> HashMap<String, String> {
    // Sin `State` (un comando async con referencias tendría que devolver
    // `Result`) para que el contrato sea el mapa a secas.
    let Some(g) = app
        .try_state::<Arc<GestorTexturas>>()
        .map(|s| s.inner().clone())
    else {
        return HashMap::new();
    };
    tauri::async_runtime::spawn_blocking(move || g.nombres_items().as_ref().clone())
        .await
        .unwrap_or_default()
}

/// Extensiones que acepta `guardar_texto`.
const EXTENSIONES_TEXTO: [&str; 2] = ["csv", "txt"];

/// Escribe `contenido` (UTF-8, tal cual) en `ruta`, que tiene que ser
/// absoluta y acabar en `.csv` o `.txt`.
pub fn escribir_texto(ruta: &Path, contenido: &str) -> Result<(), ErrorApp> {
    let ext = ruta
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if !ruta.is_absolute() || !ext.is_some_and(|e| EXTENSIONES_TEXTO.contains(&e.as_str())) {
        return Err(ErrorApp::NombreInvalido(
            "Solo se puede guardar en un archivo .csv o .txt con la ruta completa.".into(),
        ));
    }
    fs::write(ruta, contenido).map_err(|e| ErrorApp::io("guardar el archivo", e))
}

/// Guarda un archivo de texto en la ruta elegida con el diálogo de guardar
/// (la exportación CSV de la lista de materiales).
#[tauri::command]
pub async fn guardar_texto(ruta: String, contenido: String) -> Result<(), ErrorApp> {
    en_segundo_plano(move || escribir_texto(Path::new(&ruta), &contenido)).await
}

/// Aplica cambios puntuales y reemplazos a la copia de la biblioteca
/// (docs/edicion.md). La versión anterior queda en el historial.
#[tauri::command]
pub async fn aplicar_cambios(
    biblioteca: State<'_, Arc<Biblioteca>>,
    cache: State<'_, Arc<CacheVisor>>,
    id: String,
    cambios: Vec<Cambio>,
    reemplazos: Vec<Reemplazo>,
) -> Result<Edicion, ErrorApp> {
    let b = biblioteca.inner().clone();
    let c = cache.inner().clone();
    en_segundo_plano(move || {
        let r = b.aplicar_cambios(&id, &cambios, &reemplazos);
        // El visor tiene que volver a leer el archivo.
        c.olvidar(&id);
        r
    })
    .await
}

/// Versiones anteriores de un schematic, de la más reciente a la más antigua.
#[tauri::command]
pub async fn historial(
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
) -> Result<Vec<Version>, ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.historial(&id)).await
}

/// Vuelve a una versión del historial (la actual pasa antes al historial).
#[tauri::command]
pub async fn restaurar_version(
    biblioteca: State<'_, Arc<Biblioteca>>,
    cache: State<'_, Arc<CacheVisor>>,
    id: String,
    version: i64,
) -> Result<Edicion, ErrorApp> {
    let b = biblioteca.inner().clone();
    let c = cache.inner().clone();
    en_segundo_plano(move || {
        let r = b.restaurar_version(&id, version);
        c.olvidar(&id);
        r
    })
    .await
}

/// Escribe la copia actual como .litematic en la ruta elegida con el diálogo
/// de guardar (absoluta y acabada en `.litematic`).
#[tauri::command]
pub async fn exportar_schematic(
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
    ruta: String,
) -> Result<(), ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.exportar(&id, Path::new(&ruta))).await
}

/// Cuántos parecidos o familias se devuelven si no se dice.
pub const PARECIDOS_POR_DEFECTO: usize = 12;
/// Máximo de parecidos o familias en una llamada.
pub const MAX_PARECIDOS: usize = 100;
/// Longitud máxima del nombre de una familia, en bytes.
pub const MAX_LONGITUD_FAMILIA: usize = 128;

/// Catálogo de bloques para el atlas de la versión elegida (espera a que
/// esté preparado; sin jar, el integrado). Bloquea: solo en segundo plano.
fn catalogo(g: &GestorTexturas, cache: &CacheCatalogo) -> Arc<Catalogo> {
    let activo = g.preparar().ok().flatten();
    cache.obtener(activo, g.dir_cache())
}

/// `n` acotado a `1..=MAX_PARECIDOS` (por defecto, `PARECIDOS_POR_DEFECTO`).
fn cuantos(n: Option<u32>) -> usize {
    n.map_or(PARECIDOS_POR_DEFECTO, |n| n as usize)
        .clamp(1, MAX_PARECIDOS)
}

/// Comprueba el nombre de una familia (`deepslate`, `mod:granito`).
pub fn validar_familia(familia: &str) -> Result<String, ErrorApp> {
    let f = familia.trim();
    let ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.:/".contains(c);
    if f.is_empty() || f.len() > MAX_LONGITUD_FAMILIA || !f.chars().all(ok) {
        let mostrado: String = f.chars().take(80).collect();
        return Err(ErrorApp::EdicionInvalida(format!(
            "«{mostrado}» no es un nombre de familia válido."
        )));
    }
    Ok(f.to_string())
}

/// Los `n` bloques sugeribles de la misma forma más parecidos en color a
/// `estado`, con su estado destino ya construido (docs/edicion.md, «Parecidos»).
#[tauri::command]
pub async fn parecidos(
    texturas: State<'_, Arc<GestorTexturas>>,
    catalogos: State<'_, Arc<CacheCatalogo>>,
    estado: String,
    n: Option<u32>,
) -> Result<Vec<Parecido>, ErrorApp> {
    let g = texturas.inner().clone();
    let c = catalogos.inner().clone();
    en_segundo_plano(move || {
        let estado = validar_estado(&estado)?;
        Ok(catalogo(&g, &c).parecidos(&estado, cuantos(n)))
    })
    .await
}

/// Familia, variante y forma de un block state.
#[tauri::command]
pub async fn familia_de(
    texturas: State<'_, Arc<GestorTexturas>>,
    catalogos: State<'_, Arc<CacheCatalogo>>,
    estado: String,
) -> Result<FamiliaBloque, ErrorApp> {
    let g = texturas.inner().clone();
    let c = catalogos.inner().clone();
    en_segundo_plano(move || {
        let estado = validar_estado(&estado)?;
        Ok(catalogo(&g, &c).familia_bloque(&estado))
    })
    .await
}

/// Block states de una familia presentes en un schematic de la biblioteca,
/// con cuántas posiciones ocupa cada uno.
#[tauri::command]
pub async fn miembros_familia(
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
    familia: String,
) -> Result<Vec<EstadoPresente>, ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || {
        let familia = validar_familia(&familia)?;
        let (_, schem) = b.cargar(&id)?;
        Ok(miembros_en_schematic(&schem, &familia))
    })
    .await
}

/// Familias más parecidas a una (color medio y formas y variantes en común).
#[tauri::command]
pub async fn familias_parecidas(
    texturas: State<'_, Arc<GestorTexturas>>,
    catalogos: State<'_, Arc<CacheCatalogo>>,
    familia: String,
    n: Option<u32>,
) -> Result<Vec<FamiliaParecida>, ErrorApp> {
    let g = texturas.inner().clone();
    let c = catalogos.inner().clone();
    en_segundo_plano(move || {
        let familia = validar_familia(&familia)?;
        Ok(catalogo(&g, &c).familias_parecidas(&familia, cuantos(n)))
    })
    .await
}

/// Reemplazos para cambiar la familia `familia_origen` por `familia_destino`
/// en un schematic de la biblioteca; se aplican con `aplicar_cambios`.
#[tauri::command]
pub async fn plan_familia(
    biblioteca: State<'_, Arc<Biblioteca>>,
    texturas: State<'_, Arc<GestorTexturas>>,
    catalogos: State<'_, Arc<CacheCatalogo>>,
    id: String,
    familia_origen: String,
    familia_destino: String,
) -> Result<Vec<Reemplazo>, ErrorApp> {
    let b = biblioteca.inner().clone();
    let g = texturas.inner().clone();
    let c = catalogos.inner().clone();
    en_segundo_plano(move || {
        let origen = validar_familia(&familia_origen)?;
        let destino = validar_familia(&familia_destino)?;
        let cat = catalogo(&g, &c);
        if !cat.familias().contains_key(&destino) {
            return Err(ErrorApp::EdicionInvalida(format!(
                "No hay ninguna familia de bloques llamada «{destino}»."
            )));
        }
        let (_, schem) = b.cargar(&id)?;
        Ok(cat.plan_familia(&schem, &origen, &destino))
    })
    .await
}

/// Schematic combinado del id, leído y parseado del archivo (sin caché).
fn cargar_mundo(b: &Biblioteca, id: &str) -> Result<Mundo, ErrorApp> {
    let (entrada, schem) = b.cargar(id)?;
    Mundo::desde_schematic(&schem).map_err(|detalle| ErrorApp::DemasiadoGrande {
        archivo: entrada.nombre,
        detalle,
    })
}

/// Schematic combinado del id: de la caché o leído y parseado (y cacheado).
fn mundo_de(b: &Biblioteca, cache: &CacheVisor, id: &str) -> Result<Arc<Mundo>, ErrorApp> {
    if let Some(m) = cache.obtener(id) {
        return Ok(m);
    }
    let mundo = Arc::new(cargar_mundo(b, id)?);
    cache.guardar(id, Arc::clone(&mundo));
    Ok(mundo)
}

/// Malla 3D del schematic en el formato binario de `docs/visor.md`
/// (llega a JS como `ArrayBuffer`). `capas`: solo las capas Y de ese rango
/// (coordenadas del modelo, inclusivo); lo de fuera cuenta como aire.
///
/// Usa el atlas activo si ya está preparado; si no, sale solo con colores
/// (`atlas: null`) y deja la preparación en marcha en segundo plano.
#[tauri::command]
pub async fn malla(
    biblioteca: State<'_, Arc<Biblioteca>>,
    cache: State<'_, Arc<CacheVisor>>,
    texturas: State<'_, Arc<GestorTexturas>>,
    id: String,
    capas: Option<[i32; 2]>,
) -> Result<Response, ErrorApp> {
    let b = biblioteca.inner().clone();
    let c = cache.inner().clone();
    let activo = texturas.listo();
    if activo.is_none() {
        preparar_en_fondo(texturas.inner().clone());
    }
    en_segundo_plano(move || {
        let mundo = mundo_de(&b, &c, &id)?;
        let opciones = malla::Opciones {
            capas,
            ..Default::default()
        };
        let fuente = activo.as_deref().map(|a| a as &dyn FuenteTexturas);
        let m = malla::mallar_con_texturas(&mundo, &opciones, fuente).map_err(ErrorApp::Interno)?;
        Ok(Response::new(malla::binario::serializar(&m)))
    })
    .await
}

/// Malla del schematic entero para su miniatura (docs/miniaturas.md). A
/// diferencia de `malla`, espera a que el atlas esté preparado (si falla o no
/// hay jar, sale con colores) y no toca la caché del visor: generar miniaturas
/// en segundo plano no echa de ella el schematic que se está viendo.
#[tauri::command]
pub async fn malla_miniatura(
    biblioteca: State<'_, Arc<Biblioteca>>,
    cache: State<'_, Arc<CacheVisor>>,
    texturas: State<'_, Arc<GestorTexturas>>,
    id: String,
) -> Result<Response, ErrorApp> {
    let b = biblioteca.inner().clone();
    let c = cache.inner().clone();
    let g = texturas.inner().clone();
    en_segundo_plano(move || {
        let activo = g.preparar().ok().flatten();
        let mundo = match c.obtener(&id) {
            Some(m) => m,
            None => Arc::new(cargar_mundo(&b, &id)?),
        };
        let fuente = activo.as_deref().map(|a| a as &dyn FuenteTexturas);
        let m = malla::mallar_con_texturas(&mundo, &malla::Opciones::default(), fuente)
            .map_err(ErrorApp::Interno)?;
        Ok(Response::new(malla::binario::serializar(&m)))
    })
    .await
}

/// Cabeceras de `guardar_miniatura` (el cuerpo de la petición es el PNG).
pub const CABECERA_ID: &str = "miniatura-id";
pub const CABECERA_TEXTURAS: &str = "miniatura-texturas";

/// Guarda la miniatura de un schematic. El PNG llega como cuerpo binario de
/// la petición (sin pasar por JSON); el id y la clave de texturas, en las
/// cabeceras `miniatura-id` y `miniatura-texturas`. El archivo se llama como
/// el id: el frontend no elige ninguna ruta. Devuelve la entrada actualizada.
#[tauri::command]
pub async fn guardar_miniatura(
    biblioteca: State<'_, Arc<Biblioteca>>,
    peticion: Request<'_>,
) -> Result<Entrada, ErrorApp> {
    let InvokeBody::Raw(png) = peticion.body() else {
        return Err(ErrorApp::MiniaturaInvalida(
            "no ha llegado la imagen".into(),
        ));
    };
    let cabecera = |nombre: &str| {
        peticion
            .headers()
            .get(nombre)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let id = cabecera(CABECERA_ID).ok_or(ErrorApp::NoEncontrado)?;
    let texturas = cabecera(CABECERA_TEXTURAS)
        .ok_or_else(|| ErrorApp::MiniaturaInvalida("falta la clave de texturas".into()))?;
    let png = png.clone();
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.guardar_miniatura(&id, &png, &texturas)).await
}

/// Prepara el atlas de la versión elegida sin esperar el resultado (si
/// falla, `malla` sigue con colores y `atlas` devolverá el error).
pub fn preparar_en_fondo(gestor: Arc<GestorTexturas>) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = gestor.preparar();
    });
}

/// Versiones de Minecraft instaladas con jar, releases y recientes primero.
#[tauri::command]
pub async fn versiones_minecraft(
    texturas: State<'_, Arc<GestorTexturas>>,
) -> Result<Vec<VersionMinecraft>, ErrorApp> {
    let g = texturas.inner().clone();
    en_segundo_plano(move || Ok(g.versiones())).await
}

#[tauri::command]
pub fn ajustes(texturas: State<'_, Arc<GestorTexturas>>) -> Ajustes {
    texturas.ajustes()
}

/// Guarda los ajustes. Devuelve `true` si cambian las texturas (la versión
/// que se usa de verdad ya no es la misma): entonces se ha descartado el atlas
/// activo y el nuevo empieza a prepararse en segundo plano.
#[tauri::command]
pub async fn guardar_ajustes(
    texturas: State<'_, Arc<GestorTexturas>>,
    biblioteca: State<'_, Arc<Biblioteca>>,
    ajustes: Ajustes,
) -> Result<bool, ErrorApp> {
    let g = texturas.inner().clone();
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || {
        let cambian = g.guardar_ajustes(ajustes)?;
        if cambian {
            preparar_en_fondo(g);
            // Las miniaturas quedan obsoletas aquí mismo (no espera al
            // frontend): si se cierra la app antes de regenerarlas, al volver
            // a abrirla se sabe cuáles faltan. Un fallo aquí no deshace los
            // ajustes ya guardados: en esta sesión el frontend las regenera
            // igualmente.
            let _ = b.invalidar_miniaturas();
        }
        Ok(cambian)
    })
    .await
}

/// Carpeta `.minecraft` que se usa sin elegir ninguna: la primera habitual
/// que existe (`%APPDATA%\.minecraft` en Windows). `null` si no hay ninguna.
#[tauri::command]
pub fn carpeta_minecraft_por_defecto() -> Option<String> {
    versiones::carpetas_por_defecto()
        .into_iter()
        .next()
        .map(|p| p.to_string_lossy().into_owned())
}

/// PNG del atlas de la versión elegida (lo genera si hace falta, la primera
/// vez tarda unos segundos). `ArrayBuffer` vacío si no hay ningún jar.
#[tauri::command]
pub async fn atlas(texturas: State<'_, Arc<GestorTexturas>>) -> Result<Response, ErrorApp> {
    let g = texturas.inner().clone();
    en_segundo_plano(move || {
        let bytes = g
            .preparar()?
            .map_or_else(Vec::new, |a| a.png.as_ref().clone());
        Ok(Response::new(bytes))
    })
    .await
}

/// Datos del atlas activo (los mismos que `atlas` en la cabecera de la
/// malla) o `null` si aún no hay. No genera nada.
#[tauri::command]
pub fn atlas_info(texturas: State<'_, Arc<GestorTexturas>>) -> Option<DatosAtlas> {
    texturas.listo().map(|a| a.datos_atlas())
}

/// Bloque en una posición del modelo (las coordenadas de los vértices de la
/// malla). `None` si es aire o está fuera.
#[tauri::command]
pub async fn bloque_en(
    biblioteca: State<'_, Arc<Biblioteca>>,
    cache: State<'_, Arc<CacheVisor>>,
    id: String,
    x: i32,
    y: i32,
    z: i32,
) -> Result<Option<BloqueSeleccionado>, ErrorApp> {
    let b = biblioteca.inner().clone();
    let c = cache.inner().clone();
    en_segundo_plano(move || Ok(mundo_de(&b, &c, &id)?.bloque_en(x, y, z))).await
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn forma_json_del_error_de_importacion() {
        let r = ResultadoImportacion::Error {
            ruta: "C:/x.schem".into(),
            error: ErrorApp::NoEncontrado,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["estado"], "error");
        assert_eq!(v["ruta"], "C:/x.schem");
        assert_eq!(v["error"]["tipo"], "noEncontrado");
    }

    #[test]
    fn nombres_de_familia() {
        assert_eq!(validar_familia(" deepslate ").unwrap(), "deepslate");
        assert_eq!(
            validar_familia("otro_mod:granito").unwrap(),
            "otro_mod:granito"
        );
        for malo in ["", "Deepslate", "piedra negra", "a[b]", &"x".repeat(200)] {
            let e = validar_familia(malo).unwrap_err();
            assert_eq!(e.tipo(), "edicionInvalida", "{malo}");
        }
        assert_eq!(cuantos(None), PARECIDOS_POR_DEFECTO);
        assert_eq!(cuantos(Some(0)), 1);
        assert_eq!(cuantos(Some(5000)), MAX_PARECIDOS);
    }

    #[test]
    fn guardar_texto_solo_csv_o_txt_con_ruta_absoluta() {
        let tmp = tempfile::tempdir().unwrap();
        for nombre in ["lista.csv", "lista.TXT"] {
            let ruta = tmp.path().join(nombre);
            escribir_texto(&ruta, "Ítem;Cantidad\nPiedra;64\n").unwrap();
            assert_eq!(
                fs::read_to_string(&ruta).unwrap(),
                "Ítem;Cantidad\nPiedra;64\n"
            );
        }
        for nombre in ["lista.exe", "lista.csv.bat", "lista", "lista.json"] {
            let ruta = tmp.path().join(nombre);
            let e = escribir_texto(&ruta, "x").unwrap_err();
            assert_eq!(e.tipo(), "nombreInvalido", "{nombre}");
            assert!(!ruta.exists());
        }
        let e = escribir_texto(Path::new("relativa.csv"), "x").unwrap_err();
        assert_eq!(e.tipo(), "nombreInvalido");
        assert!(!Path::new("relativa.csv").exists());
    }
}
