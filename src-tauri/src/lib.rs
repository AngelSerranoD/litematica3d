// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

pub mod biblioteca;
pub mod comandos;
pub mod error;
pub mod schematic;

use std::sync::Arc;

use tauri::Manager;

use crate::biblioteca::Biblioteca;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir_datos = app.path().app_data_dir()?;
            let biblioteca = Biblioteca::abrir(&dir_datos)?;
            app.manage(Arc::new(biblioteca));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::importar,
            comandos::listar,
            comandos::obtener,
            comandos::renombrar,
            comandos::eliminar,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicación Tauri");
}
