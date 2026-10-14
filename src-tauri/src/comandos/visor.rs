// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos del visor 3D: malla del schematic, miniaturas y bloque en una
//! posición. Ver `docs/visor.md` y `docs/miniaturas.md`.

use std::sync::Arc;

use tauri::State;
use tauri::ipc::{InvokeBody, Request, Response};

use super::en_segundo_plano;
use super::texturas::preparar_en_fondo;
use crate::biblioteca::{Biblioteca, Entrada};
use crate::error::ErrorApp;
use crate::malla::{self, BloqueSeleccionado, CacheVisor, FuenteTexturas, Mundo};
use crate::texturas::GestorTexturas;

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
    if png.len() > crate::biblioteca::MAX_BYTES_MINIATURA {
        return Err(ErrorApp::MiniaturaInvalida(
            "la imagen es demasiado grande".into(),
        ));
    }
    let png = png.clone();
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || b.guardar_miniatura(&id, &png, &texturas)).await
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
