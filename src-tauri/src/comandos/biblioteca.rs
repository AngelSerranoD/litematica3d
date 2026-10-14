// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos de la biblioteca: importar, listar, ver, renombrar y eliminar.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use super::en_segundo_plano;
use crate::biblioteca::{Biblioteca, Detalle, Entrada, Importacion};
use crate::error::ErrorApp;
use crate::malla::CacheVisor;

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

/// Archivo con el que se arrancó la app (doble clic en un schematic), si lo
/// hay. Se entrega una sola vez.
#[tauri::command]
pub fn archivo_pendiente(
    estado: tauri::State<'_, crate::argumentos::ArchivoPendiente>,
) -> Option<String> {
    estado.0.lock().ok().and_then(|mut g| g.take())
}

/// Máximo de rutas en una llamada a `importar`.
pub const MAX_RUTAS_IMPORTAR: usize = 1000;

/// Error de entrada si se pasan demasiadas rutas de golpe.
fn comprobar_rutas(rutas: &[String]) -> Result<(), ErrorApp> {
    if rutas.len() > MAX_RUTAS_IMPORTAR {
        return Err(ErrorApp::EdicionInvalida(format!(
            "Son demasiados archivos de golpe (máximo {MAX_RUTAS_IMPORTAR})."
        )));
    }
    Ok(())
}

/// Importa varios archivos; devuelve un resultado por ruta, en el mismo orden.
#[tauri::command]
pub async fn importar(app: AppHandle, rutas: Vec<String>) -> Vec<ResultadoImportacion> {
    if let Err(e) = comprobar_rutas(&rutas) {
        let texto = e.to_string();
        return rutas
            .into_iter()
            .map(|ruta| ResultadoImportacion::Error {
                ruta,
                error: ErrorApp::EdicionInvalida(texto.clone()),
            })
            .collect();
    }
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

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn tope_de_rutas_a_importar() {
        let justo = vec![String::new(); MAX_RUTAS_IMPORTAR];
        assert!(comprobar_rutas(&justo).is_ok());
        let de_mas = vec![String::new(); MAX_RUTAS_IMPORTAR + 1];
        assert_eq!(
            comprobar_rutas(&de_mas).unwrap_err().tipo(),
            "edicionInvalida"
        );
    }

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
