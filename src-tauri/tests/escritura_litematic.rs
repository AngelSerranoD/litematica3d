// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Ida y vuelta: cada archivo válido de tests/datos se carga, se escribe como
//! .litematic y se vuelve a cargar; el resultado debe tener las mismas
//! dimensiones y el mismo block state en cada posición. Además se comprueba con
//! fastnbt que las etiquetas tienen los tipos que espera Litematica.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use fastnbt::Value;
use schematic_lab_lib::schematic::{
    self, Formato, OpcionesLitematic, Schematic, VERSION_LITEMATIC, escribir_litematic,
};
use serde_json::Value as Json;

fn carpeta_datos() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("datos")
}

/// Archivos de esperados.json que deben cargarse sin error.
fn archivos_validos() -> Vec<String> {
    let texto = std::fs::read_to_string(carpeta_datos().join("esperados.json")).unwrap();
    let esperados: serde_json::Map<String, Json> = serde_json::from_str(&texto).unwrap();
    let validos: Vec<String> = esperados
        .into_iter()
        .filter(|(_, e)| e.get("error").and_then(Json::as_bool) != Some(true))
        .map(|(k, _)| k)
        .collect();
    assert!(validos.len() >= 9, "faltan casos válidos en esperados.json");
    validos
}

fn opciones() -> OpcionesLitematic {
    OpcionesLitematic {
        version_datos: Some(3955),
        ahora_ms: Some(1_750_000_000_000),
    }
}

fn compuesto<'a>(v: &'a Value, que: &str) -> &'a HashMap<String, Value> {
    match v {
        Value::Compound(m) => m,
        otro => panic!("{que} debía ser Compound y es {otro:?}"),
    }
}

fn entero(v: &Value, que: &str) -> i32 {
    match v {
        Value::Int(i) => *i,
        otro => panic!("{que} debía ser Int y es {otro:?}"),
    }
}

fn largo(v: &Value, que: &str) -> i64 {
    match v {
        Value::Long(i) => *i,
        otro => panic!("{que} debía ser Long y es {otro:?}"),
    }
}

fn texto<'a>(v: &'a Value, que: &str) -> &'a str {
    match v {
        Value::String(s) => s,
        otro => panic!("{que} debía ser String y es {otro:?}"),
    }
}

fn vector(v: &Value, que: &str) -> [i32; 3] {
    let c = compuesto(v, que);
    ["x", "y", "z"].map(|k| entero(&c[k], &format!("{que}.{k}")))
}

/// Comprueba la estructura de etiquetas del .litematic escrito.
fn validar_estructura(bytes: &[u8], s: &Schematic, archivo: &str) {
    assert_eq!(&bytes[..2], &[0x1f, 0x8b], "{archivo}: no es gzip");
    let mut nbt = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .read_to_end(&mut nbt)
        .unwrap();
    let raiz: HashMap<String, Value> = fastnbt::from_bytes(&nbt).unwrap();

    assert_eq!(entero(&raiz["Version"], "Version"), VERSION_LITEMATIC);
    entero(&raiz["MinecraftDataVersion"], "MinecraftDataVersion");

    let meta = compuesto(&raiz["Metadata"], "Metadata");
    for k in ["Name", "Author", "Description"] {
        texto(&meta[k], k);
    }
    largo(&meta["TimeCreated"], "TimeCreated");
    assert_eq!(
        largo(&meta["TimeModified"], "TimeModified"),
        1_750_000_000_000
    );
    let caja = vector(&meta["EnclosingSize"], "EnclosingSize");
    let d = s.dimensiones;
    assert_eq!(
        caja,
        [d.ancho as i32, d.alto as i32, d.largo as i32],
        "{archivo}: EnclosingSize"
    );
    assert_eq!(
        entero(&meta["TotalBlocks"], "TotalBlocks") as u64,
        s.total_bloques(),
        "{archivo}: TotalBlocks"
    );
    let volumen: u64 = s.regiones.iter().map(|r| r.tamano.volumen()).sum();
    assert_eq!(entero(&meta["TotalVolume"], "TotalVolume") as u64, volumen);
    assert_eq!(
        entero(&meta["RegionCount"], "RegionCount") as usize,
        s.regiones.len()
    );

    let regiones = compuesto(&raiz["Regions"], "Regions");
    assert_eq!(
        regiones.len(),
        s.regiones.len(),
        "{archivo}: nº de regiones"
    );
    for (nombre, reg) in regiones {
        let reg = compuesto(reg, nombre);
        vector(&reg["Position"], "Position");
        let tam = vector(&reg["Size"], "Size");
        assert!(tam.iter().all(|&t| t > 0), "{archivo}: Size {tam:?}");

        let Value::List(paleta) = &reg["BlockStatePalette"] else {
            panic!("{archivo}: BlockStatePalette debía ser List");
        };
        assert!(!paleta.is_empty());
        for (i, e) in paleta.iter().enumerate() {
            let e = compuesto(e, "entrada de paleta");
            let n = texto(&e["Name"], "Name");
            if i == 0 {
                assert_eq!(n, "minecraft:air", "{archivo}: el aire no está en el 0");
            }
            if let Some(p) = e.get("Properties") {
                for (k, v) in compuesto(p, "Properties") {
                    texto(v, k);
                }
            }
        }

        let Value::LongArray(estados) = &reg["BlockStates"] else {
            panic!("{archivo}: BlockStates debía ser LongArray");
        };
        let n = paleta.len();
        let bits = (usize::BITS - (n - 1).leading_zeros()).max(2) as u64;
        let posiciones = tam.iter().map(|&t| t as u64).product::<u64>();
        assert_eq!(
            estados.len() as u64,
            (posiciones * bits).div_ceil(64),
            "{archivo}: longitud de BlockStates"
        );

        for k in [
            "TileEntities",
            "Entities",
            "PendingBlockTicks",
            "PendingFluidTicks",
        ] {
            assert!(
                matches!(reg[k], Value::List(_)),
                "{archivo}: {k} debía ser List"
            );
        }
    }
}

#[test]
fn ida_y_vuelta_de_todos_los_archivos_validos() {
    for archivo in archivos_validos() {
        let bytes = std::fs::read(carpeta_datos().join(&archivo)).unwrap();
        let original = schematic::cargar(&bytes).unwrap_or_else(|e| panic!("{archivo}: {e}"));

        let escrito = escribir_litematic(&original, &opciones())
            .unwrap_or_else(|e| panic!("{archivo}: al escribir: {e}"));
        validar_estructura(&escrito, &original, &archivo);

        let leido =
            schematic::cargar(&escrito).unwrap_or_else(|e| panic!("{archivo}: al releer: {e}"));
        assert_eq!(leido.formato, Formato::Litematic);
        assert_eq!(leido.dimensiones, original.dimensiones, "{archivo}");
        assert_eq!(leido.nombre, original.nombre, "{archivo}: nombre");
        assert_eq!(leido.autor, original.autor, "{archivo}: autor");
        assert_eq!(leido.descripcion, original.descripcion, "{archivo}");
        assert_eq!(
            leido.version_datos,
            original.version_datos.or(Some(3955)),
            "{archivo}: versión de datos"
        );
        if original.fecha_creacion.is_some() {
            assert_eq!(leido.fecha_creacion, original.fecha_creacion, "{archivo}");
        }
        assert_eq!(leido.total_bloques(), original.total_bloques(), "{archivo}");
        assert_eq!(leido.regiones.len(), original.regiones.len(), "{archivo}");

        for (ro, rl) in original.regiones.iter().zip(&leido.regiones) {
            let que = format!("{archivo} / {}", ro.nombre);
            assert_eq!(rl.nombre, ro.nombre, "{que}");
            assert_eq!(rl.origen, ro.origen, "{que}: origen");
            assert_eq!(rl.tamano, ro.tamano, "{que}: tamaño");
            let t = ro.tamano;
            for y in 0..t.alto {
                for z in 0..t.largo {
                    for x in 0..t.ancho {
                        assert_eq!(
                            rl.bloque(x, y, z),
                            ro.bloque(x, y, z),
                            "{que}: bloque ({x},{y},{z})"
                        );
                    }
                }
            }
            // Las listas que traía un .litematic se conservan tal cual.
            if original.formato == Formato::Litematic {
                let (a, b) = (&ro.extras, &rl.extras);
                let o_vacia = |v: &Option<Value>| v.clone().unwrap_or(Value::List(vec![]));
                assert_eq!(o_vacia(&b.entidades_bloque), o_vacia(&a.entidades_bloque));
                assert_eq!(o_vacia(&b.entidades), o_vacia(&a.entidades));
                assert_eq!(o_vacia(&b.ticks_bloque), o_vacia(&a.ticks_bloque));
                assert_eq!(o_vacia(&b.ticks_fluido), o_vacia(&a.ticks_fluido));
            }
        }
    }
}
