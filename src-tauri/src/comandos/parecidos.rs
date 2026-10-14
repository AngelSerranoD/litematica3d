// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Comandos de bloques parecidos y familias (docs/edicion.md, «Parecidos»).

use std::sync::Arc;

use tauri::State;

use super::en_segundo_plano;
use crate::biblioteca::Biblioteca;
use crate::edicion::{Reemplazo, validar_estado};
use crate::error::ErrorApp;
use crate::parecidos::{
    CacheCatalogo, Catalogo, EstadoPresente, FamiliaBloque, FamiliaParecida, Parecido,
    miembros_en_schematic,
};
use crate::texturas::GestorTexturas;

/// Cuántos parecidos o familias se devuelven si no se dice.
pub const PARECIDOS_POR_DEFECTO: usize = 12;
/// Máximo de parecidos o familias en una llamada.
pub const MAX_PARECIDOS: usize = 100;
/// Máximo de estados de origen en `destinos_conservando`.
pub const MAX_ESTADOS_ORIGEN: usize = 1000;
/// Longitud máxima del nombre de una familia, en bytes.
pub const MAX_LONGITUD_FAMILIA: usize = 128;

/// Catálogo de bloques para el atlas de la versión elegida (espera a que
/// esté preparado; sin jar, el integrado). Bloquea: solo en segundo plano.
fn catalogo(g: &GestorTexturas, cache: &CacheCatalogo) -> Arc<Catalogo> {
    let activo = g.preparar().ok().flatten();
    cache.obtener(activo, g.dir_cache())
}

/// `n` acotado a `1..=MAX_PARECIDOS` (por defecto, `PARECIDOS_POR_DEFECTO`).
fn cuantos(n: Option<u32>) -> usize {
    n.map_or(PARECIDOS_POR_DEFECTO, |n| n as usize)
        .clamp(1, MAX_PARECIDOS)
}

/// Comprueba el nombre de una familia (`deepslate`, `mod:granito`).
pub fn validar_familia(familia: &str) -> Result<String, ErrorApp> {
    let f = familia.trim();
    let ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.:/".contains(c);
    if f.is_empty() || f.len() > MAX_LONGITUD_FAMILIA || !f.chars().all(ok) {
        let mostrado: String = f.chars().take(80).collect();
        return Err(ErrorApp::EdicionInvalida(format!(
            "«{mostrado}» no es un nombre de familia válido."
        )));
    }
    Ok(f.to_string())
}

/// Los `n` bloques sugeribles de la misma forma más parecidos en color a
/// `estado`, con su estado destino ya construido (docs/edicion.md, «Parecidos»).
#[tauri::command]
pub async fn parecidos(
    texturas: State<'_, Arc<GestorTexturas>>,
    catalogos: State<'_, Arc<CacheCatalogo>>,
    estado: String,
    n: Option<u32>,
) -> Result<Vec<Parecido>, ErrorApp> {
    let g = texturas.inner().clone();
    let c = catalogos.inner().clone();
    en_segundo_plano(move || {
        let estado = validar_estado(&estado)?;
        Ok(catalogo(&g, &c).parecidos(&estado, cuantos(n)))
    })
    .await
}

/// Familia, variante y forma de un block state.
#[tauri::command]
pub async fn familia_de(
    texturas: State<'_, Arc<GestorTexturas>>,
    catalogos: State<'_, Arc<CacheCatalogo>>,
    estado: String,
) -> Result<FamiliaBloque, ErrorApp> {
    let g = texturas.inner().clone();
    let c = catalogos.inner().clone();
    en_segundo_plano(move || {
        let estado = validar_estado(&estado)?;
        Ok(catalogo(&g, &c).familia_bloque(&estado))
    })
    .await
}

/// Block states de una familia presentes en un schematic de la biblioteca,
/// con cuántas posiciones ocupa cada uno.
#[tauri::command]
pub async fn miembros_familia(
    biblioteca: State<'_, Arc<Biblioteca>>,
    id: String,
    familia: String,
) -> Result<Vec<EstadoPresente>, ErrorApp> {
    let b = biblioteca.inner().clone();
    en_segundo_plano(move || {
        let familia = validar_familia(&familia)?;
        let (_, schem) = b.cargar(&id)?;
        Ok(miembros_en_schematic(&schem, &familia))
    })
    .await
}

/// Familias más parecidas a una (color medio y formas y variantes en común).
#[tauri::command]
pub async fn familias_parecidas(
    texturas: State<'_, Arc<GestorTexturas>>,
    catalogos: State<'_, Arc<CacheCatalogo>>,
    familia: String,
    n: Option<u32>,
) -> Result<Vec<FamiliaParecida>, ErrorApp> {
    let g = texturas.inner().clone();
    let c = catalogos.inner().clone();
    en_segundo_plano(move || {
        let familia = validar_familia(&familia)?;
        Ok(catalogo(&g, &c).familias_parecidas(&familia, cuantos(n)))
    })
    .await
}

/// Reemplazos para cambiar la familia `familia_origen` por `familia_destino`
/// en un schematic de la biblioteca; se aplican con `aplicar_cambios`.
#[tauri::command]
pub async fn plan_familia(
    biblioteca: State<'_, Arc<Biblioteca>>,
    texturas: State<'_, Arc<GestorTexturas>>,
    catalogos: State<'_, Arc<CacheCatalogo>>,
    id: String,
    familia_origen: String,
    familia_destino: String,
) -> Result<Vec<Reemplazo>, ErrorApp> {
    let b = biblioteca.inner().clone();
    let g = texturas.inner().clone();
    let c = catalogos.inner().clone();
    en_segundo_plano(move || {
        let origen = validar_familia(&familia_origen)?;
        let destino = validar_familia(&familia_destino)?;
        let cat = catalogo(&g, &c);
        if !cat.familias().contains_key(&destino) {
            return Err(ErrorApp::EdicionInvalida(format!(
                "No hay ninguna familia de bloques llamada «{destino}»."
            )));
        }
        let (_, schem) = b.cargar(&id)?;
        Ok(cat.plan_familia(&schem, &origen, &destino))
    })
    .await
}

/// Rechaza listas de estados de origen demasiado largas.
fn comprobar_origenes(estados: &[String]) -> Result<(), ErrorApp> {
    if estados.len() > MAX_ESTADOS_ORIGEN {
        return Err(ErrorApp::EdicionInvalida(format!(
            "Son demasiados estados de origen (máximo {MAX_ESTADOS_ORIGEN})."
        )));
    }
    Ok(())
}

/// Reemplazos de cada estado de `estados_origen` por el bloque de
/// `estado_destino` conservando las propiedades de cada uno (para las filas
/// de familia que agrupan varios estados del mismo bloque).
#[tauri::command]
pub async fn destinos_conservando(
    texturas: State<'_, Arc<GestorTexturas>>,
    catalogos: State<'_, Arc<CacheCatalogo>>,
    estado_destino: String,
    estados_origen: Vec<String>,
) -> Result<Vec<Reemplazo>, ErrorApp> {
    let g = texturas.inner().clone();
    let c = catalogos.inner().clone();
    en_segundo_plano(move || {
        comprobar_origenes(&estados_origen)?;
        let destino = validar_estado(&estado_destino)?;
        let origenes = estados_origen
            .iter()
            .map(|e| validar_estado(e))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(catalogo(&g, &c).reemplazos_conservando(&destino, &origenes))
    })
    .await
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn nombres_de_familia() {
        assert_eq!(validar_familia(" deepslate ").unwrap(), "deepslate");
        assert_eq!(
            validar_familia("otro_mod:granito").unwrap(),
            "otro_mod:granito"
        );
        for malo in ["", "Deepslate", "piedra negra", "a[b]", &"x".repeat(200)] {
            let e = validar_familia(malo).unwrap_err();
            assert_eq!(e.tipo(), "edicionInvalida", "{malo}");
        }
        assert_eq!(cuantos(None), PARECIDOS_POR_DEFECTO);
        assert_eq!(cuantos(Some(0)), 1);
        assert_eq!(cuantos(Some(5000)), MAX_PARECIDOS);
    }

    #[test]
    fn tope_de_estados_de_origen() {
        let justo = vec!["minecraft:stone".to_string(); MAX_ESTADOS_ORIGEN];
        assert!(comprobar_origenes(&justo).is_ok());
        let de_mas = vec!["minecraft:stone".to_string(); MAX_ESTADOS_ORIGEN + 1];
        assert_eq!(
            comprobar_origenes(&de_mas).unwrap_err().tipo(),
            "edicionInvalida"
        );
    }
}
