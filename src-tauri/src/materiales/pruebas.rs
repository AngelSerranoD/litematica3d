// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Pruebas de la lista de materiales: una por regla del JSON y el desglose.

use super::reglas::{self, PREFIJO_DESCONOCIDO, Reglas, Resolucion};
use super::*;
use crate::schematic::modelo::{Dimensiones, Formato, caja_englobante};

fn reglas() -> &'static Reglas {
    reglas::globales().expect("el JSON de reglas debe leerse")
}

/// Región `nombre` de una sola fila (alto 1, largo 1) con `veces` de cada estado.
fn fila(nombre: &str, origen: [i32; 3], estados: &[(&str, u32)]) -> Region {
    let paleta: Vec<String> = estados.iter().map(|(e, _)| e.to_string()).collect();
    let bloques: Vec<u32> = estados
        .iter()
        .enumerate()
        .flat_map(|(i, &(_, n))| std::iter::repeat_n(i as u32, n as usize))
        .collect();
    Region {
        nombre: nombre.into(),
        origen,
        tamano: Dimensiones {
            ancho: bloques.len() as u32,
            alto: 1,
            largo: 1,
        },
        paleta,
        bloques,
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

fn lista(estados: &[(&str, u32)], opciones: &Opciones) -> ListaMateriales {
    calcular(&schem(vec![fila("r", [0, 0, 0], estados)]), opciones).unwrap()
}

/// Cantidades por ítem, sin opciones.
fn cuentas(estados: &[(&str, u32)]) -> HashMap<String, u64> {
    cuentas_con(estados, &Opciones::default())
}

fn cuentas_con(estados: &[(&str, u32)], opciones: &Opciones) -> HashMap<String, u64> {
    lista(estados, opciones)
        .materiales
        .into_iter()
        .map(|m| (m.item, m.cantidad))
        .collect()
}

fn mapa(pares: &[(&str, u64)]) -> HashMap<String, u64> {
    pares.iter().map(|&(k, v)| (k.to_string(), v)).collect()
}

// ---- El JSON ----

#[test]
fn el_json_parsea_y_sus_items_son_de_minecraft() {
    let r = Reglas::desde_json(reglas::JSON_REGLAS).unwrap();
    let items = r.items_declarados();
    assert!(items.len() > 100);
    for item in items {
        assert!(item.starts_with("minecraft:"), "ítem sin namespace: {item}");
        assert!(
            !item.contains('*') && !item.contains('['),
            "ítem raro: {item}"
        );
    }
}

#[test]
fn rechaza_claves_con_dos_comodines_o_propiedades() {
    let base = |clave: &str| {
        format!(
            r#"{{"version":1,"ignorar":["{clave}"],"item":{{}},"varios":{{}},"solo_si":{{}},
            "cantidad_por_propiedad":{{}},"multiplicador":{{}},"fluidos":{{}},"stack":{{}}}}"#
        )
    };
    assert!(Reglas::desde_json(&base("minecraft:*_a_*")).is_err());
    assert!(Reglas::desde_json(&base("minecraft:a[b=c]")).is_err());
    assert!(Reglas::desde_json(&base("minecraft:*_a")).is_ok());
}

// ---- Reglas ----

#[test]
fn aire_e_ignorados_no_cuentan() {
    let c = cuentas(&[
        ("minecraft:air", 5),
        ("minecraft:cave_air", 1),
        ("minecraft:void_air", 1),
        ("minecraft:barrier", 2),
        ("minecraft:light[level=15,waterlogged=false]", 1),
        ("minecraft:structure_void", 1),
        // No se obtienen como ítem en supervivencia.
        (
            "minecraft:chorus_plant[down=true,east=false,north=false,south=false,up=true,west=false]",
            1,
        ),
        ("minecraft:frogspawn", 2),
        ("minecraft:stone", 3),
    ]);
    assert_eq!(c, mapa(&[("minecraft:stone", 3)]));
}

#[test]
fn bloques_tecnicos_no_cuentan() {
    let c = cuentas(&[
        (
            "minecraft:piston_head[facing=up,short=false,type=normal]",
            1,
        ),
        ("minecraft:moving_piston[facing=up,type=normal]", 1),
        ("minecraft:bubble_column[drag=true]", 1),
        (
            "minecraft:fire[age=0,east=false,north=false,south=false,up=false,west=false]",
            1,
        ),
        ("minecraft:nether_portal[axis=x]", 1),
        ("minecraft:end_gateway", 1),
        ("minecraft:frosted_ice[age=0]", 1),
        ("minecraft:piston[extended=true,facing=up]", 1),
    ]);
    assert_eq!(c, mapa(&[("minecraft:piston", 1)]));
}

#[test]
fn item_uno_a_uno() {
    let c = cuentas(&[
        ("minecraft:wall_torch[facing=north]", 2),
        ("minecraft:torch", 1),
        ("minecraft:oak_wall_sign[facing=east,waterlogged=false]", 1),
        (
            "minecraft:redstone_wire[east=none,north=side,power=0,south=side,west=none]",
            4,
        ),
        (
            "minecraft:tripwire[attached=false,disarmed=false,east=true,north=false,powered=false,south=false,west=true]",
            3,
        ),
        ("minecraft:wheat[age=7]", 5),
        ("minecraft:carrots[age=2]", 1),
    ]);
    assert_eq!(
        c,
        mapa(&[
            ("minecraft:torch", 3),
            ("minecraft:oak_sign", 1),
            ("minecraft:redstone", 4),
            ("minecraft:string", 3),
            ("minecraft:wheat_seeds", 5),
            ("minecraft:carrot", 1),
        ])
    );
}

#[test]
fn maceta_da_dos_items() {
    let c = cuentas(&[
        ("minecraft:potted_poppy", 2),
        ("minecraft:potted_cactus", 1),
        ("minecraft:flower_pot", 1),
    ]);
    assert_eq!(
        c,
        mapa(&[
            ("minecraft:flower_pot", 4),
            ("minecraft:poppy", 2),
            ("minecraft:cactus", 1),
        ])
    );
}

#[test]
fn puerta_cuenta_solo_la_mitad_inferior() {
    let c = cuentas(&[
        (
            "minecraft:oak_door[facing=north,half=lower,hinge=left,open=false,powered=false]",
            3,
        ),
        (
            "minecraft:oak_door[facing=north,half=upper,hinge=left,open=false,powered=false]",
            3,
        ),
        (
            "minecraft:iron_door[facing=east,half=lower,hinge=right,open=true,powered=false]",
            1,
        ),
        (
            "minecraft:iron_door[facing=east,half=upper,hinge=right,open=true,powered=false]",
            1,
        ),
        // Las trampillas no son puertas: `*_door` no debe atraparlas.
        (
            "minecraft:oak_trapdoor[facing=north,half=top,open=false,powered=false,waterlogged=false]",
            2,
        ),
    ]);
    assert_eq!(
        c,
        mapa(&[
            ("minecraft:oak_door", 3),
            ("minecraft:iron_door", 1),
            ("minecraft:oak_trapdoor", 2),
        ])
    );
}

#[test]
fn cama_cuenta_solo_la_cabecera() {
    let l = lista(
        &[
            (
                "minecraft:red_bed[facing=north,occupied=false,part=head]",
                2,
            ),
            (
                "minecraft:red_bed[facing=north,occupied=false,part=foot]",
                2,
            ),
        ],
        &Opciones::default(),
    );
    assert_eq!(l.materiales.len(), 1);
    assert_eq!(l.materiales[0].item, "minecraft:red_bed");
    assert_eq!(l.materiales[0].cantidad, 2);
    assert_eq!(l.materiales[0].tamano_stack, 1);
}

#[test]
fn planta_alta_cuenta_una_vez() {
    let c = cuentas(&[
        ("minecraft:sunflower[half=lower]", 4),
        ("minecraft:sunflower[half=upper]", 4),
        ("minecraft:tall_grass[half=lower]", 1),
        ("minecraft:tall_grass[half=upper]", 1),
        // Está en `item` (-> seagrass) y en `solo_si`: solo_si mira el ID original.
        ("minecraft:tall_seagrass[half=lower]", 2),
        ("minecraft:tall_seagrass[half=upper]", 2),
    ]);
    assert_eq!(
        c,
        mapa(&[
            ("minecraft:sunflower", 4),
            ("minecraft:tall_grass", 1),
            ("minecraft:seagrass", 2),
        ])
    );
}

#[test]
fn losa_doble_cuenta_dos() {
    let c = cuentas(&[
        ("minecraft:stone_slab[type=double,waterlogged=false]", 3),
        ("minecraft:stone_slab[type=bottom,waterlogged=false]", 2),
        ("minecraft:oak_slab[type=top,waterlogged=true]", 1),
    ]);
    assert_eq!(
        c,
        mapa(&[("minecraft:stone_slab", 8), ("minecraft:oak_slab", 1)])
    );
}

#[test]
fn cantidad_segun_propiedad() {
    let c = cuentas(&[
        ("minecraft:candle[candles=3,lit=false,waterlogged=false]", 2),
        (
            "minecraft:red_candle[candles=4,lit=true,waterlogged=false]",
            1,
        ),
        ("minecraft:sea_pickle[pickles=2,waterlogged=true]", 3),
        ("minecraft:snow[layers=5]", 2),
        ("minecraft:turtle_egg[eggs=4,hatch=0]", 1),
        ("minecraft:pink_petals[facing=north,flower_amount=3]", 2),
        ("minecraft:leaf_litter[facing=north,segment_amount=2]", 1),
    ]);
    assert_eq!(
        c,
        mapa(&[
            ("minecraft:candle", 6),
            ("minecraft:red_candle", 4),
            ("minecraft:sea_pickle", 6),
            ("minecraft:snow", 10),
            ("minecraft:turtle_egg", 4),
            ("minecraft:pink_petals", 6),
            ("minecraft:leaf_litter", 2),
        ])
    );
}

#[test]
fn tarta_con_vela_no_usa_la_cantidad_de_velas() {
    let c = cuentas(&[("minecraft:red_candle_cake[lit=false]", 2)]);
    assert_eq!(
        c,
        mapa(&[("minecraft:cake", 2), ("minecraft:red_candle", 2)])
    );
}

#[test]
fn fluidos_excluidos_por_defecto() {
    let c = cuentas(&[
        ("minecraft:water[level=0]", 5),
        ("minecraft:lava[level=0]", 2),
        ("minecraft:stone", 1),
    ]);
    assert_eq!(c, mapa(&[("minecraft:stone", 1)]));
}

#[test]
fn fluidos_con_la_opcion_solo_fuentes() {
    let opciones = Opciones {
        incluir_fluidos: true,
        ..Opciones::default()
    };
    let l = lista(
        &[
            ("minecraft:water[level=0]", 5),
            ("minecraft:water[level=3]", 7),
            ("minecraft:lava[level=0]", 2),
            ("minecraft:lava[level=8]", 1),
        ],
        &opciones,
    );
    let c: HashMap<String, u64> = l
        .materiales
        .iter()
        .map(|m| (m.item.clone(), m.cantidad))
        .collect();
    assert_eq!(
        c,
        mapa(&[("minecraft:water_bucket", 5), ("minecraft:lava_bucket", 2)])
    );
    assert!(l.materiales.iter().all(|m| m.tamano_stack == 1));
}

#[test]
fn desconocidos_agrupados_sin_fallar() {
    let id = |n: u32| format!("{PREFIJO_DESCONOCIDO}{n}_0");
    let (a, b) = (id(4000), id(4001));
    let l = lista(
        &[(a.as_str(), 3), (b.as_str(), 2), ("minecraft:stone", 1)],
        &Opciones::default(),
    );
    assert_eq!(l.desconocidos, 5);
    let d = l
        .materiales
        .iter()
        .find(|m| m.item == ITEM_DESCONOCIDO)
        .unwrap();
    assert_eq!(d.cantidad, 5);
    assert_eq!(l.materiales.len(), 2);
    assert_eq!(l.total_items, 6);
}

#[test]
fn filtro_por_region() {
    let s = schem(vec![
        fila("Casa", [0, 0, 0], &[("minecraft:stone", 3)]),
        fila(
            "Torre",
            [0, 5, 0],
            &[("minecraft:stone", 4), ("minecraft:dirt", 1)],
        ),
    ]);
    let todas = calcular(&s, &Opciones::default()).unwrap();
    assert_eq!(todas.total_items, 8);
    let torre = calcular(
        &s,
        &Opciones {
            region: Some("Torre".into()),
            ..Opciones::default()
        },
    )
    .unwrap();
    assert_eq!(torre.total_items, 5);
    assert_eq!(torre.materiales[0].item, "minecraft:stone");
    assert_eq!(torre.materiales[0].cantidad, 4);
    let falta = calcular(
        &s,
        &Opciones {
            region: Some("Sótano".into()),
            ..Opciones::default()
        },
    );
    assert!(matches!(falta, Err(ErrorApp::RegionNoEncontrada { .. })));
}

#[test]
fn filtro_por_altura_en_coordenadas_del_schematic() {
    // Región de 1×4×2 (ancho×alto×largo) con origen en Y=10: capas Y 10..=13.
    let paleta = [
        "minecraft:stone",
        "minecraft:dirt",
        "minecraft:oak_planks",
        "minecraft:glass",
    ];
    let region = Region {
        nombre: "r".into(),
        origen: [0, 10, 0],
        tamano: Dimensiones {
            ancho: 1,
            alto: 4,
            largo: 2,
        },
        paleta: paleta.iter().map(|s| s.to_string()).collect(),
        // Capa y local -> índice de paleta y (dos bloques por capa).
        bloques: vec![0, 0, 1, 1, 2, 2, 3, 3],
    };
    let s = schem(vec![
        region,
        fila("abajo", [0, 0, 0], &[("minecraft:stone", 5)]),
    ]);
    let rango = |y_min, y_max| {
        calcular(
            &s,
            &Opciones {
                y_min,
                y_max,
                ..Opciones::default()
            },
        )
        .unwrap()
        .materiales
        .into_iter()
        .map(|m| (m.item, m.cantidad))
        .collect::<HashMap<_, _>>()
    };
    // Inclusivo por los dos lados.
    assert_eq!(
        rango(Some(11), Some(12)),
        mapa(&[("minecraft:dirt", 2), ("minecraft:oak_planks", 2)])
    );
    assert_eq!(rango(Some(13), None), mapa(&[("minecraft:glass", 2)]));
    assert_eq!(rango(None, Some(10)), mapa(&[("minecraft:stone", 7)]));
    assert_eq!(rango(Some(0), Some(0)), mapa(&[("minecraft:stone", 5)]));
    // Fuera de todo y rango al revés: nada.
    assert!(rango(Some(50), Some(60)).is_empty());
    assert!(rango(Some(12), Some(11)).is_empty());
}

#[test]
fn la_clave_exacta_gana_al_comodin() {
    let json = r#"{
        "version": 1,
        "ignorar": ["minecraft:*_glass", "minecraft:tinted_glass_x"],
        "item": {"minecraft:*_wall_torch": "minecraft:torch", "minecraft:redstone_wall_torch": "minecraft:redstone_torch"},
        "varios": {},
        "solo_si": {},
        "cantidad_por_propiedad": {},
        "multiplicador": {
            "minecraft:*_slab": {"type": {"double": 2}},
            "minecraft:oak_slab": {"type": {"double": 5}},
            "minecraft:*": {"type": {"double": 9}}
        },
        "fluidos": {},
        "stack": {}
    }"#;
    let r = Reglas::desde_json(json).unwrap();
    let items = |estado: &str| match r.resolver(estado, false) {
        Resolucion::Items(v) => v,
        otro => panic!("{estado}: {otro:?}"),
    };
    assert_eq!(
        items("minecraft:oak_slab[type=double]"),
        vec![("minecraft:oak_slab".into(), 5)]
    );
    assert_eq!(
        items("minecraft:stone_slab[type=double]"),
        vec![("minecraft:stone_slab".into(), 2)]
    );
    // Entre comodines gana el más específico.
    assert_eq!(
        items("minecraft:stone[type=double]"),
        vec![("minecraft:stone".into(), 9)]
    );
    assert_eq!(
        items("minecraft:soul_wall_torch[facing=north]"),
        vec![("minecraft:torch".into(), 1)]
    );
    assert_eq!(
        items("minecraft:redstone_wall_torch[facing=north,lit=true]"),
        vec![("minecraft:redstone_torch".into(), 1)]
    );
    assert_eq!(
        r.resolver("minecraft:red_stained_glass", false),
        Resolucion::Nada
    );
    // El comodín puede valer vacío; «minecraft:glass» no acaba en «_glass».
    assert_eq!(r.resolver("minecraft:_glass", false), Resolucion::Nada);
    assert!(matches!(
        r.resolver("minecraft:glass", false),
        Resolucion::Items(_)
    ));
}

#[test]
fn la_resolucion_se_cachea_por_estado() {
    // Mismo estado en dos regiones y repetido: se cuenta bien una sola resolución.
    let s = schem(vec![
        fila(
            "a",
            [0, 0, 0],
            &[("minecraft:wall_torch[facing=north]", 1000)],
        ),
        fila(
            "b",
            [0, 1, 0],
            &[("minecraft:wall_torch[facing=north]", 24)],
        ),
    ]);
    let l = calcular(&s, &Opciones::default()).unwrap();
    assert_eq!(
        l.materiales,
        vec![desglosar("minecraft:torch".into(), 1024, 64)]
    );
}

// ---- Desglose y totales ----

fn desglose(cantidad: u64, t: u32) -> (u64, u64, u64, u64, u64) {
    let m = desglosar("x".into(), cantidad, t);
    assert_eq!(m.stacks * u64::from(t) + m.sueltos, cantidad);
    assert_eq!(
        (m.shulkers * 27 + m.stacks_en_shulker) * u64::from(t) + m.sueltos_en_shulker,
        cantidad
    );
    (
        m.stacks,
        m.sueltos,
        m.shulkers,
        m.stacks_en_shulker,
        m.sueltos_en_shulker,
    )
}

#[test]
fn desglose_en_stacks_y_shulkers() {
    assert_eq!(desglose(3456, 64), (54, 0, 2, 0, 0));
    assert_eq!(desglose(1728, 64), (27, 0, 1, 0, 0));
    assert_eq!(desglose(100, 64), (1, 36, 0, 1, 36));
    assert_eq!(desglose(50, 16), (3, 2, 0, 3, 2));
    assert_eq!(desglose(30, 1), (30, 0, 1, 3, 0));
    assert_eq!(desglose(0, 64), (0, 0, 0, 0, 0));
}

#[test]
fn tamanos_de_stack_del_json() {
    let r = reglas();
    assert_eq!(r.tamano_stack("minecraft:stone"), 64);
    assert_eq!(r.tamano_stack("minecraft:oak_sign"), 16);
    assert_eq!(r.tamano_stack("minecraft:water_bucket"), 1);
}

#[test]
fn orden_y_totales() {
    let l = lista(
        &[
            ("minecraft:stone", 100),
            ("minecraft:oak_wall_sign[facing=east,waterlogged=false]", 50),
            ("minecraft:dirt", 100),
            (
                "minecraft:white_bed[facing=north,occupied=false,part=head]",
                3,
            ),
        ],
        &Opciones::default(),
    );
    let orden: Vec<&str> = l.materiales.iter().map(|m| m.item.as_str()).collect();
    assert_eq!(
        orden,
        [
            "minecraft:dirt",
            "minecraft:stone",
            "minecraft:oak_sign",
            "minecraft:white_bed"
        ]
    );
    assert_eq!(l.total_items, 253);
    // Huecos: 2 (dirt) + 2 (stone) + 4 (carteles: 3 st + 2) + 3 (camas) = 11.
    assert_eq!(l.total_stacks, 11);
    assert_eq!(l.total_shulkers, 1);
    assert_eq!(l.desconocidos, 0);
}

#[test]
fn forma_json_del_resultado() {
    let l = lista(&[("minecraft:stone", 1)], &Opciones::default());
    let v = serde_json::to_value(&l).unwrap();
    assert_eq!(v["totalItems"], 1);
    assert_eq!(v["materiales"][0]["tamanoStack"], 64);
    assert_eq!(v["materiales"][0]["stacksEnShulker"], 0);
    let o: Opciones =
        serde_json::from_str(r#"{"incluirFluidos":true,"region":null,"yMin":-3}"#).unwrap();
    assert!(o.incluir_fluidos);
    assert_eq!(o.y_min, Some(-3));
    assert_eq!(o.y_max, None);
}
