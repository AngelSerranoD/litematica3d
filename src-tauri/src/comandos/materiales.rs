// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos de materiales: lista, ítems reunidos, nombres y exportación a CSV.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State, WebviewWindow};

use super::{en_segundo_plano, pedir_destino};
use crate::biblioteca::Biblioteca;
use crate::error::ErrorApp;
use crate::materiales::{self, ListaMateriales, Opciones};
use crate::texturas::GestorTexturas;

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

/// Id de ítem (`minecraft:stone`) → nombre en inglés (`Stone`), del
/// `en_us.json` del jar de la versión de Minecraft elegida. Vacío si no lo hay;
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

/// Extensiones que acepta `escribir_texto`.
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

/// Pide dónde guardar la lista de materiales y escribe el CSV. `false` si el
/// usuario cancela.
#[tauri::command]
pub async fn guardar_csv(
    ventana: WebviewWindow,
    nombre: String,
    contenido: String,
) -> Result<bool, ErrorApp> {
    en_segundo_plano(move || {
        let Some(ruta) = pedir_destino(
            &ventana,
            "Guardar lista de materiales",
            &nombre,
            "CSV (Excel, separado por punto y coma)",
            "csv",
        ) else {
            return Ok(false);
        };
        escribir_texto(&ruta, &contenido).map(|()| true)
    })
    .await
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn escribir_texto_solo_csv_o_txt_con_ruta_absoluta() {
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
