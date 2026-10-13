// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos Tauri que usa el frontend. El trabajo pesado (leer, descomprimir,
//! calcular el hash, SQLite) va a un hilo bloqueante para no frenar la interfaz.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};

use crate::biblioteca::{Biblioteca, Detalle, Entrada, Importacion};
use crate::error::ErrorApp;
use crate::malla::{self, BloqueSeleccionado, CacheVisor, DatosAtlas, FuenteTexturas, Mundo};
use crate::materiales::{self, ListaMateriales, Opciones};
use crate::texturas::versiones::VersionMinecraft;
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

/// Id de ítem (`minecraft:stone`) → nombre en español (`Piedra`), del
/// `es_es.json` de la versión de Minecraft elegida. Vacío si no lo hay;
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

/// Schematic combinado del id: de la caché o leído y parseado (y cacheado).
fn mundo_de(b: &Biblioteca, cache: &CacheVisor, id: &str) -> Result<Arc<Mundo>, ErrorApp> {
    if let Some(m) = cache.obtener(id) {
        return Ok(m);
    }
    let (entrada, schem) = b.cargar(id)?;
    let mundo = Mundo::desde_schematic(&schem).map_err(|detalle| ErrorApp::DemasiadoGrande {
        archivo: entrada.nombre,
        detalle,
    })?;
    drop(schem);
    let mundo = Arc::new(mundo);
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

/// Guarda los ajustes; si cambia la versión o la carpeta, descarta el atlas
/// activo y empieza a preparar el nuevo en segundo plano.
#[tauri::command]
pub async fn guardar_ajustes(
    texturas: State<'_, Arc<GestorTexturas>>,
    ajustes: Ajustes,
) -> Result<(), ErrorApp> {
    let g = texturas.inner().clone();
    en_segundo_plano(move || {
        g.guardar_ajustes(ajustes)?;
        preparar_en_fondo(g);
        Ok(())
    })
    .await
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
