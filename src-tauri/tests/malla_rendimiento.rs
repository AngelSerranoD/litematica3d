// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Medida de rendimiento del mallado con 1 M de bloques sintéticos (100³).
//! Se ejecuta a mano, en release:
//!
//! ```text
//! cargo test --release --test malla_rendimiento -- --ignored --nocapture
//! ```
//!
//! Con la variable `LITEMATICA3D_VOLCAR_MALLA=<carpeta>` deja además el
//! binario de cada caso en `<carpeta>/<caso>.l3dm`, para probar el
//! decodificador de `src/api/visor.ts` fuera de la aplicación.

use std::time::Instant;

use litematica3d_lib::malla::{Mundo, Opciones, binario, mallar};
use litematica3d_lib::schematic::modelo::{Dimensiones, Formato, Region, Schematic};

const LADO: u32 = 100;

/// Pseudoaleatorio estable (xorshift) para no depender de crates.
fn azar(semilla: &mut u64) -> u32 {
    *semilla ^= *semilla << 13;
    *semilla ^= *semilla >> 7;
    *semilla ^= *semilla << 17;
    (*semilla >> 32) as u32
}

/// `elegir(x, y, z, azar) -> índice de paleta`.
fn schematic(paleta: &[&str], elegir: impl Fn(u32, u32, u32, u32) -> u32) -> Schematic {
    let mut semilla = 0x9e37_79b9_7f4a_7c15u64;
    let mut bloques = Vec::with_capacity((LADO * LADO * LADO) as usize);
    for y in 0..LADO {
        for z in 0..LADO {
            for x in 0..LADO {
                bloques.push(elegir(x, y, z, azar(&mut semilla) % 100));
            }
        }
    }
    let tamano = Dimensiones {
        ancho: LADO,
        alto: LADO,
        largo: LADO,
    };
    Schematic {
        formato: Formato::Litematic,
        nombre: None,
        autor: None,
        descripcion: None,
        version_datos: None,
        fecha_creacion: None,
        dimensiones: tamano,
        regiones: vec![Region {
            nombre: "prueba".into(),
            origen: [0, 0, 0],
            tamano,
            paleta: paleta.iter().map(|s| s.to_string()).collect(),
            bloques,
            extras: Default::default(),
        }],
    }
}

fn medir(nombre: &str, s: &Schematic) {
    let t0 = Instant::now();
    let mundo = Mundo::desde_schematic(s).unwrap();
    let t1 = Instant::now();
    let m = mallar(&mundo, &Opciones::default()).unwrap();
    let t2 = Instant::now();
    let bytes = binario::serializar(&m);
    let t3 = Instant::now();
    if let Some(dir) = std::env::var_os("LITEMATICA3D_VOLCAR_MALLA") {
        let ruta = std::path::Path::new(&dir).join(format!("{}.l3dm", nombre.replace(' ', "_")));
        std::fs::write(&ruta, &bytes).unwrap();
    }
    let vertices: usize = m.chunks.iter().map(|c| c.vertices()).sum();
    let indices: usize = m.chunks.iter().map(|c| c.indices.len()).sum();
    println!(
        "[{nombre}] bloques={} chunks={} materiales={} vertices={} triangulos={} \
         binario={:.1} MiB | combinar={:?} mallar={:?} serializar={:?}",
        mundo.total_bloques(),
        m.chunks.len(),
        m.materiales.len(),
        vertices,
        indices / 3,
        bytes.len() as f64 / (1024.0 * 1024.0),
        t1 - t0,
        t2 - t1,
        t3 - t2,
    );
}

#[test]
#[ignore = "medida de rendimiento: ejecutar a mano en release"]
fn un_millon_de_bloques() {
    let paleta = [
        "minecraft:air",
        "minecraft:stone",
        "minecraft:dirt",
        "minecraft:oak_planks",
        "minecraft:glass",
        "minecraft:oak_stairs[facing=north,half=bottom,shape=straight,waterlogged=false]",
        "minecraft:stone_slab[type=bottom,waterlogged=false]",
        "minecraft:oak_leaves[distance=1,persistent=true,waterlogged=false]",
        "minecraft:oak_fence[east=true,north=true,south=false,waterlogged=false,west=false]",
        "minecraft:poppy",
        "minecraft:cobblestone",
    ];

    // Peor caso: mezcla aleatoria bloque a bloque (casi nada se fusiona).
    let aleatorio = schematic(&paleta, |_, _, _, r| match r {
        0..10 => 0,
        10..50 => 1,
        50..65 => 2,
        65..75 => 3,
        75..80 => 4,
        80..85 => 5,
        85..90 => 6,
        90..95 => 7,
        95..98 => 8,
        _ => 9,
    });
    medir("mezcla aleatoria", &aleatorio);

    // Caso más realista: capas macizas por altura con un 3 % de detalles,
    // y la mitad de arriba hueca con muros de 1 bloque cada 10.
    let estructura = schematic(&paleta, |x, y, z, r| {
        let relleno = match y {
            0..30 => 1,
            30..45 => 2,
            45..50 => 10,
            _ if x % 10 == 0 || z % 10 == 0 => 3,
            _ if y % 10 == 0 => 3,
            _ => 0,
        };
        if relleno != 0 && r < 3 {
            5 + r
        } else {
            relleno
        }
    });
    medir("estructura", &estructura);

    // Bloque macizo de piedra: lo mejor que puede ir el greedy.
    let macizo = schematic(&paleta, |_, _, _, _| 1);
    medir("macizo", &macizo);
}
