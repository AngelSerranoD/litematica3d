// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Recorre tests/datos/esperados.json y comprueba cada archivo de prueba.

use std::path::PathBuf;

use schematic_lab_lib::schematic::{self, Formato};
use serde_json::Value;

fn carpeta_datos() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("datos")
}

fn formato_texto(f: Formato) -> &'static str {
    f.como_texto()
}

#[test]
fn todos_los_archivos_de_prueba() {
    let texto = std::fs::read_to_string(carpeta_datos().join("esperados.json")).unwrap();
    let esperados: serde_json::Map<String, Value> = serde_json::from_str(&texto).unwrap();
    assert!(esperados.len() >= 11, "faltan casos en esperados.json");

    for (archivo, esp) in &esperados {
        let bytes = std::fs::read(carpeta_datos().join(archivo)).unwrap();
        let resultado = schematic::cargar(&bytes);

        if esp.get("error").and_then(Value::as_bool) == Some(true) {
            assert!(resultado.is_err(), "{archivo}: debía fallar");
            continue;
        }
        let s = resultado.unwrap_or_else(|e| panic!("{archivo}: {e}"));

        assert_eq!(
            formato_texto(s.formato),
            esp["formato"],
            "{archivo}: formato"
        );
        assert_eq!(
            s.dimensiones.ancho,
            esp["ancho"].as_u64().unwrap() as u32,
            "{archivo}: ancho"
        );
        assert_eq!(
            s.dimensiones.alto,
            esp["alto"].as_u64().unwrap() as u32,
            "{archivo}: alto"
        );
        assert_eq!(
            s.dimensiones.largo,
            esp["largo"].as_u64().unwrap() as u32,
            "{archivo}: largo"
        );
        assert_eq!(
            s.total_bloques(),
            esp["total_bloques"].as_u64().unwrap(),
            "{archivo}: total"
        );
        assert_eq!(
            s.regiones.len() as u64,
            esp["regiones"].as_u64().unwrap(),
            "{archivo}: regiones"
        );
        assert_eq!(
            s.nombre.as_deref(),
            esp["nombre"].as_str(),
            "{archivo}: nombre"
        );
        assert_eq!(
            s.autor.as_deref(),
            esp["autor"].as_str(),
            "{archivo}: autor"
        );
        assert_eq!(
            s.version_datos.map(i64::from),
            esp["version_datos"].as_i64(),
            "{archivo}: version_datos"
        );

        for r in &s.regiones {
            assert_eq!(
                r.bloques.len() as u64,
                r.tamano.volumen(),
                "{archivo}: volumen de {}",
                r.nombre
            );
            // Estructura (tiene huecos) y litematic (aire en el 0 por convención).
            let tiene_aire = r.paleta.iter().any(|p| p == "minecraft:air");
            if s.formato == Formato::Estructura || (tiene_aire && s.formato == Formato::Litematic) {
                assert_eq!(
                    r.paleta[0], "minecraft:air",
                    "{archivo}: aire en el índice 0"
                );
            }
        }

        let muestras = esp["muestras"].as_array().unwrap();
        assert!(!muestras.is_empty(), "{archivo}: sin muestras");
        for m in muestras {
            let region = &s.regiones[m["region"].as_u64().unwrap() as usize];
            let (x, y, z) = (
                m["x"].as_u64().unwrap() as u32,
                m["y"].as_u64().unwrap() as u32,
                m["z"].as_u64().unwrap() as u32,
            );
            assert_eq!(
                region.bloque(x, y, z),
                m["bloque"].as_str(),
                "{archivo}: región {} ({x},{y},{z})",
                region.nombre
            );
        }
    }
}

#[test]
fn orden_de_regiones_y_esquina_minima() {
    let bytes = std::fs::read(carpeta_datos().join("litematic_multi.litematic")).unwrap();
    let s = schematic::cargar(&bytes).unwrap();
    let nombres: Vec<&str> = s.regiones.iter().map(|r| r.nombre.as_str()).collect();
    assert_eq!(nombres, ["negativa", "positiva"]);
    // Position (0,0,0) con Size x = -5: la esquina mínima es x = -4.
    assert_eq!(s.regiones[0].origen, [-4, 0, 0]);
    assert_eq!(s.regiones[0].tamano.ancho, 5);
    // La segunda región tiene tamaño positivo: el origen es su Position.
    assert_eq!(s.regiones[1].origen, [20, 2, -10]);
}

#[test]
fn mcedit_traduce_ids_altos_como_desconocidos() {
    let bytes = std::fs::read(carpeta_datos().join("mcedit.schematic")).unwrap();
    let s = schematic::cargar(&bytes).unwrap();
    let paleta = &s.regiones[0].paleta;
    assert!(
        paleta.iter().any(|p| p == "schematiclab:desconocido_300_0"),
        "{paleta:?}"
    );
    assert!(
        paleta
            .iter()
            .any(|p| p == "schematiclab:desconocido_4095_5"),
        "{paleta:?}"
    );
}

#[test]
fn archivos_truncados_en_cualquier_punto_no_hacen_panic() {
    for archivo in [
        "litematic_simple.litematic",
        "sponge_v2.schem",
        "sin_comprimir.nbt",
        "mcedit.schematic",
    ] {
        let bytes = std::fs::read(carpeta_datos().join(archivo)).unwrap();
        for corte in (0..bytes.len()).step_by(7) {
            let _ = schematic::cargar(&bytes[..corte]);
        }
        // Sin comprimir: también con bytes alterados.
        if archivo.ends_with(".nbt") {
            for i in (0..bytes.len()).step_by(11) {
                let mut b = bytes.clone();
                b[i] ^= 0xa5;
                let _ = schematic::cargar(&b);
            }
        }
    }
}
