// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Archivos diminutos que declaran volúmenes enormes: deben rechazarse sin
//! reservar memoria para todas las posiciones.

use std::collections::HashMap;

use fastnbt::Value;
use litematica3d_lib::schematic::cargar;

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
