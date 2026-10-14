// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos Tauri que usa el frontend. El trabajo pesado (leer, descomprimir,
//! calcular el hash, SQLite) va a un hilo bloqueante para no frenar la interfaz.
//! Las miniaturas: `docs/miniaturas.md`. La edición y los parecidos:
//! `docs/edicion.md`.
//!
//! Se reparten por área: biblioteca, materiales, edición, parecidos, visor y
//! texturas. Aquí quedan los ayudantes que comparten.

pub mod biblioteca;
pub mod edicion;
pub mod materiales;
pub mod parecidos;
pub mod texturas;
pub mod visor;

use std::path::PathBuf;

use tauri::WebviewWindow;
use tauri_plugin_dialog::DialogExt;

use crate::error::ErrorApp;

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
/// Pide con el diálogo nativo dónde guardar y garantiza la extensión `ext`.
/// `None` si el usuario cancela. El diálogo se abre aquí y no en el frontend
/// para que los comandos solo escriban donde el usuario ha elegido, nunca en
/// una ruta que llegue del webview. Bloquea: solo en segundo plano.
fn pedir_destino(
    ventana: &WebviewWindow,
    titulo: &str,
    propuesto: &str,
    filtro: &str,
    ext: &str,
) -> Option<PathBuf> {
    let ruta = ventana
        .dialog()
        .file()
        .set_parent(ventana)
        .set_title(titulo)
        .set_file_name(propuesto)
        .add_filter(filtro, &[ext])
        .blocking_save_file()?
        .into_path()
        .ok()?;
    if ruta
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    {
        return Some(ruta);
    }
    let mut con_ext = ruta.into_os_string();
    con_ext.push(".");
    con_ext.push(ext);
    Some(con_ext.into())
}
