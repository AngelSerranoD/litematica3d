// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

pub mod argumentos;
pub mod biblioteca;
pub mod comandos;
pub mod error;
pub mod malla;
pub mod materiales;
pub mod schematic;
pub mod texturas;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use crate::biblioteca::Biblioteca;
use crate::malla::CacheVisor;
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
                let _ = app.emit("abrir-archivo", ruta);
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
            let texturas = Arc::new(GestorTexturas::nuevo(
                &app.path().app_cache_dir()?,
                &app.path().app_config_dir()?,
            ));
            // El atlas se prepara en segundo plano desde el arranque; mientras
            // tanto, la malla sale con colores.
            comandos::preparar_en_fondo(Arc::clone(&texturas));
            app.manage(texturas);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::archivo_pendiente,
            comandos::importar,
            comandos::listar,
            comandos::obtener,
            comandos::renombrar,
            comandos::eliminar,
            comandos::materiales,
            comandos::comprobacion,
            comandos::marcar,
            comandos::nombres_items,
            comandos::guardar_texto,
            comandos::malla,
            comandos::malla_miniatura,
            comandos::guardar_miniatura,
            comandos::bloque_en,
            comandos::versiones_minecraft,
            comandos::ajustes,
            comandos::guardar_ajustes,
            comandos::carpeta_minecraft_por_defecto,
            comandos::atlas,
            comandos::atlas_info,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicación Tauri");
}
