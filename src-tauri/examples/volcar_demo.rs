// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Vuelca los datos que necesita la demo web (el frontend real con un núcleo
//! falso, sin Tauri) a partir de unos schematics pequeños de `tests/datos/`
//! (los genera `generar.py`, no son de terceros).
//!
//! Importa los schematics en una biblioteca temporal y escribe, con la misma
//! forma que devolverían los comandos de Tauri sin jar de Minecraft:
//!
//! ```text
//! <salida>/listar.json            `listar`
//! <salida>/atlas_info.json        `atlas_info` (siempre `null`: sin jar)
//! <salida>/colores_bloques.json   tabla completa de `colores_bloques.json`
//! <salida>/<id>/obtener.json      `obtener { id }`
//! <salida>/<id>/materiales.json           `materiales` con `incluirFluidos: false`
//! <salida>/<id>/materiales_fluidos.json   `materiales` con `incluirFluidos: true`
//! <salida>/<id>/malla.bin         `malla { id, capas: null }` (docs/visor.md)
//! ```
//!
//! Uso (desde `src-tauri`): `cargo run --example volcar_demo [carpeta_salida]`.
//! Por defecto escribe en `../demo-datos`.

use std::fs;
use std::path::{Path, PathBuf};

use schematic_lab_lib::biblioteca::{Biblioteca, Importacion};
use schematic_lab_lib::malla::{self, Mundo};
use schematic_lab_lib::materiales::{self, Opciones};

/// Schematics de `tests/datos/` que se vuelcan (pequeños y con algo de gracia).
const ARCHIVOS: [&str; 3] = [
    "litematic_simple.litematic",
    "litematic_multi.litematic",
    "sponge_v2.schem",
];

fn escribir_json<T: serde::Serialize>(ruta: &Path, valor: &T) -> Result<(), String> {
    let texto = serde_json::to_string(valor).map_err(|e| e.to_string())?;
    fs::write(ruta, texto).map_err(|e| format!("{}: {e}", ruta.display()))
}

fn main() -> Result<(), String> {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let salida = std::env::args_os()
        .nth(1)
        .map_or_else(|| raiz.join("..").join("demo-datos"), PathBuf::from);
    fs::create_dir_all(&salida).map_err(|e| e.to_string())?;

    // Biblioteca temporal: no toca la de la app.
    let tmp = std::env::temp_dir().join(format!("l3d-volcar-demo-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let resultado = volcar(&raiz, &salida, &tmp);
    let _ = fs::remove_dir_all(&tmp);
    resultado
}

fn volcar(raiz: &Path, salida: &Path, tmp: &Path) -> Result<(), String> {
    let b = Biblioteca::abrir(tmp).map_err(|e| e.to_string())?;
    let mut ids = Vec::new();
    for nombre in ARCHIVOS {
        let ruta = raiz.join("tests").join("datos").join(nombre);
        match b.importar(&ruta).map_err(|e| e.to_string())? {
            Importacion::Importado(e) | Importacion::Duplicado(e) => ids.push(e.id),
        }
        // Que `fecha_importacion` distinga el orden de `listar`.
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    escribir_json(
        &salida.join("listar.json"),
        &b.listar().map_err(|e| e.to_string())?,
    )?;
    escribir_json(&salida.join("atlas_info.json"), &Option::<()>::None)?;
    escribir_json(
        &salida.join("colores_bloques.json"),
        &malla::apariencia::colores_integrados(),
    )?;

    for id in &ids {
        let dir = salida.join(id);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        escribir_json(
            &dir.join("obtener.json"),
            &b.obtener(id).map_err(|e| e.to_string())?,
        )?;
        let (_, schem) = b.cargar(id).map_err(|e| e.to_string())?;
        for (archivo, incluir_fluidos) in [
            ("materiales.json", false),
            ("materiales_fluidos.json", true),
        ] {
            let opciones = Opciones {
                incluir_fluidos,
                ..Default::default()
            };
            escribir_json(
                &dir.join(archivo),
                &materiales::calcular(&schem, &opciones).map_err(|e| e.to_string())?,
            )?;
        }
        let mundo = Mundo::desde_schematic(&schem)?;
        let m = malla::mallar_con_texturas(&mundo, &malla::Opciones::default(), None)?;
        fs::write(dir.join("malla.bin"), malla::binario::serializar(&m))
            .map_err(|e| e.to_string())?;
    }
    println!("Datos volcados en {}", salida.display());
    Ok(())
}
