// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos de texturas: versiones de Minecraft, ajustes y atlas.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tauri::ipc::Response;

use super::en_segundo_plano;
use crate::biblioteca::Biblioteca;
use crate::edicion::validar_estado;
use crate::error::ErrorApp;
use crate::malla::{self, Cara, DatosAtlas, FuenteTexturas};
use crate::texturas::versiones::{self, VersionMinecraft};
use crate::texturas::{Ajustes, GestorTexturas};

/// Prepara el atlas de la versión elegida sin esperar el resultado (si
/// falla, `malla` sigue con colores y `atlas` devolverá el error).
pub fn preparar_en_fondo(gestor: Arc<GestorTexturas>) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = gestor.preparar();
    });
}

/// Versiones de Minecraft instaladas con jar, releases y recientes primero.
#[tauri::command]
pub async fn versiones_minecraft(
    texturas: State<'_, Arc<GestorTexturas>>,
) -> Result<Vec<VersionMinecraft>, ErrorApp> {
    let g = texturas.inner().clone();
    en_segundo_plano(move || Ok(g.versiones())).await
}

#[tauri::command]
pub fn ajustes(texturas: State<'_, Arc<GestorTexturas>>) -> Ajustes {
    texturas.ajustes()
}

/// Guarda los ajustes. Devuelve `true` si cambian las texturas (la versión
/// que se usa de verdad ya no es la misma): entonces se ha descartado el atlas
/// activo y el nuevo empieza a prepararse en segundo plano.
#[tauri::command]
pub async fn guardar_ajustes(
    texturas: State<'_, Arc<GestorTexturas>>,
    biblioteca: State<'_, Arc<Biblioteca>>,
    ajustes: Ajustes,
) -> Result<bool, ErrorApp> {
    let g = texturas.inner().clone();
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || {
        let cambian = g.guardar_ajustes(ajustes)?;
        if cambian {
            preparar_en_fondo(g);
            // Las miniaturas quedan obsoletas aquí mismo (no espera al
            // frontend): si se cierra la app antes de regenerarlas, al volver
            // a abrirla se sabe cuáles faltan. Un fallo aquí no deshace los
            // ajustes ya guardados: en esta sesión el frontend las regenera
            // igualmente.
            let _ = b.invalidar_miniaturas();
        }
        Ok(cambian)
    })
    .await
}

/// Carpeta `.minecraft` que se usa sin elegir ninguna: la primera habitual
/// que existe (`%APPDATA%\.minecraft` en Windows). `null` si no hay ninguna.
#[tauri::command]
pub fn carpeta_minecraft_por_defecto() -> Option<String> {
    versiones::carpetas_por_defecto()
        .into_iter()
        .next()
        .map(|p| p.to_string_lossy().into_owned())
}

/// PNG del atlas de la versión elegida (lo genera si hace falta, la primera
/// vez tarda unos segundos). `ArrayBuffer` vacío si no hay ningún jar.
#[tauri::command]
pub async fn atlas(texturas: State<'_, Arc<GestorTexturas>>) -> Result<Response, ErrorApp> {
    let g = texturas.inner().clone();
    en_segundo_plano(move || {
        let bytes = g
            .preparar()?
            .map_or_else(Vec::new, |a| a.png.as_ref().clone());
        Ok(Response::new(bytes))
    })
    .await
}

/// Datos del atlas activo (los mismos que `atlas` en la cabecera de la
/// malla) o `null` si aún no hay. No genera nada.
#[tauri::command]
pub fn atlas_info(texturas: State<'_, Arc<GestorTexturas>>) -> Option<DatosAtlas> {
    texturas.listo().map(|a| a.datos_atlas())
}

/// Orden de las caras en cada fila de `celdas_atlas` (el frontend lo repite
/// en `CARAS_ATLAS`; `espejos.rs` vigila que coincidan).
pub(crate) const CARAS: [Cara; 7] = [
    Cara::Arriba,
    Cara::Abajo,
    Cara::Norte,
    Cara::Sur,
    Cara::Oeste,
    Cara::Este,
    Cara::Cruz,
];

/// Como mucho tantos block states por llamada a `celdas_atlas`.
pub const MAX_CELDAS_ATLAS: usize = 256;

/// Textura de una cara de un block state en el atlas activo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CeldaAtlas {
    pub celda: u32,
    /// Giro en grados (0, 90, 180 o 270), como `Material::rotacion`.
    pub rotacion: u16,
    /// Color medio de la textura (con el tinte aplicado).
    pub color: [u8; 3],
}

/// Celdas del atlas de varios block states, cara a cara.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CeldasAtlas {
    /// El atlas con el que se resolvieron (su `version` debe casar con el
    /// que tenga el frontend).
    pub atlas: DatosAtlas,
    /// Una fila por estado pedido, en el mismo orden; cada fila trae las 7
    /// caras en el orden de `Cara` (arriba, abajo, norte, sur, oeste, este,
    /// cruz), `None` donde no hay textura. Fila vacía si el estado no es válido.
    pub celdas: Vec<Vec<Option<CeldaAtlas>>>,
}

/// Celdas del atlas activo para block states que no tienen por qué estar en
/// el schematic abierto (iconos de los bloques parecidos y previsualización
/// de los cambios pendientes). `None` si el atlas aún no está listo o no hay
/// jar: no genera nada.
#[tauri::command]
pub async fn celdas_atlas(
    texturas: State<'_, Arc<GestorTexturas>>,
    estados: Vec<String>,
) -> Result<Option<CeldasAtlas>, ErrorApp> {
    if estados.len() > MAX_CELDAS_ATLAS {
        return Err(ErrorApp::EdicionInvalida(format!(
            "Como mucho {MAX_CELDAS_ATLAS} bloques por llamada."
        )));
    }
    let Some(activo) = texturas.listo() else {
        return Ok(None);
    };
    en_segundo_plano(move || {
        let celdas = estados
            .iter()
            .map(|e| match validar_estado(e) {
                Ok(estado) => CARAS
                    .iter()
                    .map(|&cara| {
                        activo.textura(&estado, cara).map(|t| CeldaAtlas {
                            celda: t.celda,
                            rotacion: t.rotacion,
                            color: t.color,
                        })
                    })
                    .collect(),
                Err(_) => Vec::new(),
            })
            .collect();
        Ok(Some(CeldasAtlas {
            atlas: activo.datos_atlas(),
            celdas,
        }))
    })
    .await
}

/// Color plano de varios block states, el que usa la malla cuando no hay
/// atlas (`color_de` por su nombre sin propiedades): previsualización de los
/// cambios pendientes sin jar. `None` donde el estado no es válido. Como
/// mucho [`MAX_CELDAS_ATLAS`] estados por llamada.
#[tauri::command]
pub fn colores_bloques(estados: Vec<String>) -> Result<Vec<Option<[u8; 3]>>, ErrorApp> {
    if estados.len() > MAX_CELDAS_ATLAS {
        return Err(ErrorApp::EdicionInvalida(format!(
            "Como mucho {MAX_CELDAS_ATLAS} bloques por llamada."
        )));
    }
    Ok(estados
        .iter()
        .map(|e| {
            validar_estado(e)
                .ok()
                .map(|estado| malla::color_de(malla::apariencia::nombre_de(&estado)))
        })
        .collect())
}
