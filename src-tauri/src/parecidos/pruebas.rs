// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Pruebas de bloques parecidos: color (jar sintético y tabla integrada),
//! OKLab, caché, formas, propiedades y exclusiones. Ninguna necesita un jar
//! real salvo las marcadas `#[ignore]`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::json;

use super::color::{self, CacheColores, MediaTextura, Oklab};
use super::forma::{FormaBloque, forma_por_sufijo, forma_sin_jar};
use super::propiedades;
use super::*;
use crate::texturas::pruebas::png_de;
use crate::texturas::versiones::VersionMinecraft;
use crate::texturas::{self, Activo};

// ------------------------------------------------------------ jar sintético

const CUBE: &str = r##"{ "elements": [ { "from": [0,0,0], "to": [16,16,16], "faces": {
    "down":  { "texture": "#down" }, "up":    { "texture": "#up" },
    "north": { "texture": "#north" }, "south": { "texture": "#south" },
    "west":  { "texture": "#west" }, "east":  { "texture": "#east" } } } ] }"##;
const CUBE_ALL: &str = r##"{ "parent": "minecraft:block/cube", "textures": {
    "particle": "#all", "down": "#all", "up": "#all", "north": "#all",
    "south": "#all", "west": "#all", "east": "#all" } }"##;
const CUBE_COLUMN: &str = r##"{ "parent": "block/cube", "textures": {
    "particle": "#side", "down": "#end", "up": "#end", "north": "#side",
    "south": "#side", "west": "#side", "east": "#side" } }"##;
const SLAB: &str = r##"{ "textures": { "bottom": "block/stone", "top": "block/stone", "side": "block/stone" },
  "elements": [ { "from": [0,0,0], "to": [16,8,16], "faces": {
    "down": { "texture": "#bottom" }, "up": { "texture": "#top" },
    "north": { "texture": "#side" }, "south": { "texture": "#side" },
    "west": { "texture": "#side" }, "east": { "texture": "#side" } } } ] }"##;
const FAROL: &str = r##"{ "textures": { "all": "block/lantern" },
  "elements": [ { "from": [5,0,5], "to": [11,7,11], "faces": {
    "up": { "texture": "#all" }, "north": { "texture": "#all" } } } ] }"##;
const COFRE: &str = r##"{ "textures": { "particle": "block/stone" } }"##;

fn modelo_cube_all(textura: &str) -> String {
    format!(
        r#"{{ "parent": "minecraft:block/cube_all", "textures": {{ "all": "minecraft:block/{textura}" }} }}"#
    )
}

/// Jar mínimo con blockstates, modelos y texturas de colores conocidos.
fn jar_sintetico(ruta: &Path) {
    let f = std::fs::File::create(ruta).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let o =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut poner = |nombre: &str, bytes: &[u8]| {
        z.start_file(nombre, o).unwrap();
        z.write_all(bytes).unwrap();
    };
    let bs = |n: &str| format!("assets/minecraft/blockstates/{n}.json");
    let md = |n: &str| format!("assets/minecraft/models/block/{n}.json");
    let tx = |n: &str| format!("assets/minecraft/textures/block/{n}.png");

    poner(
        &bs("stone"),
        br#"{ "variants": { "": { "model": "minecraft:block/stone" } } }"#,
    );
    poner(
        &bs("columna"),
        br#"{ "variants": {
            "axis=y": { "model": "minecraft:block/columna" },
            "axis=z": { "model": "minecraft:block/columna", "x": 90 },
            "axis=x": { "model": "minecraft:block/columna", "x": 90, "y": 90 } } }"#,
    );
    poner(
        &bs("lampara"),
        br#"{ "variants": {
            "lit=false": { "model": "minecraft:block/lampara_off" },
            "lit=true": { "model": "minecraft:block/lampara_on" } } }"#,
    );
    poner(
        &bs("prueba_slab"),
        br#"{ "variants": {
            "type=bottom": { "model": "minecraft:block/prueba_slab" },
            "type=top": { "model": "minecraft:block/prueba_slab", "x": 180 },
            "type=double": { "model": "minecraft:block/stone" } } }"#,
    );
    poner(
        &bs("lantern"),
        br#"{ "variants": { "hanging=false": { "model": "minecraft:block/lantern" },
                            "hanging=true": { "model": "minecraft:block/lantern" } } }"#,
    );
    poner(
        &bs("chest"),
        br#"{ "variants": { "facing=north": { "model": "minecraft:block/chest" } } }"#,
    );

    poner(&md("cube"), CUBE.as_bytes());
    poner(&md("cube_all"), CUBE_ALL.as_bytes());
    poner(&md("cube_column"), CUBE_COLUMN.as_bytes());
    poner(&md("stone"), modelo_cube_all("stone").as_bytes());
    poner(
        &md("lampara_off"),
        modelo_cube_all("lampara_off").as_bytes(),
    );
    poner(&md("lampara_on"), modelo_cube_all("lampara_on").as_bytes());
    poner(
        &md("columna"),
        br#"{ "parent": "minecraft:block/cube_column", "textures": {
            "end": "minecraft:block/columna_top", "side": "minecraft:block/columna_side" } }"#,
    );
    poner(&md("prueba_slab"), SLAB.as_bytes());
    poner(&md("lantern"), FAROL.as_bytes());
    poner(&md("chest"), COFRE.as_bytes());

    let liso = |c: [u8; 4]| png_de(16, 16, move |_, _| c);
    poner(&tx("stone"), &liso([120, 120, 120, 255]));
    poner(&tx("columna_top"), &liso([255, 0, 0, 255]));
    // Lado: mitad de arriba azul opaca; abajo, blanco transparente (alfa 0)
    // a la izquierda y verde casi transparente (alfa 50) a la derecha.
    poner(
        &tx("columna_side"),
        &png_de(16, 16, |x, y| {
            if y < 8 {
                [0, 0, 255, 255]
            } else if x < 8 {
                [255, 255, 255, 0]
            } else {
                [0, 255, 0, 50]
            }
        }),
    );
    poner(&tx("lampara_off"), &liso([20, 20, 20, 255]));
    poner(&tx("lampara_on"), &liso([250, 210, 60, 255]));
    poner(&tx("lantern"), &liso([60, 60, 70, 255]));
    z.finish().unwrap();
}

/// Carpeta temporal con un jar sintético y su atlas generado.
struct Entorno {
    _dir: tempfile::TempDir,
    cache: PathBuf,
    activo: Arc<Activo>,
}

fn entorno() -> Entorno {
    let dir = tempfile::tempdir().unwrap();
    let jar = dir.path().join("prueba.jar");
    jar_sintetico(&jar);
    let v = VersionMinecraft {
        id: "prueba-1".into(),
        ruta: jar.to_string_lossy().into_owned(),
        es_release: true,
        fecha: String::new(),
    };
    let cache = dir.path().join("cache").join("texturas");
    let activo = Arc::new(texturas::cargar_o_generar(&cache, &v).unwrap());
    Entorno {
        _dir: dir,
        cache,
        activo,
    }
}

// ------------------------------------------------------------------- color

#[test]
fn media_de_textura_ignora_transparentes() {
    let mut px = Vec::new();
    px.extend_from_slice(&[200, 0, 0, 255]);
    px.extend_from_slice(&[0, 0, 200, 255]);
    px.extend_from_slice(&[0, 255, 0, 0]);
    px.extend_from_slice(&[0, 255, 0, 127]);
    let m = color::media_textura(&px);
    assert_eq!(
        m,
        MediaTextura {
            rgb: [100, 0, 100],
            n: 2
        }
    );
    assert_eq!(color::media_textura(&[9, 9, 9, 10]).n, 0);
}

#[test]
fn color_desde_jar_sintetico() {
    let e = entorno();
    let c = Catalogo::con_activo(Arc::clone(&e.activo), &e.cache);
    assert!(c.con_jar());
    assert_eq!(c.version(), Some("prueba-1"));
    assert_eq!(c.len(), 6);
    assert_eq!(
        c.bloque("minecraft:stone").unwrap().color,
        Some([120, 120, 120])
    );
    // Columna de pie: arriba roja (peso 2, 256 px) y cuatro lados con solo
    // 128 px azules opacos cada uno; ni el blanco de alfa 0 ni el verde de
    // alfa 50 cuentan → mitad rojo, mitad azul.
    let col = c.bloque("minecraft:columna").unwrap();
    assert_eq!(col.color, Some([128, 0, 128]));
    assert_eq!(
        c.color_estado("minecraft:columna[axis=y]"),
        Some([128, 0, 128])
    );
    // El color depende del estado.
    assert_eq!(c.color_estado("minecraft:lampara"), Some([20, 20, 20]));
    assert_eq!(
        c.color_estado("minecraft:lampara[lit=true]"),
        Some([250, 210, 60])
    );
    assert_eq!(
        c.bloque("minecraft:lampara").unwrap().estado_por_defecto(),
        "minecraft:lampara[lit=false]"
    );
    // Sin elementos: la textura de partícula.
    assert_eq!(
        c.color_estado("minecraft:chest[facing=north]"),
        Some([120, 120, 120])
    );
    // Bloques de mods: no se conocen.
    assert_eq!(c.color_estado("otro:piedra"), None);
    assert!(c.oklab_estado("minecraft:stone").is_some());
}

#[test]
fn forma_con_jar_mira_los_modelos() {
    let e = entorno();
    let c = Catalogo::con_activo(Arc::clone(&e.activo), &e.cache);
    assert_eq!(c.forma("minecraft:stone"), FormaBloque::Entero);
    assert_eq!(c.forma("minecraft:columna[axis=x]"), FormaBloque::Entero);
    assert_eq!(c.forma("minecraft:prueba_slab"), FormaBloque::Losa);
    assert_eq!(c.forma("minecraft:lantern"), FormaBloque::Otro);
    assert_eq!(c.forma("minecraft:chest"), FormaBloque::Otro);
    assert_eq!(c.forma("otro:piedra"), FormaBloque::Otro);
    assert!(!c.bloque("minecraft:chest").unwrap().sugerible);
    let losa = c.bloque("minecraft:prueba_slab").unwrap();
    assert_eq!(losa.propiedad("type").unwrap().defecto, "bottom");
    assert_eq!(
        losa.propiedad("waterlogged").unwrap().valores,
        ["false", "true"]
    );
    assert_eq!(
        losa.estado_desde("minecraft:oak_slab[type=top,waterlogged=true]"),
        "minecraft:prueba_slab[type=top,waterlogged=true]"
    );
}

#[test]
fn cache_escrita_y_releida() {
    let e = entorno();
    let ruta = color::ruta_cache(&e.cache, "prueba-1");
    assert!(ruta.ends_with("texturas/prueba-1/colores.json"));
    assert!(!ruta.exists());
    let _ = Catalogo::con_activo(Arc::clone(&e.activo), &e.cache);
    let mut guardada: CacheColores =
        serde_json::from_slice(&std::fs::read(&ruta).unwrap()).unwrap();
    assert_eq!(guardada.esquema, color::ESQUEMA_CACHE);
    assert_eq!(guardada.umbral_alfa, color::UMBRAL_ALFA);
    assert_eq!(
        guardada.texturas["block/columna_side"],
        MediaTextura {
            rgb: [0, 0, 255],
            n: 128
        }
    );
    assert_eq!(guardada.bloques["minecraft:stone"], [120, 120, 120]);

    // Se relee: un valor cambiado a mano en la caché es el que sale.
    guardada.bloques.insert("minecraft:stone".into(), [1, 2, 3]);
    guardada.texturas.insert(
        "block/stone".into(),
        MediaTextura {
            rgb: [1, 2, 3],
            n: 256,
        },
    );
    std::fs::write(&ruta, serde_json::to_vec(&guardada).unwrap()).unwrap();
    let c = Catalogo::con_activo(Arc::clone(&e.activo), &e.cache);
    assert_eq!(c.bloque("minecraft:stone").unwrap().color, Some([1, 2, 3]));
    assert_eq!(c.color_estado("minecraft:stone"), Some([1, 2, 3]));

    // Con otro esquema (o dañada) se recalcula y se reescribe.
    guardada.esquema = 999;
    std::fs::write(&ruta, serde_json::to_vec(&guardada).unwrap()).unwrap();
    let c = Catalogo::con_activo(Arc::clone(&e.activo), &e.cache);
    assert_eq!(c.color_estado("minecraft:stone"), Some([120, 120, 120]));
    std::fs::write(&ruta, b"{ roto").unwrap();
    let _ = Catalogo::con_activo(Arc::clone(&e.activo), &e.cache);
    let releida: CacheColores = serde_json::from_slice(&std::fs::read(&ruta).unwrap()).unwrap();
    assert_eq!(releida.bloques["minecraft:stone"], [120, 120, 120]);
}

#[test]
fn cache_de_catalogo_se_rehace_al_cambiar_de_atlas() {
    let e = entorno();
    let cc = CacheCatalogo::default();
    let sin = cc.obtener(None, &e.cache);
    assert!(!sin.con_jar());
    assert!(Arc::ptr_eq(&sin, &cc.obtener(None, &e.cache)));
    let con = cc.obtener(Some(Arc::clone(&e.activo)), &e.cache);
    assert!(con.con_jar());
    assert!(Arc::ptr_eq(
        &con,
        &cc.obtener(Some(Arc::clone(&e.activo)), &e.cache)
    ));
}

#[test]
fn color_sin_jar_desde_la_tabla_integrada() {
    let c = Catalogo::integrado();
    assert!(!c.con_jar());
    assert_eq!(c.version(), None);
    assert!(c.len() > 900, "{}", c.len());
    let tablones = c.bloque("minecraft:acacia_planks").unwrap();
    assert_eq!(tablones.color, Some([168, 90, 50]));
    assert_eq!(tablones.forma, FormaBloque::Entero);
    assert_eq!(
        c.color_estado("minecraft:acacia_stairs[facing=east,half=top]"),
        Some([168, 90, 50])
    );
    assert_eq!(c.color_estado("minecraft:no_existe"), None);
    assert_eq!(c.color_estado("mod:piedra"), None);
    assert!(!c.bloque("minecraft:air").is_some_and(|b| b.sugerible));
}

#[test]
fn estado_destino_conserva_propiedades() {
    let c = Catalogo::integrado();
    let origen =
        "minecraft:stone_brick_stairs[facing=east,half=top,shape=outer_left,waterlogged=true]";
    let escalera = c.bloque("minecraft:acacia_stairs").unwrap();
    assert_eq!(
        escalera.estado_desde(origen),
        "minecraft:acacia_stairs[facing=east,half=top,shape=outer_left,waterlogged=true]"
    );
    assert_eq!(
        escalera.estado_por_defecto(),
        "minecraft:acacia_stairs[facing=north,half=bottom,shape=straight,waterlogged=false]"
    );
    // Un valor que el destino no admite cae a su defecto.
    assert_eq!(
        escalera.estado_con(&[("facing", "up"), ("color", "rojo")]),
        "minecraft:acacia_stairs[facing=north,half=bottom,shape=straight,waterlogged=false]"
    );
    let muro = c.bloque("minecraft:cobblestone_wall").unwrap();
    assert_eq!(muro.color, c.bloque("minecraft:cobblestone").unwrap().color);
    assert_eq!(
        c.bloque("minecraft:stone_brick_wall").unwrap().color,
        c.bloque("minecraft:stone_bricks").unwrap().color
    );
    let n_muros = c.bloques().filter(|b| b.forma == FormaBloque::Muro).count();
    assert_eq!(n_muros, MUROS_VANILLA.len());
    assert_eq!(
        muro.estado_por_defecto(),
        "minecraft:cobblestone_wall[east=none,north=none,south=none,up=true,waterlogged=false,west=none]"
    );
    assert_eq!(
        c.bloque("minecraft:stone").unwrap().estado_por_defecto(),
        "minecraft:stone"
    );
}

// ------------------------------------------------------------------- OKLab

#[test]
fn distancia_oklab_coherente() {
    let gris = [128, 128, 128];
    let cerca = distancia_rgb(gris, [136, 136, 136]);
    let rojo = distancia_rgb(gris, [200, 30, 30]);
    assert!(cerca < rojo, "{cerca} {rojo}");
    assert!(cerca > 0.0 && cerca < 0.05, "{cerca}");
    assert_eq!(distancia_rgb(gris, gris), 0.0);
    let blanco = Oklab::desde_rgb([255, 255, 255]);
    let negro = Oklab::desde_rgb([0, 0, 0]);
    assert!((blanco.l - 1.0).abs() < 1e-3 && blanco.a.abs() < 1e-3 && blanco.b.abs() < 1e-3);
    assert!(negro.l.abs() < 1e-9);
    let g = Oklab::desde_rgb(gris);
    assert!(g.a.abs() < 1e-3 && g.b.abs() < 1e-3);
    // Simétrica.
    assert_eq!(
        distancia_rgb([10, 200, 30], [90, 20, 250]),
        distancia_rgb([90, 20, 250], [10, 200, 30])
    );
    // Rojo puro de referencia (Ottosson): L ≈ 0,628, a ≈ 0,225, b ≈ 0,126.
    let r = Oklab::desde_rgb([255, 0, 0]);
    assert!((r.l - 0.628).abs() < 2e-3 && (r.a - 0.225).abs() < 2e-3 && (r.b - 0.126).abs() < 2e-3);
}

// ------------------------------------------------------------------ formas

#[test]
fn formas_de_ids_reales() {
    use FormaBloque::*;
    let casos = [
        ("deepslate_brick_stairs", Escalera),
        ("polished_blackstone_slab", Losa),
        ("cobblestone_wall", Muro),
        ("oak_fence_gate", PuertaValla),
        ("nether_brick_fence", Valla),
        ("oak_door", Puerta),
        ("iron_trapdoor", Trampilla),
        ("stone_button", Boton),
        ("heavy_weighted_pressure_plate", PlacaPresion),
        ("oak_pressure_plate", PlacaPresion),
        ("glass_pane", Panel),
        ("red_stained_glass_pane", Panel),
        ("iron_bars", Panel),
        ("white_carpet", Alfombra),
        ("stone", Entero),
        ("oak_planks", Entero),
        ("oak_log", Entero),
        ("grass_block", Entero),
        ("white_wool", Entero),
        ("glass", Entero),
        ("crimson_stem", Entero),
        ("mushroom_stem", Entero),
        ("brown_mushroom_block", Entero),
        ("dead_tube_coral_block", Entero),
        ("torch", Otro),
        ("oak_wall_sign", Otro),
        ("lantern", Otro),
        ("poppy", Otro),
        ("chest", Otro),
        ("short_grass", Otro),
        ("flower_pot", Otro),
        ("cactus", Otro),
        ("dirt_path", Otro),
        ("oak_sapling", Otro),
        ("brown_mushroom", Otro),
    ];
    for (n, f) in casos {
        assert_eq!(forma_sin_jar(&format!("minecraft:{n}")), f, "{n}");
    }
    assert_eq!(
        forma_por_sufijo("minecraft:oak_stairs[facing=east]"),
        Some(Escalera)
    );
    assert_eq!(forma_por_sufijo("stone"), None);
    assert_eq!(forma_sin_jar("mod:piedra"), Otro);
}

// ------------------------------------------------------------- propiedades

#[test]
fn propiedades_desde_blockstate() {
    let escalera = json!({ "variants": {
        "facing=east,half=bottom,shape=straight": { "model": "a" },
        "facing=north,half=top,shape=inner_left": { "model": "a" } } });
    let p = propiedades::desde_blockstate(&escalera, FormaBloque::Escalera);
    let def: Vec<(&str, &str)> = p
        .iter()
        .map(|p| (p.nombre.as_str(), p.defecto.as_str()))
        .collect();
    assert_eq!(
        def,
        [
            ("facing", "north"),
            ("half", "bottom"),
            ("shape", "straight")
        ]
    );

    let muro = json!({ "multipart": [
        { "when": { "up": "true" }, "apply": { "model": "post" } },
        { "when": { "north": "low" }, "apply": { "model": "side" } },
        { "when": { "OR": [ { "north": "tall" }, { "east": "tall|low" } ] }, "apply": { "model": "x" } } ] });
    let p = propiedades::desde_blockstate(&muro, FormaBloque::Muro);
    let buscar = |n: &str| p.iter().find(|x| x.nombre == n).unwrap().clone();
    assert_eq!(buscar("up").valores, ["true", "false"]);
    assert_eq!(buscar("up").defecto, "true");
    assert_eq!(buscar("north").valores, ["none", "low", "tall"]);
    assert_eq!(buscar("north").defecto, "none");
    assert_eq!(buscar("east").defecto, "none");
    let todas = propiedades::combinar(p.clone(), propiedades::plantilla(FormaBloque::Muro, "x"));
    assert!(
        todas
            .iter()
            .any(|x| x.nombre == "waterlogged" && x.defecto == "false")
    );
    assert!(todas.iter().any(|x| x.nombre == "south"));

    let nieve = json!({ "variants": { "layers=10": {}, "layers=2": {}, "layers=1": {} } });
    let p = propiedades::desde_blockstate(&nieve, FormaBloque::Otro);
    assert_eq!(p[0].valores, ["1", "2", "10"]);
    assert_eq!(p[0].defecto, "1");
}

// ------------------------------------------------------------- exclusiones

#[test]
fn exclusiones() {
    use Exclusion::*;
    let casos = [
        ("air", Some(Aire)),
        ("cave_air", Some(Aire)),
        ("water", Some(Fluido)),
        ("lava", Some(Fluido)),
        ("barrier", Some(Tecnico)),
        ("structure_void", Some(Tecnico)),
        ("light", Some(Tecnico)),
        ("jigsaw", Some(Tecnico)),
        ("command_block", Some(Tecnico)),
        ("chain_command_block", Some(Tecnico)),
        ("repeating_command_block", Some(Tecnico)),
        ("structure_block", Some(Tecnico)),
        ("chest", Some(ConDatos)),
        ("barrel", Some(ConDatos)),
        ("furnace", Some(ConDatos)),
        ("shulker_box", Some(ConDatos)),
        ("red_shulker_box", Some(ConDatos)),
        ("oak_sign", Some(ConDatos)),
        ("cherry_hanging_sign", Some(ConDatos)),
        ("oak_wall_sign", Some(SinObjeto)),
        ("potted_poppy", Some(SinObjeto)),
        ("infested_stone", Some(SinObjeto)),
        ("stone", None),
        ("oak_planks", None),
        ("oak_stairs", None),
        ("glass", None),
    ];
    for (n, m) in casos {
        assert_eq!(motivo_exclusion(&format!("minecraft:{n}")), m, "{n}");
        assert_eq!(es_sugerible(n), m.is_none(), "{n}");
    }
    assert!(!es_sugerible("minecraft:chest[facing=east,type=single]"));
}

// ------------------------------------------------------------- jar real

/// Compara la heurística sin jar con los modelos del jar real: ningún bloque
/// que según el jar no es un cubo puede salir `Entero` sin jar.
#[test]
#[ignore = "necesita un jar de Minecraft instalado"]
fn heuristica_sin_jar_frente_al_jar_real() {
    let v = crate::texturas::pruebas::jar_real();
    let dir = tempfile::tempdir().unwrap();
    let inicio = std::time::Instant::now();
    let activo = Arc::new(texturas::cargar_o_generar(dir.path(), &v).unwrap());
    let t_atlas = inicio.elapsed();
    let inicio = std::time::Instant::now();
    let c = Catalogo::con_activo(Arc::clone(&activo), dir.path());
    let t_cat = inicio.elapsed();
    let inicio = std::time::Instant::now();
    let c2 = Catalogo::con_activo(Arc::clone(&activo), dir.path());
    println!(
        "{}: {} bloques; atlas {t_atlas:?}, catálogo {t_cat:?}, desde caché {:?}",
        v.id,
        c.len(),
        inicio.elapsed()
    );
    assert_eq!(c.len(), c2.len());
    let mut falsos_enteros = Vec::new();
    let mut perdidos = Vec::new();
    for b in c.bloques() {
        let sin = forma_sin_jar(&b.id);
        if sin == FormaBloque::Entero && b.forma != FormaBloque::Entero {
            falsos_enteros.push(b.id.clone());
        } else if sin != FormaBloque::Entero && b.forma == FormaBloque::Entero {
            perdidos.push(b.id.clone());
        }
    }
    println!(
        "enteros con jar que sin jar son «otro» ({}): {perdidos:?}",
        perdidos.len()
    );
    for (id, f) in [
        ("minecraft:stone", FormaBloque::Entero),
        ("minecraft:oak_stairs", FormaBloque::Escalera),
    ] {
        assert_eq!(c.forma(id), f, "{id}");
    }
    let escalera = c.bloque("minecraft:oak_stairs").unwrap();
    println!("oak_stairs: {:?}", escalera.propiedades);
    println!(
        "cobblestone_wall: {}",
        c.bloque("minecraft:cobblestone_wall")
            .unwrap()
            .estado_por_defecto()
    );
    assert!(
        falsos_enteros.is_empty(),
        "falsos enteros sin jar: {falsos_enteros:?}"
    );
}
