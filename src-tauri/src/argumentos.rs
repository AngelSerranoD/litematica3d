// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Archivo que Windows pasa por la línea de comandos al abrir un schematic con
//! doble clic (asociación de archivos) o al lanzar la app con una ruta.

use std::sync::Mutex;

/// Extensiones que la app abre. Espejo de `EXTENSIONES_ADMITIDAS` en
/// `src/api/dialogo.ts` y de `fileAssociations` en `tauri.conf.json`: si se
/// cambia una, hay que cambiar las tres (lo comprueba `espejos.rs`).
/// `.mcstructure` no está soportado.
pub(crate) const EXTENSIONES: [&str; 4] = [".litematic", ".schem", ".schematic", ".nbt"];

/// Primera ruta válida de `args` (el primer elemento es el ejecutable y se
/// salta). Ignora las opciones (`-x`, `--y`) y las extensiones no admitidas.
pub fn ruta_de_argv<S: AsRef<str>>(args: &[S]) -> Option<String> {
    args.iter()
        .skip(1)
        .map(|a| a.as_ref().trim().trim_matches('"'))
        .filter(|a| !a.is_empty() && !a.starts_with('-'))
        .find(|a| {
            let minusculas = a.to_lowercase();
            EXTENSIONES.iter().any(|ext| minusculas.ends_with(ext))
        })
        .map(str::to_owned)
}

/// Archivo recibido en el arranque que el frontend aún no ha recogido.
#[derive(Default)]
pub struct ArchivoPendiente(pub Mutex<Option<String>>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toma_la_primera_ruta_valida() {
        let args = ["app.exe", r"C:\a\torre.litematic", r"C:\b\otra.schem"];
        assert_eq!(
            ruta_de_argv(&args).as_deref(),
            Some(r"C:\a\torre.litematic")
        );
    }

    #[test]
    fn salta_el_ejecutable_aunque_tenga_extension_valida() {
        assert_eq!(ruta_de_argv(&["x.schem"]), None);
    }

    #[test]
    fn ignora_flags_y_extensiones_no_validas() {
        let args = [
            "app.exe",
            "--debug",
            "-v",
            r"C:\notas.txt",
            r"C:\m.SCHEMATIC",
        ];
        assert_eq!(ruta_de_argv(&args).as_deref(), Some(r"C:\m.SCHEMATIC"));
        assert_eq!(ruta_de_argv(&["app.exe", "--flag.litematic"]), None);
        assert_eq!(ruta_de_argv(&["app.exe"]), None);
        assert_eq!(ruta_de_argv::<&str>(&[]), None);
    }

    #[test]
    fn admite_espacios_y_tildes() {
        let ruta = r"C:\Users\Usuario Ñandú\Construcciones\Catedral gótica.litematic";
        assert_eq!(ruta_de_argv(&["app.exe", ruta]).as_deref(), Some(ruta));
        let entre_comillas = format!("\"{ruta}\"");
        assert_eq!(
            ruta_de_argv(&["app.exe", &entre_comillas]).as_deref(),
            Some(ruta)
        );
    }
}
