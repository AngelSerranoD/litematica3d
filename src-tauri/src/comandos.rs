// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos Tauri que usa el frontend. El trabajo pesado (leer, descomprimir,
//! calcular el hash, SQLite) va a un hilo bloqueante para no frenar la interfaz.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::biblioteca::{Biblioteca, Detalle, Entrada, Importacion};
use crate::error::ErrorApp;
use crate::materiales::{self, ListaMateriales, Opciones};

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
pub async fn eliminar(biblioteca: State<'_, Arc<Biblioteca>>, id: String) -> Result<(), ErrorApp> {
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
}
