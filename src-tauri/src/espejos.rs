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

    // Paquete MSIX de la Store: <uap:FileType>.ext</uap:FileType>.
    let manifiesto = leer("src-tauri/msix/AppxManifest.xml");
    let mut msix: Vec<String> = manifiesto
        .split("<uap:FileType>")
        .skip(1)
        .filter_map(|r| r.split_once("</uap:FileType>"))
        .map(|(e, _)| e.trim().trim_start_matches('.').to_owned())
        .collect();
    msix.sort();
    assert_eq!(msix, nucleo, "src-tauri/msix/AppxManifest.xml");
}

/// El alcance del protocolo `asset` (miniaturas) cubre la carpeta de copias
/// de la biblioteca en las dos versiones: NSIS (`app_data_dir`) y la de la
/// Store (`LocalState` del paquete, `paquete.rs`).
#[test]
fn alcance_de_miniaturas_en_la_carpeta_de_copias() {
    let conf: serde_json::Value =
        serde_json::from_str(&leer("src-tauri/tauri.conf.json")).expect("tauri.conf.json");
    let alcance = conf["app"]["security"]["assetProtocol"]["scope"]
        .as_array()
        .expect("assetProtocol.scope");
    let copias = biblioteca::CARPETA_COPIAS;
    for esperado in [
        format!("$APPDATA/{copias}/*.miniatura.png"),
        format!("$LOCALDATA/Packages/*/LocalState/{copias}/*.miniatura.png"),
    ] {
        assert!(
            alcance.iter().any(|a| a.as_str() == Some(esperado.as_str())),
            "tauri.conf.json: falta {esperado}"
        );
    }
    // La ruta del paquete en el alcance es la que calcula `paquete.rs`.
    let c = crate::paquete::Carpetas::del_paquete(Path::new("L"), "F");
    assert_eq!(c.datos, Path::new("L/Packages/F/LocalState"));
}

#[test]
fn version_de_cargo_igual_que_la_de_package_json() {
    // tauri.conf.json ya la toma de package.json; Cargo.toml la repite.
    let paquete: serde_json::Value =
        serde_json::from_str(&leer("package.json")).expect("package.json");
    assert_eq!(paquete["version"].as_str(), Some(env!("CARGO_PKG_VERSION")));
}

/// Valor de `const NOMBRE = "texto";` (exportada o no) en un archivo TS.
fn cadena_ts(ruta: &str, nombre: &str) -> String {
    let texto = leer(ruta);
    let patron = format!("const {nombre} = \"");
    let resto = texto
        .lines()
        .find_map(|l| l.split_once(&patron).map(|(_, r)| r.to_owned()))
        .unwrap_or_else(|| panic!("{ruta}: no está `{nombre}`"));
    resto
        .split_once('"')
        .unwrap_or_else(|| panic!("{ruta}: `{nombre}` sin comilla de cierre"))
        .0
        .to_owned()
}

/// Literales de cadena (entre comillas dobles) de un texto, en orden.
fn literales(texto: &str) -> Vec<String> {
    texto
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[test]
fn cabeceras_de_miniatura_iguales_en_nucleo_y_frontend() {
    let ruta = "src/api/miniaturas.ts";
    assert_eq!(cadena_ts(ruta, "CABECERA_ID"), comandos::visor::CABECERA_ID);
    assert_eq!(
        cadena_ts(ruta, "CABECERA_TEXTURAS"),
        comandos::visor::CABECERA_TEXTURAS
    );
}

#[test]
fn maximo_de_versiones_igual_en_historial_y_frontend() {
    assert_eq!(
        constante_ts("src/api/edicion.ts", "MAX_VERSIONES"),
        biblioteca::MAX_VERSIONES
    );
}

#[test]
fn evento_de_abrir_archivo_igual_en_nucleo_y_frontend() {
    assert_eq!(
        cadena_ts("src/api/biblioteca.ts", "EVENTO_ABRIR_ARCHIVO"),
        crate::EVENTO_ABRIR_ARCHIVO
    );
}

#[test]
fn tipos_de_error_iguales_en_nucleo_y_frontend() {
    // Las cadenas de `ErrorApp::tipo()`, leídas del propio código fuente.
    let fuente = leer("src-tauri/src/error.rs");
    let cuerpo = fuente
        .split_once("pub fn tipo(&self)")
        .and_then(|(_, r)| r.split_once("\n    }\n"))
        .expect("`tipo()` en error.rs")
        .0;
    let mut nucleo: Vec<String> = cuerpo
        .lines()
        .filter(|l| l.contains("=>"))
        .flat_map(literales)
        .collect();
    nucleo.sort();

    let ts = leer("src/api/biblioteca.ts");
    let union = ts
        .split_once("type TipoError =")
        .and_then(|(_, r)| r.split_once(';'))
        .expect("unión TipoError en biblioteca.ts")
        .0;
    let mut frontend = literales(union);
    frontend.sort();

    assert!(!nucleo.is_empty());
    assert_eq!(nucleo, frontend);
}

#[test]
fn formatos_iguales_en_nucleo_y_frontend() {
    use crate::schematic::Formato;
    // El `match` sin comodín obliga a revisar esta lista al añadir un formato.
    let todos = [
        Formato::Litematic,
        Formato::Sponge,
        Formato::Mcedit,
        Formato::Estructura,
    ];
    for f in todos {
        match f {
            Formato::Litematic | Formato::Sponge | Formato::Mcedit | Formato::Estructura => {}
        }
    }
    let mut nucleo: Vec<String> = todos.iter().map(|f| f.como_texto().to_owned()).collect();
    nucleo.sort();

    let ts = leer("src/api/biblioteca.ts");
    let union = ts
        .split_once("type Formato =")
        .and_then(|(_, r)| r.split_once(';'))
        .expect("unión Formato en biblioteca.ts")
        .0;
    let mut frontend = literales(union);
    frontend.sort();
    assert_eq!(nucleo, frontend);
}

#[test]
fn orden_de_caras_igual_en_nucleo_y_frontend() {
    use crate::malla::apariencia::Cara;
    // Nombres serde de Rust (camelCase) en el orden de `celdas_atlas`.
    let nucleo: Vec<String> = comandos::texturas::CARAS
        .iter()
        .map(|c| {
            serde_json::to_value(c)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();

    let atlas = leer("src/api/atlas.ts");
    let lista = atlas
        .split_once("CARAS_ATLAS = [")
        .and_then(|(_, r)| r.split_once(']'))
        .expect("CARAS_ATLAS en atlas.ts")
        .0;
    assert_eq!(nucleo, literales(lista), "CARAS_ATLAS");

    // `CaraMaterial` (visor.ts) es la misma lista.
    let visor = leer("src/api/visor.ts");
    let union = visor
        .split_once("type CaraMaterial =")
        .and_then(|(_, r)| r.split_once(';'))
        .expect("CaraMaterial en visor.ts")
        .0;
    assert_eq!(nucleo, literales(union), "CaraMaterial");

    // `NORMALES` (visor.ts): el índice de cada cara de `Cara` es el de su normal.
    let bloque = visor
        .split_once("NORMALES: readonly Vec3[] = [")
        .and_then(|(_, r)| r.split_once("];"))
        .expect("NORMALES en visor.ts")
        .0;
    for (i, cara) in Cara::CUBO.iter().enumerate() {
        let linea = bloque
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with('['))
            .nth(i)
            .unwrap_or_else(|| panic!("NORMALES: falta el índice {i}"));
        let vector: Vec<i32> = linea
            .trim_start_matches('[')
            .split(']')
            .next()
            .unwrap()
            .split(',')
            .map(|n| n.trim().parse().expect("componente entera"))
            .collect();
        assert_eq!(vector, cara.delta().to_vec(), "NORMALES[{i}] ({cara:?})");
        assert_eq!(*cara as usize, i, "índice de {cara:?}");
    }
}
