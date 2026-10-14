// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

pub mod argumentos;
pub mod biblioteca;
pub mod comandos;
pub mod edicion;
pub mod error;
pub mod malla;
pub mod materiales;
pub mod parecidos;
pub mod schematic;
pub mod texturas;

#[cfg(test)]
mod espejos;

use std::sync::Arc;

/// Evento que avisa al frontend de que otro proceso pidió abrir un archivo
/// (`EVENTO_ABRIR_ARCHIVO` en `src/api/biblioteca.ts`).
pub(crate) const EVENTO_ABRIR_ARCHIVO: &str = "abrir-archivo";

use tauri::{Emitter, Manager};

use crate::biblioteca::Biblioteca;
use crate::malla::CacheVisor;
use crate::parecidos::CacheCatalogo;
use crate::texturas::GestorTexturas;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Debe ir el primero. Si la app ya está abierta, el segundo proceso
        // termina y su argv llega aquí: se trae la ventana al frente y se avisa
        // al frontend para que importe y abra el archivo.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(ventana) = app.get_webview_window("main") {
                let _ = ventana.unminimize();
                let _ = ventana.show();
                let _ = ventana.set_focus();
            }
            if let Some(ruta) = argumentos::ruta_de_argv(&argv) {
                let _ = app.emit(EVENTO_ABRIR_ARCHIVO, ruta);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Archivo con el que se lanzó la app: el frontend lo recoge con
            // `archivo_pendiente` cuando ya escucha (evita la carrera con el evento).
            let argv: Vec<String> = std::env::args().collect();
            app.manage(argumentos::ArchivoPendiente(std::sync::Mutex::new(
                argumentos::ruta_de_argv(&argv),
            )));
            let dir_datos = app.path().app_data_dir()?;
            let biblioteca = Biblioteca::abrir(&dir_datos)?;
            app.manage(Arc::new(biblioteca));
            app.manage(Arc::new(CacheVisor::default()));
            app.manage(Arc::new(CacheCatalogo::default()));
            let texturas = Arc::new(GestorTexturas::nuevo(
                &app.path().app_cache_dir()?,
                &app.path().app_config_dir()?,
            ));
            // El atlas se prepara en segundo plano desde el arranque; mientras
            // tanto, la malla sale con colores.
            comandos::texturas::preparar_en_fondo(Arc::clone(&texturas));
            app.manage(texturas);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::biblioteca::archivo_pendiente,
            comandos::biblioteca::importar,
            comandos::biblioteca::listar,
            comandos::biblioteca::obtener,
            comandos::biblioteca::renombrar,
            comandos::biblioteca::eliminar,
            comandos::materiales::materiales,
            comandos::materiales::comprobacion,
            comandos::materiales::marcar,
            comandos::materiales::nombres_items,
            comandos::materiales::guardar_csv,
            comandos::edicion::aplicar_cambios,
            comandos::edicion::historial,
            comandos::edicion::restaurar_version,
            comandos::edicion::exportar_schematic,
            comandos::parecidos::parecidos,
            comandos::parecidos::familia_de,
            comandos::parecidos::miembros_familia,
            comandos::parecidos::familias_parecidas,
            comandos::parecidos::plan_familia,
            comandos::parecidos::destinos_conservando,
            comandos::visor::malla,
            comandos::visor::malla_miniatura,
            comandos::visor::guardar_miniatura,
            comandos::visor::bloque_en,
            comandos::texturas::versiones_minecraft,
            comandos::texturas::ajustes,
            comandos::texturas::guardar_ajustes,
            comandos::texturas::carpeta_minecraft_por_defecto,
            comandos::texturas::atlas,
            comandos::texturas::atlas_info,
            comandos::texturas::celdas_atlas,
            comandos::texturas::colores_bloques,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicación Tauri");
}
