// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Utilidades de archivos de la biblioteca: lectura acotada al importar,
//! validación de ids y sustitución de archivos.

use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use super::MAX_BYTES_IMPORTAR;
use crate::error::ErrorApp;

/// Lee el archivo que se va a importar sin pasar de [`MAX_BYTES_IMPORTAR`]:
/// mira el tamaño antes de leer y, por si crece mientras tanto, no lee más
/// allá del tope.
pub(super) fn leer_acotado(ruta: &Path, nombre_archivo: &str) -> Result<Vec<u8>, ErrorApp> {
    use std::io::Read;

    let accion = || format!("leer «{nombre_archivo}»");
    let demasiado_grande = || ErrorApp::DemasiadoGrande {
        archivo: nombre_archivo.to_string(),
        detalle: format!(
            "pasa de {} MiB, el máximo que se puede importar",
            MAX_BYTES_IMPORTAR / (1024 * 1024)
        ),
    };
    let archivo = fs::File::open(ruta).map_err(|e| ErrorApp::io(accion(), e))?;
    let tamano = archivo
        .metadata()
        .map_err(|e| ErrorApp::io(accion(), e))?
        .len();
    if tamano > MAX_BYTES_IMPORTAR {
        return Err(demasiado_grande());
    }
    let mut bytes = Vec::with_capacity(tamano as usize);
    archivo
        .take(MAX_BYTES_IMPORTAR + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| ErrorApp::io(accion(), e))?;
    if bytes.len() as u64 > MAX_BYTES_IMPORTAR {
        return Err(demasiado_grande());
    }
    Ok(bytes)
}

/// Ids que genera la biblioteca (UUID): solo letras, cifras y guiones.
pub(super) fn id_valido(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// `rename` que sustituye el destino. En Windows puede fallar un momento si
/// el webview está leyendo la miniatura anterior: se reintenta unas veces.
pub(super) fn reemplazar(origen: &Path, destino: &Path) -> io::Result<()> {
    let mut intento = 0u64;
    loop {
        match fs::rename(origen, destino) {
            Ok(()) => return Ok(()),
            Err(e) if intento >= 4 => return Err(e),
            Err(_) => {
                intento += 1;
                std::thread::sleep(Duration::from_millis(40 * intento));
            }
        }
    }
}
