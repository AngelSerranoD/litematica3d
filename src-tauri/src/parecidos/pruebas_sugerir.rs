// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Pruebas de familias, parecidos, familias parecidas y plan de cambio de
//! familia. Todas con el catálogo integrado (sin jar).

use std::sync::OnceLock;

use super::familias::{Variante, base_otro};
use super::*;
use crate::edicion::{Reemplazo, aplicar};
use crate::schematic::modelo::caja_englobante;
use crate::schematic::{Dimensiones, ExtrasRegion, Formato, Region, Schematic};

fn cat() -> &'static Catalogo {
    static C: OnceLock<Catalogo> = OnceLock::new();
    C.get_or_init(Catalogo::integrado)
}

fn fam(id: &str) -> (String, String) {
    let p = familia_de(id);
    (p.familia, p.variante.nombre())
}

fn props(estado: &str) -> Vec<(String, String)> {
    let (_, p) = separar(estado);
    p.into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn prop<'a>(estado: &'a str, k: &str) -> Option<&'a str> {
    crate::malla::apariencia::propiedad(estado, k)
}

#[test]
fn familias_y_variantes() {
    let casos: &[(&str, &str, &str)] = &[
        ("minecraft:deepslate", "deepslate", "base"),
        ("minecraft:cobbled_deepslate_wall", "deepslate", "adoquín"),
        ("minecraft:polished_deepslate_stairs", "deepslate", "pulido"),
        (
            "minecraft:deepslate_brick_stairs[facing=east]",
            "deepslate",
            "ladrillo",
        ),
        (
            "minecraft:cracked_deepslate_tiles",
            "deepslate",
            "agrietado+baldosa",
        ),
        ("minecraft:chiseled_deepslate", "deepslate", "cincelado"),
        ("minecraft:blackstone_slab", "blackstone", "base"),
        (
            "minecraft:polished_blackstone_brick_stairs",
            "blackstone",
            "pulido+ladrillo",
        ),
        (
            "minecraft:cracked_polished_blackstone_bricks",
            "blackstone",
            "agrietado+pulido+ladrillo",
        ),
        (
            "minecraft:polished_blackstone_button",
            "blackstone",
            "pulido",
        ),
        ("minecraft:gilded_blackstone", "blackstone", "dorado"),
        // Ladrillo de arcilla, del Nether y rojo del Nether: familias propias.
        ("minecraft:bricks", "bricks", "ladrillo"),
        ("minecraft:brick_stairs", "bricks", "ladrillo"),
        ("minecraft:nether_brick_fence", "nether_bricks", "ladrillo"),
        (
            "minecraft:chiseled_nether_bricks",
            "nether_bricks",
            "cincelado+ladrillo",
        ),
        (
            "minecraft:red_nether_brick_wall",
            "red_nether_bricks",
            "ladrillo",
        ),
        // Piedra: adoquín, ladrillos, lisa, musgosa.
        ("minecraft:stone", "stone", "base"),
        ("minecraft:cobblestone", "stone", "adoquín"),
        (
            "minecraft:mossy_cobblestone_slab",
            "stone",
            "musgoso+adoquín",
        ),
        (
            "minecraft:mossy_stone_brick_wall",
            "stone",
            "musgoso+ladrillo",
        ),
        ("minecraft:smooth_stone_slab", "stone", "liso"),
        ("minecraft:stone_button", "stone", "base"),
        ("minecraft:end_stone_bricks", "end_stone", "ladrillo"),
        // Cuarzo, arenisca, prismarina, purpur.
        ("minecraft:quartz_block", "quartz", "base"),
        ("minecraft:chiseled_quartz_block", "quartz", "cincelado"),
        ("minecraft:quartz_pillar", "quartz", "pilar"),
        ("minecraft:smooth_quartz_stairs", "quartz", "liso"),
        ("minecraft:quartz_bricks", "quartz", "ladrillo"),
        ("minecraft:nether_quartz_ore", "nether_quartz_ore", "base"),
        ("minecraft:cut_sandstone_slab", "sandstone", "cortado"),
        ("minecraft:smooth_red_sandstone", "red_sandstone", "liso"),
        ("minecraft:dark_prismarine_stairs", "prismarine", "oscuro"),
        ("minecraft:prismarine_brick_slab", "prismarine", "ladrillo"),
        ("minecraft:purpur_block", "purpur", "base"),
        ("minecraft:purpur_pillar", "purpur", "pilar"),
        // Cobre por oxidación, con la cera como variante.
        (
            "minecraft:waxed_exposed_cut_copper_slab",
            "exposed_copper",
            "encerado+cortado",
        ),
        ("minecraft:copper_block", "copper", "base"),
        ("minecraft:copper_ore", "copper_ore", "base"),
        // Maderas por especie.
        ("minecraft:oak_planks", "oak", "tablones"),
        ("minecraft:oak_stairs", "oak", "tablones"),
        ("minecraft:oak_log", "oak", "tronco"),
        ("minecraft:stripped_oak_wood", "oak", "descortezado+madera"),
        ("minecraft:dark_oak_fence_gate", "dark_oak", "tablones"),
        ("minecraft:pale_oak_door", "pale_oak", "tablones"),
        ("minecraft:crimson_hyphae", "crimson", "madera"),
        (
            "minecraft:stripped_bamboo_block",
            "bamboo",
            "descortezado+tronco",
        ),
        ("minecraft:bamboo_mosaic_slab", "bamboo", "mosaico"),
        ("minecraft:bamboo", "bamboo_plant", "base"),
        ("minecraft:oak_leaves", "oak_leaves", "base"),
        // Alfombras con su lana; paneles con su cristal; placas con peso.
        ("minecraft:white_carpet", "white_wool", "base"),
        ("minecraft:moss_carpet", "moss", "base"),
        (
            "minecraft:red_stained_glass_pane",
            "red_stained_glass",
            "base",
        ),
        ("minecraft:iron_bars", "iron", "base"),
        ("minecraft:heavy_weighted_pressure_plate", "iron", "base"),
        ("minecraft:light_weighted_pressure_plate", "gold", "base"),
        ("minecraft:tuff_brick_wall", "tuff", "ladrillo"),
        ("minecraft:mud_bricks", "mud", "ladrillo"),
        ("otro_mod:polished_granito", "otro_mod:granito", "pulido"),
    ];
    for (id, familia, variante) in casos {
        assert_eq!(fam(id), (familia.to_string(), variante.to_string()), "{id}");
    }
}

#[test]
fn coste_de_variantes() {
    let v = |id: &str| familia_de(id).variante;
    let ladrillo = v("minecraft:deepslate_bricks");
    // `ladrillo` → `pulido+ladrillo` mejor que → base o → `pulido`.
    let pl = ladrillo.coste(&v("minecraft:polished_blackstone_bricks"));
    assert!(pl < ladrillo.coste(&Variante::default()));
    assert!(pl < ladrillo.coste(&v("minecraft:polished_blackstone")));
    // Y al revés: `pulido+ladrillo` → `ladrillo` mejor que → `pulido`.
    let pbl = v("minecraft:polished_blackstone_bricks");
    assert!(pbl.coste(&ladrillo) < pbl.coste(&v("minecraft:polished_deepslate")));
    // `adoquín` → base mejor que → `pulido`.
    let ad = v("minecraft:cobbled_deepslate");
    assert!(ad.coste(&Variante::default()) < ad.coste(&v("minecraft:polished_blackstone")));
    // La exacta es la mejor.
    assert!(ladrillo.coste(&ladrillo) < pl);
    assert_eq!(ladrillo.parecido(&ladrillo), 1.0);
}

#[test]
fn base_de_los_otros() {
    assert_eq!(base_otro("minecraft:white_candle"), "candle");
    assert_eq!(base_otro("minecraft:light_blue_candle[lit=true]"), "candle");
    assert_eq!(base_otro("minecraft:soul_torch"), "torch");
    assert_eq!(base_otro("minecraft:torch"), "torch");
    assert_eq!(base_otro("minecraft:dark_oak_sapling"), "sapling");
    assert_eq!(
        base_otro("minecraft:waxed_oxidized_copper_bulb"),
        "copper_bulb"
    );
}

#[test]
fn escalera_de_deepslate_sugiere_escaleras_de_blackstone() {
    let origen =
        "minecraft:deepslate_brick_stairs[facing=east,half=top,shape=straight,waterlogged=false]";
    let todos = cat().parecidos(origen, 500);
    assert!(!todos.is_empty());
    assert!(todos.iter().all(|p| p.forma == FormaBloque::Escalera));
    assert!(
        todos
            .iter()
            .all(|p| p.id != "minecraft:deepslate_brick_stairs")
    );
    assert!(todos.windows(2).all(|w| w[0].distancia <= w[1].distancia));
    let negra = todos
        .iter()
        .find(|p| p.familia == "blackstone")
        .expect("alguna escalera de blackstone");
    assert_eq!(
        prop(&negra.estado, "facing"),
        Some("east"),
        "{}",
        negra.estado
    );
    assert_eq!(prop(&negra.estado, "half"), Some("top"));
    // Todos conservan las propiedades del origen.
    for p in &todos {
        assert_eq!(props(&p.estado), props(origen), "{}", p.estado);
    }
    // Y en el plan de familia, la equivalente es la de ladrillo pulido.
    assert_eq!(
        cat().equivalente(origen, "blackstone").as_deref(),
        Some(
            "minecraft:polished_blackstone_brick_stairs[facing=east,half=top,shape=straight,waterlogged=false]"
        )
    );
    // `n` limita.
    assert_eq!(cat().parecidos(origen, 3).len(), 3);
}

#[test]
fn una_losa_nunca_sugiere_un_bloque_entero() {
    for losa in cat().bloques().filter(|b| b.forma == FormaBloque::Losa) {
        let estado = losa.estado_con(&[("type", "top")]);
        for p in cat().parecidos(&estado, 1000) {
            assert_eq!(p.forma, FormaBloque::Losa, "{} → {}", losa.id, p.id);
            assert!(p.id.ends_with("_slab"), "{} → {}", losa.id, p.id);
            assert_eq!(prop(&p.estado, "type"), Some("top"));
        }
    }
    // Y el equivalente de una losa en otra familia sin losas es una losa.
    let e = cat()
        .equivalente(
            "minecraft:stone_slab[type=double,waterlogged=false]",
            "basalt",
        )
        .unwrap();
    assert!(e.contains("_slab[type=double"), "{e}");
}

#[test]
fn sin_jar_con_colores_planos() {
    let c = cat();
    assert!(!c.con_jar());
    let p = c.parecidos("minecraft:stone", 5);
    assert_eq!(p.len(), 5);
    for x in &p {
        assert_eq!(x.forma, FormaBloque::Entero);
        assert!(es_sugerible(&x.id));
        assert_eq!(Some(x.color), c.bloque(&x.id).unwrap().color);
    }
    // Lo no sugerible no aparece nunca (cofres, infestados…).
    let todos = c.parecidos("minecraft:stone", 2000);
    assert!(todos.iter().all(|x| !x.id.contains("infested")));
    assert!(todos.iter().all(|x| x.id != "minecraft:chest"));
    // Un bloque desconocido no tiene parecidos.
    assert!(c.parecidos("otro_mod:cosa", 5).is_empty());
    // Los troncos conservan el eje también sin jar.
    let t = c.parecidos("minecraft:oak_log[axis=x]", 2000);
    let abeto = t.iter().find(|x| x.id == "minecraft:spruce_log").unwrap();
    assert_eq!(abeto.estado, "minecraft:spruce_log[axis=x]");
}

#[test]
fn otro_solo_sugiere_su_mismo_id_base() {
    let v = cat().parecidos(
        "minecraft:white_candle[candles=1,lit=false,waterlogged=false]",
        50,
    );
    assert!(!v.is_empty());
    assert!(
        v.iter()
            .all(|p| p.id.ends_with("_candle") || p.id == "minecraft:candle"),
        "{v:?}"
    );
    let t = cat().parecidos("minecraft:torch", 50);
    assert!(t.iter().all(|p| p.id == "minecraft:soul_torch"), "{t:?}");
}

#[test]
fn deepslate_se_parece_a_blackstone() {
    let v = cat().familias_parecidas("deepslate", 5);
    let nombres: Vec<&str> = v.iter().map(|f| f.familia.as_str()).collect();
    assert!(nombres.contains(&"blackstone"), "{nombres:?}");
    assert!(!nombres.contains(&"deepslate"));
    assert!(v.windows(2).all(|w| w[0].puntuacion <= w[1].puntuacion));
    assert!(v.iter().all(|f| f.miembros.iter().all(|m| m.sugerible)));
    assert!(cat().familias_parecidas("no_existe", 5).is_empty());
}

#[test]
fn familia_de_un_bloque() {
    let f = cat().familia_bloque("minecraft:polished_blackstone_brick_stairs[facing=east]");
    assert_eq!(f.familia, "blackstone");
    assert_eq!(f.nombre_variante, "pulido+ladrillo");
    assert_eq!(f.forma, FormaBloque::Escalera);
    assert!(f.sugerible);
    let v = serde_json::to_value(&f).unwrap();
    assert_eq!(v["variante"], serde_json::json!(["pulido", "ladrillo"]));
    assert_eq!(v["nombreVariante"], "pulido+ladrillo");
    assert_eq!(v["forma"], "escalera");
    let miembros = &cat().familias()["deepslate"];
    assert!(
        miembros
            .iter()
            .any(|m| m.id == "minecraft:cobbled_deepslate_wall")
    );
}

/// Schematic de una región 4×1×1 con `estados` (uno por posición).
fn schem(estados: &[&str]) -> Schematic {
    let mut paleta = vec!["minecraft:air".to_string()];
    let bloques: Vec<u32> = estados
        .iter()
        .map(|e| match paleta.iter().position(|p| p == e) {
            Some(i) => i as u32,
            None => {
                paleta.push(e.to_string());
                (paleta.len() - 1) as u32
            }
        })
        .collect();
    let regiones = vec![Region {
        nombre: "r".into(),
        origen: [0, 0, 0],
        tamano: Dimensiones {
            ancho: estados.len() as u32,
            alto: 1,
            largo: 1,
        },
        paleta,
        bloques,
        extras: ExtrasRegion::default(),
    }];
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

#[test]
fn plan_de_familia_sobre_un_schematic() {
    let escalera =
        "minecraft:deepslate_brick_stairs[facing=east,half=top,shape=straight,waterlogged=false]";
    let mut s = schem(&[
        "minecraft:deepslate",
        "minecraft:cobbled_deepslate",
        escalera,
        escalera,
        "minecraft:polished_deepslate_slab[type=bottom,waterlogged=false]",
        "minecraft:deepslate_tile_wall[east=none,north=low,south=none,up=true,waterlogged=false,west=none]",
        "minecraft:chiseled_deepslate",
        "minecraft:stone",
        "minecraft:air",
    ]);

    let presentes = miembros_en_schematic(&s, "deepslate");
    assert_eq!(presentes.len(), 6);
    assert_eq!(
        presentes
            .iter()
            .find(|p| p.estado == escalera)
            .unwrap()
            .cantidad,
        2
    );
    assert!(presentes.iter().all(|p| p.estado != "minecraft:stone"));

    let plan = cat().plan_familia(&s, "deepslate", "blackstone");
    let destino = |o: &str| {
        plan.iter()
            .find(|r| r.estado_origen == o)
            .map(|r| r.estado_destino.as_str())
            .unwrap_or_else(|| panic!("falta {o} en {plan:?}"))
    };
    assert_eq!(plan.len(), 6, "{plan:?}");
    assert_eq!(destino("minecraft:deepslate"), "minecraft:blackstone");
    assert_eq!(
        destino("minecraft:cobbled_deepslate"),
        "minecraft:blackstone"
    );
    assert_eq!(
        destino(escalera),
        "minecraft:polished_blackstone_brick_stairs[facing=east,half=top,shape=straight,waterlogged=false]"
    );
    assert_eq!(
        destino("minecraft:polished_deepslate_slab[type=bottom,waterlogged=false]"),
        "minecraft:polished_blackstone_slab[type=bottom,waterlogged=false]"
    );
    let muro = destino(
        "minecraft:deepslate_tile_wall[east=none,north=low,south=none,up=true,waterlogged=false,west=none]",
    );
    assert!(
        muro.starts_with("minecraft:polished_blackstone_brick_wall["),
        "{muro}"
    );
    assert_eq!(prop(muro, "north"), Some("low"));
    assert_eq!(
        destino("minecraft:chiseled_deepslate"),
        "minecraft:chiseled_polished_blackstone"
    );
    // Todas las formas se conservan.
    for r in &plan {
        assert_eq!(
            cat().forma(&r.estado_origen),
            cat().forma(&r.estado_destino),
            "{r:?}"
        );
    }

    // El plan se aplica tal cual con el modelo de cambios.
    let cambiadas = aplicar(&mut s, &[], &plan).unwrap();
    assert_eq!(cambiadas, 7);
    assert!(miembros_en_schematic(&s, "deepslate").is_empty());
    assert_eq!(miembros_en_schematic(&s, "blackstone").len(), 5);

    // Familia origen ausente o igual a la destino: plan vacío.
    assert!(cat().plan_familia(&s, "deepslate", "blackstone").is_empty());
    assert!(
        cat()
            .plan_familia(&s, "blackstone", "blackstone")
            .is_empty()
    );
}

#[test]
fn plan_sin_la_forma_en_la_familia_destino_usa_todo_el_catalogo() {
    // `basalt` no tiene escaleras: se usa la escalera más parecida en color.
    let s = schem(&[
        "minecraft:stone_stairs[facing=north,half=bottom,shape=straight,waterlogged=false]",
    ]);
    let plan = cat().plan_familia(&s, "stone", "basalt");
    assert_eq!(plan.len(), 1);
    let d = &plan[0].estado_destino;
    assert_eq!(cat().forma(d), FormaBloque::Escalera, "{d}");
    let mejor = cat().parecidos(&plan[0].estado_origen, 1);
    assert_eq!(*d, mejor[0].estado);
    let _: &Reemplazo = &plan[0];
}
