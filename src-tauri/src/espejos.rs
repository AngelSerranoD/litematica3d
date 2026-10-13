// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Valores que el frontend y la configuración repiten del núcleo. No pueden
//! importarse de un lado a otro, así que estas pruebas vigilan que coincidan:
//! si se cambia uno, falla hasta cambiar su espejo.

use std::fs;
use std::path::Path;

use crate::{argumentos, biblioteca, comandos, malla::binario};

/// Contenido de un archivo relativo a la raíz del proyecto.
fn leer(ruta: &str) -> String {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    fs::read_to_string(raiz.join(ruta)).unwrap_or_else(|e| panic!("{ruta}: {e}"))
}

/// Valor numérico de `const NOMBRE = n;` (exportada o no) en un archivo TS.
fn constante_ts(ruta: &str, nombre: &str) -> usize {
    let texto = leer(ruta);
    let patron = format!("const {nombre} = ");
    let resto = texto
        .lines()
        .find_map(|l| l.split_once(&patron).map(|(_, r)| r.to_owned()))
        .unwrap_or_else(|| panic!("{ruta}: no está `{nombre}`"));
    resto
        .trim_end_matches(';')
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("{ruta}: `{nombre}` no es un número: {e}"))
}

#[test]
fn constantes_del_frontend_iguales_que_en_el_nucleo() {
    let casos = [
        (
            "src/api/atlas.ts",
            "MAX_CELDAS_ATLAS",
            comandos::texturas::MAX_CELDAS_ATLAS,
        ),
        (
            "src/api/miniaturas.ts",
            "MAX_CLAVE",
            biblioteca::MAX_CLAVE_TEXTURAS,
        ),
        (
            "src/api/visor.ts",
            "VERSION_MALLA",
            binario::VERSION as usize,
        ),
        (
            "src/api/visor.ts",
            "CABECERA_CHUNK",
            binario::CABECERA_CHUNK,
        ),
        (
            "src/componentes/NombreEditable.tsx",
            "MAX_NOMBRE",
            biblioteca::MAX_NOMBRE,
        ),
    ];
    for (ruta, nombre, nucleo) in casos {
        assert_eq!(constante_ts(ruta, nombre), nucleo, "{ruta}: {nombre}");
    }
}

#[test]
fn extensiones_iguales_en_nucleo_dialogo_y_asociaciones() {
    let mut nucleo: Vec<String> = argumentos::EXTENSIONES
        .iter()
        .map(|e| e.trim_start_matches('.').to_owned())
        .collect();
    nucleo.sort();

    let dialogo = leer("src/api/dialogo.ts");
    let lista = dialogo
        .split_once("EXTENSIONES_ADMITIDAS = [")
        .and_then(|(_, r)| r.split_once(']'))
        .expect("EXTENSIONES_ADMITIDAS en dialogo.ts")
        .0;
    let mut frontend: Vec<String> = lista
        .split(',')
        .map(|e| e.trim().trim_matches('"').to_owned())
        .filter(|e| !e.is_empty())
        .collect();
    frontend.sort();
    assert_eq!(frontend, nucleo, "dialogo.ts");

    let conf: serde_json::Value =
        serde_json::from_str(&leer("src-tauri/tauri.conf.json")).expect("tauri.conf.json");
    let mut asociadas: Vec<String> = conf["bundle"]["fileAssociations"]
        .as_array()
        .expect("fileAssociations")
        .iter()
        .flat_map(|a| a["ext"].as_array().cloned().unwrap_or_default())
        .filter_map(|e| e.as_str().map(str::to_owned))
        .collect();
    asociadas.sort();
    assert_eq!(asociadas, nucleo, "tauri.conf.json");
}

#[test]
fn version_de_cargo_igual_que_la_de_package_json() {
    // tauri.conf.json ya la toma de package.json; Cargo.toml la repite.
    let paquete: serde_json::Value =
        serde_json::from_str(&leer("package.json")).expect("package.json");
    assert_eq!(paquete["version"].as_str(), Some(env!("CARGO_PKG_VERSION")));
}
