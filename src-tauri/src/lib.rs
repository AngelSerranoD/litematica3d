// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

pub mod biblioteca;
pub mod comandos;
pub mod error;
pub mod malla;
pub mod materiales;
pub mod schematic;
pub mod texturas;

use std::sync::Arc;

use tauri::Manager;

use crate::biblioteca::Biblioteca;
use crate::malla::CacheVisor;
use crate::texturas::GestorTexturas;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
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
