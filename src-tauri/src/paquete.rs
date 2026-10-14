// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Versión empaquetada (MSIX, Microsoft Store): carpetas de datos propias.
//!
//! Instalada desde la Store, la app corre con identidad de paquete. Guarda la
//! biblioteca, los ajustes y la caché en las carpetas del paquete
//! (`%LOCALAPPDATA%\Packages\<familia>\LocalState` y `LocalCache`), no en
//! `%APPDATA%\com.aserrano.schematiclab` como la versión NSIS. Así las dos
//! versiones no comparten (ni mezclan) biblioteca y Windows lo borra todo al
//! desinstalar. Esas carpetas no se virtualizan: la ruta es la real.
//! Ver docs/distribucion.md.

use std::path::{Path, PathBuf};

/// Carpetas de datos de la versión empaquetada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carpetas {
    /// Biblioteca (SQLite, copias, miniaturas, historial). Hace de `app_data_dir`.
    pub datos: PathBuf,
    /// `ajustes.json`. Hace de `app_config_dir`.
    pub config: PathBuf,
    /// Atlas de texturas. Hace de `app_cache_dir`.
    pub cache: PathBuf,
}

impl Carpetas {
    /// Las del paquete de la familia `familia`, dentro de `local` (`%LOCALAPPDATA%`).
    pub fn del_paquete(local: &Path, familia: &str) -> Self {
        let raiz = local.join("Packages").join(familia);
        let estado = raiz.join("LocalState");
        Self {
            datos: estado.clone(),
            config: estado,
            cache: raiz.join("LocalCache"),
        }
    }

    /// Las del paquete si el proceso corre con identidad de paquete; `None`
    /// fuera de él (instalador NSIS, `cargo run`, pruebas).
    pub fn actuales() -> Option<Self> {
        let familia = familia_paquete()?;
        let local = std::env::var_os("LOCALAPPDATA")?;
        Some(Self::del_paquete(Path::new(&local), &familia))
    }
}

/// Nombre de familia del paquete del proceso (`Nombre_hashdeleditor`), o
/// `None` si no corre empaquetado.
#[cfg(windows)]
pub fn familia_paquete() -> Option<String> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        // appmodel.h (Windows 8+). 0 = ERROR_SUCCESS; sin paquete devuelve
        // APPMODEL_ERROR_NO_PACKAGE (15700).
        fn GetCurrentPackageFamilyName(longitud: *mut u32, nombre: *mut u16) -> i32;
    }
    // PACKAGE_FAMILY_NAME_MAX_LENGTH es 64 + 1 + 13; sobra con 256.
    let mut buf = [0u16; 256];
    let mut longitud = buf.len() as u32;
    // SAFETY: `longitud` dice la capacidad real de `buf` y la función no
    // escribe más allá; los dos punteros son válidos durante la llamada.
    let r = unsafe { GetCurrentPackageFamilyName(&mut longitud, buf.as_mut_ptr()) };
    if r != 0 {
        return None;
    }
    let fin = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let familia = String::from_utf16(&buf[..fin]).ok()?;
    (!familia.is_empty()).then_some(familia)
}

#[cfg(not(windows))]
pub fn familia_paquete() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carpetas_dentro_del_paquete() {
        let c = Carpetas::del_paquete(
            Path::new(r"C:\Users\u\AppData\Local"),
            "Editor.SchematicLab_abc123",
        );
        let raiz = Path::new(r"C:\Users\u\AppData\Local\Packages\Editor.SchematicLab_abc123");
        assert_eq!(c.datos, raiz.join("LocalState"));
        assert_eq!(c.config, raiz.join("LocalState"));
        assert_eq!(c.cache, raiz.join("LocalCache"));
    }

    #[test]
    fn fuera_del_paquete_no_hay_familia() {
        // Las pruebas no corren empaquetadas: la app usa las carpetas de siempre.
        assert_eq!(familia_paquete(), None);
        assert_eq!(Carpetas::actuales(), None);
    }
}
