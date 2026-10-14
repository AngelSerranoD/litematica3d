// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Archivos diminutos que declaran volúmenes enormes: deben rechazarse sin
//! reservar memoria para todas las posiciones.

use std::collections::HashMap;

use fastnbt::Value;
use schematic_lab_lib::schematic::cargar;

fn compuesto(pares: Vec<(&str, Value)>) -> Value {
    Value::Compound(
        pares
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect::<HashMap<_, _>>(),
    )
}

fn bytes(v: &Value) -> Vec<u8> {
    fastnbt::to_bytes(v).expect("serializar NBT")
}

#[test]
fn estructura_enorme_sin_bloques_se_rechaza() {
    // 2000 × 300 × 800 = 480 M posiciones declaradas sin ningún bloque.
    let nbt = compuesto(vec![
        ("DataVersion", Value::Int(3953)),
        (
            "size",
            Value::List(vec![Value::Int(2000), Value::Int(300), Value::Int(800)]),
        ),
        (
            "palette",
            Value::List(vec![compuesto(vec![(
                "Name",
                Value::String("minecraft:stone".into()),
            )])]),
        ),
        ("blocks", Value::List(vec![])),
        ("entities", Value::List(vec![])),
    ]);
    let datos = bytes(&nbt);
    assert!(datos.len() < 1024);
    assert!(cargar(&datos).is_err());
}

#[test]
fn sponge_v3_enorme_sin_bloques_se_rechaza() {
    let nbt = compuesto(vec![(
        "Schematic",
        compuesto(vec![
            ("Version", Value::Int(3)),
            ("DataVersion", Value::Int(3953)),
            ("Width", Value::Short(-1)),
            ("Height", Value::Short(-1)),
            ("Length", Value::Short(100)),
        ]),
    )]);
    assert!(cargar(&bytes(&nbt)).is_err());
}

#[test]
fn litematic_con_regiones_enormes_se_rechaza_antes_de_reservar() {
    let region = |x: i32| {
        compuesto(vec![
            (
                "Position",
                compuesto(vec![
                    ("x", Value::Int(x)),
                    ("y", Value::Int(0)),
                    ("z", Value::Int(0)),
                ]),
            ),
            (
                "Size",
                compuesto(vec![
                    ("x", Value::Int(1000)),
                    ("y", Value::Int(400)),
                    ("z", Value::Int(-1000)),
                ]),
            ),
            (
                "BlockStatePalette",
                Value::List(vec![compuesto(vec![(
                    "Name",
                    Value::String("minecraft:air".into()),
                )])]),
            ),
            (
                "BlockStates",
                Value::LongArray(fastnbt::LongArray::new(vec![])),
            ),
        ])
    };
    let nbt = compuesto(vec![
        ("MinecraftDataVersion", Value::Int(3953)),
        ("Version", Value::Int(6)),
        (
            "Regions",
            compuesto(vec![("a", region(0)), ("b", region(5000))]),
        ),
    ]);
    assert!(cargar(&bytes(&nbt)).is_err());
}

/// Compounds anidados `niveles` veces, con un litematic mínimo en la raíz
/// para que llegue a deserializarse si pasa el filtro.
fn anidado(niveles: usize) -> Vec<u8> {
    let mut v = compuesto(vec![]);
    for _ in 0..niveles {
        v = compuesto(vec![("a", v)]);
    }
    bytes(&compuesto(vec![("Regions", compuesto(vec![])), ("x", v)]))
}

#[test]
fn nbt_anidado_sin_fin_se_rechaza_sin_desbordar_la_pila() {
    // Unos pocos KB con 100 000 niveles: antes desbordaba la pila y cerraba la app.
    let mut datos = Vec::new();
    datos.extend_from_slice(&[10, 0, 0]);
    for _ in 0..100_000 {
        datos.extend_from_slice(&[10, 0, 1, b'a']);
    }
    datos.extend(std::iter::repeat_n(0u8, 100_001));
    let err = cargar(&datos).expect_err("debe rechazarse");
    assert!(format!("{err:?}").contains("niveles"), "{err:?}");
}

#[test]
fn nbt_en_el_limite_de_profundidad_no_se_rechaza_por_anidar() {
    let err = cargar(&anidado(120)).err().map(|e| format!("{e:?}"));
    assert!(!err.is_some_and(|e| e.contains("niveles")));
    let err = cargar(&anidado(200)).expect_err("debe rechazarse");
    assert!(format!("{err:?}").contains("niveles"), "{err:?}");
}

#[test]
fn lista_de_tipo_end_con_elementos_se_rechaza() {
    // Lista de tipo End que declara 2^31-1 elementos: fastnbt entraba en pánico.
    let mut datos = vec![10, 0, 0, 9, 0, 1, b'l', 0];
    datos.extend_from_slice(&i32::MAX.to_be_bytes());
    datos.push(0);
    assert!(cargar(&datos).is_err());
}
