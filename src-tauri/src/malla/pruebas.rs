// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Pruebas del mallado: culling, greedy, modelos aproximados, varias regiones,
//! corte por capas y formato binario.

use std::collections::{HashMap, HashSet};

use super::apariencia::{clave_visual, color_de};
use super::binario::{self, CABECERA, VERSION};
use super::formas::{Forma, Pieza, forma_de};
use super::*;
use crate::schematic::modelo::{Dimensiones, Formato, Region, Schematic, caja_englobante};

/// Región con los bloques dados (posiciones relativas a `origen`); el resto aire.
fn region(nombre: &str, origen: [i32; 3], bloques: &[([i32; 3], &str)]) -> Region {
    let mut max = [1i32; 3];
    for (p, _) in bloques {
        for e in 0..3 {
            max[e] = max[e].max(p[e] + 1);
        }
    }
    let mut paleta = vec!["minecraft:air".to_string()];
    let mut datos = vec![0u32; (max[0] * max[1] * max[2]) as usize];
    for (p, estado) in bloques {
        let i = match paleta.iter().position(|e| e == estado) {
            Some(i) => i,
            None => {
                paleta.push(estado.to_string());
                paleta.len() - 1
            }
        };
        datos[((p[1] * max[2] + p[2]) * max[0] + p[0]) as usize] = i as u32;
    }
    Region {
        nombre: nombre.into(),
        origen,
        tamano: Dimensiones {
            ancho: max[0] as u32,
            alto: max[1] as u32,
            largo: max[2] as u32,
        },
        paleta,
        bloques: datos,
        extras: Default::default(),
    }
}

fn schem(regiones: Vec<Region>) -> Schematic {
    Schematic {
        formato: Formato::Litematic,
        nombre: None,
        autor: None,
        descripcion: None,
        version_datos: None,
        fecha_creacion: None,
        dimensiones: caja_englobante(&regiones),
        regiones,
    }
}

fn mundo(bloques: &[([i32; 3], &str)]) -> Mundo {
    Mundo::desde_schematic(&schem(vec![region("r", [0, 0, 0], bloques)])).unwrap()
}

fn mallar_con(bloques: &[([i32; 3], &str)], opciones: Opciones) -> Malla {
    mallar(&mundo(bloques), &opciones).unwrap()
}

fn malla_de(bloques: &[([i32; 3], &str)]) -> Malla {
    mallar_con(bloques, Opciones::default())
}

fn quads(m: &Malla) -> usize {
    m.chunks.iter().map(|c| c.indices.len() / 6).sum()
}

/// Recorre los quads: (chunk, índice del primer vértice).
fn cada_quad(m: &Malla) -> impl Iterator<Item = (&ChunkMalla, usize)> {
    m.chunks
        .iter()
        .flat_map(|c| c.indices.chunks(6).map(move |t| (c, t[0] as usize)))
}

fn vertice(c: &ChunkMalla, i: usize) -> [f64; 3] {
    std::array::from_fn(|e| f64::from(c.posiciones[i * 3 + e]) / 16.0)
}

fn resta(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cruzar(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Área de un quad en bloques².
fn area_quad(c: &ChunkMalla, i: usize) -> f64 {
    let p0 = vertice(c, i);
    let n = cruzar(resta(vertice(c, i + 1), p0), resta(vertice(c, i + 3), p0));
    (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
}

fn area(m: &Malla) -> f64 {
    cada_quad(m).map(|(c, i)| area_quad(c, i)).sum()
}

/// Área por (nombre del bloque, cara).
fn area_por_material(m: &Malla) -> HashMap<(String, Cara), f64> {
    let mut v = HashMap::new();
    for (c, i) in cada_quad(m) {
        let mat = &m.materiales[c.materiales[i] as usize];
        *v.entry((mat.nombre.clone(), mat.cara)).or_insert(0.0) += area_quad(c, i);
    }
    v
}

fn area_de(m: &Malla, nombre: &str) -> f64 {
    area_por_material(m)
        .iter()
        .filter(|((n, _), _)| n == nombre)
        .map(|(_, a)| a)
        .sum()
}

/// Vértices (en bloques, coordenadas del modelo) de todos los quads.
fn vertices_modelo(m: &Malla) -> Vec<[f64; 3]> {
    let mut v = Vec::new();
    for c in &m.chunks {
        for i in 0..c.vertices() {
            let p = vertice(c, i);
            v.push(std::array::from_fn(|e| p[e] + f64::from(c.origen[e])));
        }
    }
    v
}

const PIEDRA: &str = "minecraft:stone";
const TIERRA: &str = "minecraft:dirt";
const CRISTAL: &str = "minecraft:glass";
const AGUA: &str = "minecraft:water[level=0]";

// --- Culling ---------------------------------------------------------------

#[test]
fn dos_cubos_distintos_adyacentes_dan_10_caras() {
    let m = malla_de(&[([0, 0, 0], PIEDRA), ([1, 0, 0], TIERRA)]);
    assert_eq!(quads(&m), 10);
    assert_eq!(area(&m), 10.0);
    assert_eq!(area_de(&m, PIEDRA), 5.0);
}

#[test]
fn dos_cubos_iguales_se_fusionan_en_6_quads() {
    let m = malla_de(&[([0, 0, 0], PIEDRA), ([1, 0, 0], PIEDRA)]);
    assert_eq!(quads(&m), 6);
    assert_eq!(area(&m), 10.0);
    let sin = mallar_con(
        &[([0, 0, 0], PIEDRA), ([1, 0, 0], PIEDRA)],
        Opciones {
            greedy: false,
            capas: None,
        },
    );
    assert_eq!(quads(&sin), 10);
}

#[test]
fn cubo_rodeado_no_emite_nada() {
    let mut b = Vec::new();
    for y in 0..3 {
        for z in 0..3 {
            for x in 0..3 {
                let e = if [x, y, z] == [1, 1, 1] {
                    TIERRA
                } else {
                    PIEDRA
                };
                b.push(([x, y, z], e));
            }
        }
    }
    let m = malla_de(&b);
    assert_eq!(area_de(&m, TIERRA), 0.0);
    assert_eq!(area(&m), 54.0);
    assert_eq!(quads(&m), 6);
}

#[test]
fn cristal_junto_a_piedra() {
    let m = malla_de(&[([0, 0, 0], PIEDRA), ([1, 0, 0], CRISTAL)]);
    // La piedra enseña sus 6 caras (el cristal no oculta); el cristal, 5.
    assert_eq!(area_de(&m, PIEDRA), 6.0);
    assert_eq!(area_de(&m, CRISTAL), 5.0);
    let a = area_por_material(&m);
    assert!(!a.contains_key(&(CRISTAL.to_string(), Cara::Oeste)));
    assert_eq!(a[&(PIEDRA.to_string(), Cara::Este)], 1.0);
}

#[test]
fn iguales_translucidos_comparten_cara_oculta() {
    for e in [
        CRISTAL,
        AGUA,
        "minecraft:oak_leaves[distance=1,persistent=false]",
    ] {
        let m = malla_de(&[([0, 0, 0], e), ([1, 0, 0], e)]);
        assert_eq!(area(&m), 10.0, "{e}");
    }
    // Hojas con distinta `distance`: mismo material, así que se fusionan.
    let m = malla_de(&[
        (
            [0, 0, 0],
            "minecraft:oak_leaves[distance=1,persistent=false]",
        ),
        (
            [1, 0, 0],
            "minecraft:oak_leaves[distance=2,persistent=false]",
        ),
    ]);
    assert_eq!(quads(&m), 6);
    // Agua con distinto `level` también se trata como la misma.
    let m = malla_de(&[([0, 0, 0], AGUA), ([1, 0, 0], "minecraft:water[level=3]")]);
    assert_eq!(area(&m), 10.0);
}

#[test]
fn translucidos_distintos_no_se_ocultan() {
    let m = malla_de(&[([0, 0, 0], CRISTAL), ([1, 0, 0], AGUA)]);
    assert_eq!(area(&m), 12.0);
}

#[test]
fn translucidos_van_al_final_de_los_indices() {
    let m = malla_de(&[([0, 0, 0], PIEDRA), ([2, 0, 0], CRISTAL)]);
    let c = &m.chunks[0];
    assert_eq!(c.indices_opacos, 36);
    assert_eq!(c.indices.len(), 72);
    for t in c.indices[36..].chunks(6) {
        let mat = &m.materiales[c.materiales[t[0] as usize] as usize];
        assert_eq!(mat.tipo, Tipo::Translucido);
    }
}

#[test]
fn invisibles_no_generan_chunks() {
    let m = malla_de(&[
        ([0, 0, 0], "minecraft:structure_void"),
        ([1, 0, 0], "minecraft:light[level=15]"),
        ([2, 0, 0], "minecraft:barrier"),
        ([3, 0, 0], "minecraft:cave_air"),
    ]);
    assert!(m.chunks.is_empty());
    // Pero el bloque invisible no oculta a su vecino.
    let m = malla_de(&[([0, 0, 0], "minecraft:structure_void"), ([1, 0, 0], PIEDRA)]);
    assert_eq!(area(&m), 6.0);
}

// --- Greedy ----------------------------------------------------------------

#[test]
fn losa_de_piedra_4x4_son_6_quads() {
    let mut b = Vec::new();
    for z in 0..4 {
        for x in 0..4 {
            b.push(([x, 0, z], PIEDRA));
        }
    }
    let m = malla_de(&b);
    assert_eq!(quads(&m), 6);
    assert_eq!(area(&m), 16.0 * 2.0 + 4.0 * 4.0);
    // Las UV van en unidades de bloque (×16): la cara de arriba abarca 4×4.
    let c = &m.chunks[0];
    let (i, _) = (0..c.vertices())
        .map(|i| (i, c.normales[i]))
        .find(|&(_, n)| n == Cara::Arriba as u8)
        .unwrap();
    let base = i - i % 4;
    let us: Vec<u16> = (base..base + 4).map(|j| c.uvs[j * 2]).collect();
    let vs: Vec<u16> = (base..base + 4).map(|j| c.uvs[j * 2 + 1]).collect();
    assert_eq!(us.iter().max().unwrap() - us.iter().min().unwrap(), 64);
    assert_eq!(vs.iter().max().unwrap() - vs.iter().min().unwrap(), 64);
}

#[test]
fn greedy_no_mezcla_materiales() {
    let m = malla_de(&[
        ([0, 0, 0], PIEDRA),
        ([1, 0, 0], TIERRA),
        ([2, 0, 0], PIEDRA),
    ]);
    // Arriba, abajo, norte y sur: 3 quads cada una; más los dos extremos.
    assert_eq!(quads(&m), 14);
}

// --- Modelos aproximados -----------------------------------------------------

#[test]
fn losas() {
    let m = malla_de(&[(
        [0, 0, 0],
        "minecraft:stone_slab[type=bottom,waterlogged=false]",
    )]);
    assert_eq!(quads(&m), 6);
    let ys: Vec<f64> = vertices_modelo(&m).iter().map(|p| p[1]).collect();
    assert_eq!(ys.iter().cloned().fold(f64::MIN, f64::max), 0.5);

    let m = malla_de(&[([0, 0, 0], "minecraft:stone_slab[type=top]")]);
    let ys: Vec<f64> = vertices_modelo(&m).iter().map(|p| p[1]).collect();
    assert_eq!(ys.iter().cloned().fold(f64::MAX, f64::min), 0.5);

    // Sobre piedra: se tapan la cara de abajo de la losa y la de arriba de la
    // piedra (la losa cubre entera su cara inferior, como en Minecraft).
    let m = malla_de(&[
        ([0, 0, 0], PIEDRA),
        ([0, 1, 0], "minecraft:stone_slab[type=bottom]"),
    ]);
    assert_eq!(quads(&m), 10);
    // Bajo una losa superior la piedra sí enseña su cara de arriba.
    let m = malla_de(&[
        ([0, 0, 0], PIEDRA),
        ([0, 1, 0], "minecraft:stone_slab[type=top]"),
    ]);
    assert_eq!(quads(&m), 12);

    // Dos losas inferiores juntas se tapan la cara compartida (media altura).
    let losa = "minecraft:oak_slab[type=bottom]";
    let m = malla_de(&[([0, 0, 0], losa), ([1, 0, 0], losa)]);
    assert_eq!(quads(&m), 10);
    // Losa inferior junto a superior: no coinciden, se ven las dos.
    let m = malla_de(&[
        ([0, 0, 0], losa),
        ([1, 0, 0], "minecraft:oak_slab[type=top]"),
    ]);
    assert_eq!(quads(&m), 12);

    // Doble = cubo: oculta y se fusiona con sus iguales.
    let doble = "minecraft:stone_slab[type=double]";
    let m = malla_de(&[([0, 0, 0], PIEDRA), ([1, 0, 0], doble)]);
    assert_eq!(area(&m), 10.0);
}

#[test]
fn escaleras_segun_facing_y_half() {
    let alto = |estado: &str| {
        let m = malla_de(&[([0, 0, 0], estado)]);
        // 2 cajas × 6 caras menos la que el escalón apoya en la base.
        assert_eq!(quads(&m), 11, "{estado}");
        let v = vertices_modelo(&m);
        let arriba: Vec<&[f64; 3]> = v.iter().filter(|p| p[1] == 1.0).collect();
        let rango = |e: usize| {
            let a = arriba.iter().map(|p| p[e]).fold(f64::MAX, f64::min);
            let b = arriba.iter().map(|p| p[e]).fold(f64::MIN, f64::max);
            (a, b)
        };
        (rango(0), rango(2))
    };
    // El escalón (la parte alta) queda hacia donde mira `facing`.
    let (x, z) = alto("minecraft:oak_stairs[facing=north,half=bottom,shape=straight]");
    assert_eq!((x, z), ((0.0, 1.0), (0.0, 0.5)));
    let (x, z) = alto("minecraft:oak_stairs[facing=east,half=bottom,shape=straight]");
    assert_eq!((x, z), ((0.5, 1.0), (0.0, 1.0)));
    let (_, z) = alto("minecraft:oak_stairs[facing=south,half=bottom,shape=straight]");
    assert_eq!(z, (0.5, 1.0));
    let (x, _) = alto("minecraft:oak_stairs[facing=west,half=bottom,shape=straight]");
    assert_eq!(x, (0.0, 0.5));
    // Al revés: la losa entera arriba.
    let (x, z) = alto("minecraft:oak_stairs[facing=north,half=top,shape=straight]");
    assert_eq!((x, z), ((0.0, 1.0), (0.0, 1.0)));

    // La espalda de la escalera (lado de `facing`) tapa entera la cara de
    // la piedra que tiene detrás; el frente no.
    let escalera = "minecraft:oak_stairs[facing=north,half=bottom,shape=straight]";
    let m = malla_de(&[([0, 0, 0], PIEDRA), ([0, 0, 1], escalera)]);
    assert_eq!(area_de(&m, PIEDRA), 5.0);
    let m = malla_de(&[([0, 0, 1], PIEDRA), ([0, 0, 0], escalera)]);
    assert_eq!(area_de(&m, PIEDRA), 6.0);
}

#[test]
fn vallas_muros_y_paneles() {
    let valla =
        "minecraft:oak_fence[east=false,north=true,south=false,waterlogged=false,west=false]";
    let Forma::Modelo(_, piezas) = forma_de(valla) else {
        panic!("la valla debe ser un modelo");
    };
    assert_eq!(piezas.len(), 3); // poste + dos travesaños hacia el norte
    // 3 cajas × 6 caras menos los extremos de los travesaños metidos en el poste.
    assert_eq!(quads(&malla_de(&[([0, 0, 0], valla)])), 16);

    let muro = "minecraft:cobblestone_wall[east=tall,north=low,south=none,up=true,waterlogged=false,west=none]";
    let Forma::Modelo(_, piezas) = forma_de(muro) else {
        panic!("el muro debe ser un modelo");
    };
    assert_eq!(piezas.len(), 3);

    let panel = "minecraft:glass_pane[east=true,north=false,south=false,west=true]";
    assert!(matches!(forma_de(panel), Forma::Modelo(Tipo::Translucido, ref p) if p.len() == 3));
}

#[test]
fn planta_cruzada_son_cuatro_quads_diagonales() {
    let m = malla_de(&[([0, 0, 0], "minecraft:poppy")]);
    assert_eq!(quads(&m), 4);
    let normales: HashSet<u8> = m.chunks[0].normales.iter().copied().collect();
    assert_eq!(normales, HashSet::from([6, 7, 8, 9]));
    assert_eq!(m.materiales.len(), 1);
    assert_eq!(m.materiales[0].cara, Cara::Cruz);
    assert_eq!(m.materiales[0].tipo, Tipo::Recorte);
}

#[test]
fn clasificacion_por_nombre() {
    use Tipo::*;
    let casos: &[(&str, Forma)] = &[
        ("minecraft:air", Forma::Nada),
        ("minecraft:stone", Forma::Cubo(Opaco)),
        ("minecraft:white_stained_glass", Forma::Cubo(Translucido)),
        ("minecraft:birch_leaves[distance=7]", Forma::Cubo(Recorte)),
        ("minecraft:lava[level=0]", Forma::Cubo(Opaco)),
        ("minecraft:snow[layers=8]", Forma::Cubo(Opaco)),
        ("minecraft:jack_o_lantern[facing=north]", Forma::Cubo(Opaco)),
        ("minecraft:crimson_stem[axis=y]", Forma::Cubo(Opaco)),
        ("minecraft:mushroom_stem", Forma::Cubo(Opaco)),
        ("minecraft:piston_head[facing=up]", Forma::Cubo(Opaco)),
        ("mod:cosa_rara", Forma::Cubo(Opaco)),
        (
            "minecraft:oak_sapling[stage=0]",
            Forma::Modelo(Recorte, vec![Pieza::Cruz]),
        ),
    ];
    for (estado, esperada) in casos {
        assert_eq!(&forma_de(estado), esperada, "{estado}");
    }
    for estado in [
        "minecraft:red_carpet",
        "minecraft:stone_pressure_plate[powered=false]",
        "minecraft:snow[layers=3]",
        "minecraft:torch",
        "minecraft:wall_torch[facing=east]",
        "minecraft:oak_door[facing=north,half=lower]",
        "minecraft:rail[shape=north_south]",
        "minecraft:lantern[hanging=false]",
    ] {
        assert!(matches!(forma_de(estado), Forma::Modelo(..)), "{estado}");
    }
}

/// Todas las caras deben ir en sentido antihorario visto desde su normal.
#[test]
fn orden_de_vertices_coincide_con_la_normal() {
    let n = std::f64::consts::FRAC_1_SQRT_2;
    let normales: [[f64; 3]; 10] = [
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [n, 0.0, n],
        [-n, 0.0, -n],
        [n, 0.0, -n],
        [-n, 0.0, n],
    ];
    let m = malla_de(&[
        ([0, 0, 0], PIEDRA),
        ([2, 0, 0], "minecraft:oak_stairs[facing=west,half=top]"),
        ([4, 0, 0], "minecraft:poppy"),
        (
            [6, 0, 0],
            "minecraft:oak_fence[north=true,east=true,south=true,west=true]",
        ),
        ([8, 0, 0], CRISTAL),
    ]);
    for (c, i) in cada_quad(&m) {
        let p0 = vertice(c, i);
        let g = cruzar(resta(vertice(c, i + 1), p0), resta(vertice(c, i + 2), p0));
        let esperada = normales[c.normales[i] as usize];
        let punto = g[0] * esperada[0] + g[1] * esperada[1] + g[2] * esperada[2];
        assert!(punto > 0.0, "quad con normal {} al revés", c.normales[i]);
    }
}

// --- Regiones, chunks y capas ----------------------------------------------

#[test]
fn varias_regiones_con_origen() {
    let s = schem(vec![
        region("a", [0, 0, 0], &[([0, 0, 0], PIEDRA)]),
        region("b", [10, 5, -3], &[([0, 0, 0], CRISTAL)]),
    ]);
    let mundo = Mundo::desde_schematic(&s).unwrap();
    assert_eq!(mundo.origen, [0, 0, -3]);
    assert_eq!(mundo.tamano, [11, 6, 4]);
    let b = mundo.bloque_en(10, 5, 0).unwrap();
    assert_eq!(b.estado, CRISTAL);
    assert_eq!(b.posicion, [10, 5, -3]);
    assert_eq!(b.region.as_deref(), Some("b"));
    let a = mundo.bloque_en(0, 0, 3).unwrap();
    assert_eq!(a.posicion, [0, 0, 0]);
    assert_eq!(a.region.as_deref(), Some("a"));
    assert!(mundo.bloque_en(5, 5, 0).is_none());
    assert!(mundo.bloque_en(-1, 0, 0).is_none());

    let m = mallar(&mundo, &Opciones::default()).unwrap();
    assert_eq!(m.origen, [0, 0, -3]);
    assert_eq!(area(&m), 12.0);
    // Los vértices van en coordenadas del modelo: la piedra en z = 3..4.
    let v = vertices_modelo(&m);
    assert!(v.contains(&[0.0, 0.0, 3.0]));
    assert!(v.contains(&[11.0, 6.0, 1.0]));
}

#[test]
fn propiedades_de_un_bloque_seleccionado() {
    let mundo = mundo(&[([0, 0, 0], "minecraft:oak_stairs[facing=north,half=bottom]")]);
    let b = mundo.bloque_en(0, 0, 0).unwrap();
    assert_eq!(b.nombre, "minecraft:oak_stairs");
    assert_eq!(b.propiedades["facing"], "north");
    assert_eq!(b.propiedades["half"], "bottom");
}

#[test]
fn las_regiones_solapadas_las_gana_la_ultima() {
    let s = schem(vec![
        region("a", [0, 0, 0], &[([0, 0, 0], PIEDRA), ([1, 0, 0], PIEDRA)]),
        region("b", [1, 0, 0], &[([0, 0, 0], TIERRA)]),
    ]);
    let mundo = Mundo::desde_schematic(&s).unwrap();
    assert_eq!(mundo.bloque_en(1, 0, 0).unwrap().estado, TIERRA);
    assert_eq!(mundo.bloque_en(0, 0, 0).unwrap().estado, PIEDRA);
    assert_eq!(mundo.total_bloques(), 2);
}

#[test]
fn culling_entre_chunks_vecinos() {
    let m = malla_de(&[([15, 0, 0], PIEDRA), ([16, 0, 0], TIERRA)]);
    assert_eq!(m.chunks.len(), 2);
    assert_eq!(area(&m), 10.0);
    assert_eq!(m.chunks[1].origen, [16, 0, 0]);
    let m = malla_de(&[([0, 15, 0], PIEDRA), ([0, 16, 0], CRISTAL)]);
    assert_eq!(area(&m), 11.0);
}

#[test]
fn capas_por_chunk() {
    let b: Vec<([i32; 3], &str)> = (0..21).map(|y| ([0, y, 0], PIEDRA)).collect();
    let m = malla_de(&b);
    assert_eq!(m.chunks.len(), 2);
    assert_eq!((m.chunks[0].y_min, m.chunks[0].y_max), (0, 15));
    assert_eq!((m.chunks[1].y_min, m.chunks[1].y_max), (16, 20));
}

#[test]
fn corte_por_capas_cierra_la_malla() {
    let b: Vec<([i32; 3], &str)> = (0..3).map(|y| ([0, y, 0], PIEDRA)).collect();
    let m = mallar_con(
        &b,
        Opciones {
            greedy: true,
            capas: Some([1, 1]),
        },
    );
    assert_eq!(area(&m), 6.0);
    assert_eq!((m.chunks[0].y_min, m.chunks[0].y_max), (1, 1));
}

// --- Apariencia ---------------------------------------------------------------

#[test]
fn clave_visual_y_color_estable() {
    assert_eq!(
        clave_visual("minecraft:oak_leaves[distance=3,persistent=true,waterlogged=false]"),
        "minecraft:oak_leaves"
    );
    assert_eq!(
        clave_visual("minecraft:oak_log[axis=x]"),
        "minecraft:oak_log[axis=x]"
    );
    assert_eq!(clave_visual("minecraft:water[level=4]"), "minecraft:water");
    assert_eq!(color_de("minecraft:stone"), color_de("minecraft:stone"));
    assert_ne!(color_de("minecraft:stone"), color_de("minecraft:dirt"));
}

// --- Formato binario ---------------------------------------------------------

#[test]
fn binario_ida_y_vuelta() {
    let s = schem(vec![
        region(
            "a",
            [0, 0, 0],
            &[([0, 0, 0], PIEDRA), ([17, 0, 0], "minecraft:poppy")],
        ),
        region("b", [-4, 2, 7], &[([0, 0, 0], CRISTAL)]),
    ]);
    let mundo = Mundo::desde_schematic(&s).unwrap();
    let m = mallar(&mundo, &Opciones::default()).unwrap();
    let bytes = binario::serializar(&m);
    assert_eq!(&bytes[0..4], b"L3DM");
    assert_eq!(bytes.len() % 4, 0);

    let (cab, chunks) = binario::leer(&bytes).unwrap();
    assert_eq!(cab.version, VERSION);
    assert_eq!(cab.origen, [-4, 0, 0]);
    assert_eq!(cab.tamano, [22, 3, 8]);
    assert_eq!(cab.n_chunks as usize, m.chunks.len());
    assert_eq!(
        cab.json["materiales"].as_array().unwrap().len(),
        m.materiales.len()
    );
    assert_eq!(cab.json["paleta"][0], "minecraft:air");
    assert_eq!(cab.json["totalBloques"], 3);
    assert_eq!(cab.json["materiales"][0]["cara"], "arriba");
    assert!(cab.json["materiales"][0]["color"].is_array());
    assert!(cab.json["materiales"][0]["textura"].is_null());

    for (a, b) in m.chunks.iter().zip(&chunks) {
        assert_eq!(a.origen, b.origen);
        assert_eq!((a.y_min, a.y_max), (b.y_min, b.y_max));
        assert_eq!(a.posiciones, b.posiciones);
        assert_eq!(a.normales, b.normales);
        assert_eq!(a.uvs, b.uvs);
        assert_eq!(a.materiales, b.materiales);
        assert_eq!(a.indices, b.indices);
        assert_eq!(a.indices_opacos, b.indices_opacos);
        assert_eq!(b.bytes_indice, 2);
    }
    // Primer chunk justo tras la cabecera y el JSON alineado.
    let json = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
    let primero = CABECERA + json.div_ceil(4) * 4;
    let ox = i32::from_le_bytes(bytes[primero..primero + 4].try_into().unwrap());
    assert_eq!(ox, m.chunks[0].origen[0]);
}

#[test]
fn binario_con_indices_de_32_bits() {
    // 16³ vallas con los cuatro brazos: 9 cajas × 24 vértices por bloque.
    let valla = "minecraft:oak_fence[east=true,north=true,south=true,west=true]";
    let mut b = Vec::new();
    for y in 0..16 {
        for z in 0..16 {
            for x in 0..16 {
                b.push(([x, y, z], valla));
            }
        }
    }
    let m = malla_de(&b);
    assert!(m.chunks[0].vertices() > 65_536);
    let (_, chunks) = binario::leer(&binario::serializar(&m)).unwrap();
    assert_eq!(chunks[0].bytes_indice, 4);
    assert_eq!(chunks[0].indices, m.chunks[0].indices);
}

#[test]
fn binario_vacio() {
    let m = malla_de(&[]);
    let (cab, chunks) = binario::leer(&binario::serializar(&m)).unwrap();
    assert_eq!(cab.n_chunks, 0);
    assert!(chunks.is_empty());
}
