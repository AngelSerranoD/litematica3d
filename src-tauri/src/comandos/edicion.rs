// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos de edición: aplicar cambios, historial de versiones y exportar.
//! Ver `docs/edicion.md`.

use std::sync::Arc;

use tauri::{State, WebviewWindow};

use super::{en_segundo_plano, pedir_destino};
use crate::biblioteca::{Biblioteca, Edicion, Version};
use crate::edicion::{Cambio, Reemplazo};
use crate::error::ErrorApp;
use crate::malla::CacheVisor;

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

/// Pide dónde exportar y escribe la copia actual como .litematic. Devuelve la
/// ruta elegida, o `None` si el usuario cancela.
#[tauri::command]
pub async fn exportar_schematic(
    ventana: WebviewWindow,
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
    nombre: String,
) -> Result<Option<String>, ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || {
        let Some(ruta) = pedir_destino(
            &ventana,
            "Exportar schematic",
            &nombre,
            "Litematica (.litematic)",
            "litematic",
        ) else {
            return Ok(None);
        };
        b.exportar(&id, &ruta)?;
        Ok(Some(ruta.to_string_lossy().into_owned()))
    })
    .await
}
