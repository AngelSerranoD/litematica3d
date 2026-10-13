// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Pruebas de texturas. Las normales usan JSON y PNG en memoria; las que leen
//! el jar real de Minecraft están marcadas `#[ignore]`.

use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::atlas::{self, RELLENO, TAM_CELDA, TAM_TEXTURA, tintes};
use super::modelos::{CaraResuelta, Recursos};
use super::versiones::{self, ENTRADA_PRUEBA};
use super::*;
use crate::malla::{self, Cara, FuenteTexturas, Mundo};

// ---------------------------------------------------------------- recursos

const BLOCK: &str = r##"{ "textures": { "particle": "#all_fallback" } }"##;
const CUBE: &str = r##"{ "parent": "block/block", "elements": [ { "from": [0,0,0], "to": [16,16,16], "faces": {
    "down":  { "texture": "#down" }, "up":    { "texture": "#up" },
    "north": { "texture": "#north" }, "south": { "texture": "#south" },
    "west":  { "texture": "#west" }, "east":  { "texture": "#east" } } } ] }"##;
const CUBE_ALL: &str = r##"{ "parent": "minecraft:block/cube", "textures": {
    "particle": "#all", "down": "#all", "up": "#all", "north": "#all",
    "south": "#all", "west": "#all", "east": "#all" } }"##;
const CUBE_COLUMN: &str = r##"{ "parent": "block/cube", "textures": {
    "particle": "#side", "down": "#end", "up": "#end", "north": "#side",
    "south": "#side", "west": "#side", "east": "#side" } }"##;
const ORIENTABLE: &str = r##"{ "parent": "block/cube", "textures": {
    "particle": "#front", "down": "#top", "up": "#top", "north": "#front",
    "south": "#side", "west": "#side", "east": "#side" } }"##;

fn recursos() -> Recursos {
    let blockstates: Vec<(&str, &str)> = vec![
        (
            "stone",
            r#"{ "variants": { "": { "model": "minecraft:block/stone" } } }"#,
        ),
        (
            "oak_log",
            r#"{ "variants": {
                "axis=x": { "model": "minecraft:block/oak_log", "x": 90, "y": 90 },
                "axis=y": { "model": "minecraft:block/oak_log" },
                "axis=z": { "model": "minecraft:block/oak_log", "x": 90 } } }"#,
        ),
        (
            "furnace",
            r#"{ "variants": {
                "facing=east,lit=false": { "model": "minecraft:block/furnace", "y": 90 },
                "facing=north,lit=false": { "model": "minecraft:block/furnace" },
                "facing=north,lit=true": { "model": "minecraft:block/furnace_on" } } }"#,
        ),
        (
            "grass_block",
            r#"{ "variants": { "snowy=false": [
                { "model": "minecraft:block/grass_block" },
                { "model": "minecraft:block/grass_block", "y": 90 } ] } }"#,
        ),
        (
            "oak_fence",
            r#"{ "multipart": [
                { "apply": { "model": "minecraft:block/oak_fence_post" } },
                { "when": { "north": "true" }, "apply": { "model": "minecraft:block/oak_fence_side", "uvlock": true } },
                { "when": { "OR": [ { "east": "true" }, { "west": "true|false" } ] },
                  "apply": { "model": "minecraft:block/oak_fence_side", "y": 90, "uvlock": true } } ] }"#,
        ),
        (
            "water",
            r#"{ "variants": { "": { "model": "minecraft:block/water" } } }"#,
        ),
        (
            "poppy",
            r#"{ "variants": { "": { "model": "minecraft:block/poppy" } } }"#,
        ),
    ];
    let modelos: Vec<(&str, &str)> = vec![
        ("block", BLOCK),
        ("cube", CUBE),
        ("cube_all", CUBE_ALL),
        ("cube_column", CUBE_COLUMN),
        ("orientable", ORIENTABLE),
        (
            "stone",
            r#"{ "parent": "minecraft:block/cube_all", "textures": { "all": "minecraft:block/stone" } }"#,
        ),
        (
            "oak_log",
            r#"{ "parent": "block/cube_column", "textures": { "end": "block/oak_log_top", "side": "block/oak_log" } }"#,
        ),
        (
            "furnace",
            r#"{ "parent": "block/orientable", "textures": { "front": "block/furnace_front", "side": "block/furnace_side", "top": "block/furnace_top" } }"#,
        ),
        (
            "furnace_on",
            r#"{ "parent": "block/furnace", "textures": { "front": "block/furnace_front_on" } }"#,
        ),
        (
            "grass_block",
            r##"{ "parent": "block/block", "textures": { "top": "block/grass_block_top", "side": "block/grass_block_side", "bottom": "block/dirt", "overlay": "block/grass_block_side_overlay" },
                "elements": [
                  { "faces": { "up": { "texture": "#top", "tintindex": 0 }, "down": { "texture": "#bottom" },
                               "north": { "texture": "#side" }, "south": { "texture": "#side" },
                               "west": { "texture": "#side" }, "east": { "texture": "#side" } } },
                  { "faces": { "north": { "texture": "#overlay", "tintindex": 0 } } } ] }"##,
        ),
        (
            "oak_fence_post",
            r##"{ "textures": { "texture": "block/oak_planks" }, "elements": [
                { "faces": { "up": { "texture": "#texture" }, "north": { "texture": "#texture" } } } ] }"##,
        ),
        (
            "oak_fence_side",
            r##"{ "textures": { "texture": "block/oak_planks" }, "elements": [
                { "faces": { "west": { "texture": "#texture" } } } ] }"##,
        ),
        (
            "water",
            r#"{ "textures": { "particle": "block/water_still" } }"#,
        ),
        (
            "poppy",
            r##"{ "parent": "block/cross", "textures": { "cross": "block/poppy" } }"##,
        ),
        (
            "cross",
            r##"{ "elements": [ { "faces": { "north": { "texture": "#cross" }, "south": { "texture": "#cross" } } } ] }"##,
        ),
    ];
    Recursos::desde_json(
        blockstates
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.as_bytes())),
        modelos
            .into_iter()
            .map(|(k, v)| (format!("block/{k}"), v.as_bytes())),
    )
}

fn tex(r: &Recursos, estado: &str, cara: Cara) -> Option<String> {
    r.resolver(estado, cara).map(|c| c.textura)
}

#[test]
fn herencia_de_modelos_y_variables() {
    let r = recursos();
    for cara in Cara::CUBO {
        assert_eq!(
            tex(&r, "minecraft:stone", cara).as_deref(),
            Some("block/stone")
        );
    }
    // El hijo cambia solo `front`; lo demás viene del padre.
    assert_eq!(
        tex(&r, "minecraft:furnace[facing=north,lit=true]", Cara::Norte).as_deref(),
        Some("block/furnace_front_on")
    );
    assert_eq!(
        tex(&r, "minecraft:furnace[facing=north,lit=true]", Cara::Arriba).as_deref(),
        Some("block/furnace_top")
    );
    // Sin espacio de nombres vanilla, nada.
    assert_eq!(tex(&r, "create:cogwheel", Cara::Arriba), None);
    assert_eq!(tex(&r, "minecraft:inventado", Cara::Arriba), None);
}

#[test]
fn variantes_con_giros() {
    let r = recursos();
    // Tronco vertical: anillos arriba y abajo.
    assert_eq!(
        tex(&r, "minecraft:oak_log[axis=y]", Cara::Arriba).as_deref(),
        Some("block/oak_log_top")
    );
    assert_eq!(
        tex(&r, "minecraft:oak_log[axis=y]", Cara::Norte).as_deref(),
        Some("block/oak_log")
    );
    // Tumbado en X: anillos al este y al oeste, corteza arriba y al norte.
    let x = "minecraft:oak_log[axis=x]";
    assert_eq!(tex(&r, x, Cara::Este).as_deref(), Some("block/oak_log_top"));
    assert_eq!(
        tex(&r, x, Cara::Oeste).as_deref(),
        Some("block/oak_log_top")
    );
    assert_eq!(tex(&r, x, Cara::Arriba).as_deref(), Some("block/oak_log"));
    assert_eq!(tex(&r, x, Cara::Norte).as_deref(), Some("block/oak_log"));
    // Tumbado en Z: anillos al norte y al sur.
    let z = "minecraft:oak_log[axis=z]";
    assert_eq!(
        tex(&r, z, Cara::Norte).as_deref(),
        Some("block/oak_log_top")
    );
    assert_eq!(tex(&r, z, Cara::Sur).as_deref(), Some("block/oak_log_top"));
    assert_eq!(tex(&r, z, Cara::Este).as_deref(), Some("block/oak_log"));
    // Horno mirando al este: el frente al este.
    let h = "minecraft:furnace[facing=east,lit=false]";
    assert_eq!(
        tex(&r, h, Cara::Este).as_deref(),
        Some("block/furnace_front")
    );
    assert_eq!(
        tex(&r, h, Cara::Norte).as_deref(),
        Some("block/furnace_side")
    );
}

#[test]
fn giro_de_la_textura_en_troncos_tumbados() {
    let r = recursos();
    let rot = |e: &str, c: Cara| r.resolver(e, c).map(|c| c.rotacion);
    assert_eq!(rot("minecraft:oak_log[axis=y]", Cara::Norte), Some(0));
    // Tumbado en Z: en el lado este la veta va de norte a sur (horizontal).
    assert_eq!(rot("minecraft:oak_log[axis=z]", Cara::Este), Some(90));
    assert_eq!(rot("minecraft:oak_log[axis=z]", Cara::Oeste), Some(270));
    assert_eq!(rot("minecraft:oak_log[axis=z]", Cara::Arriba), Some(0));
    // Tumbado en X: en la cara norte la veta va de este a oeste.
    let n = rot("minecraft:oak_log[axis=x]", Cara::Norte).unwrap();
    assert!(n == 90 || n == 270, "giro {n}");
    assert_eq!(
        rot("minecraft:oak_log[axis=x]", Cara::Arriba).map(|g| g % 180),
        Some(90)
    );
    // Con uvlock no se gira.
    assert_eq!(
        rot(
            "minecraft:oak_fence[east=true,north=false,west=false]",
            Cara::Sur
        ),
        None
    );
}

#[test]
fn variante_que_mejor_encaja_y_listas_con_peso() {
    let r = recursos();
    // Propiedad que el estado no trae (`lit`): no descarta.
    assert_eq!(
        tex(&r, "minecraft:furnace[facing=east]", Cara::Este).as_deref(),
        Some("block/furnace_front")
    );
    // Sin propiedades: la primera variante.
    assert!(tex(&r, "minecraft:furnace", Cara::Arriba).is_some());
    // Lista con pesos: la primera (sin giro): la parte de arriba es la tintada.
    assert_eq!(
        tex(&r, "minecraft:grass_block[snowy=false]", Cara::Arriba).as_deref(),
        Some("block/grass_block_top")
    );
    // El lado: el primer elemento que tiene esa cara, no la capa de encima.
    assert_eq!(
        tex(&r, "minecraft:grass_block[snowy=false]", Cara::Norte).as_deref(),
        Some("block/grass_block_side")
    );
}

#[test]
fn multipart_con_when_y_or() {
    let r = recursos();
    let partes = |props: &[(&str, &str)]| {
        r.aplicados("oak_fence", props)
            .into_iter()
            .map(|a| (a.modelo, a.y))
            .collect::<Vec<_>>()
    };
    assert_eq!(partes(&[]), vec![("block/oak_fence_post".to_string(), 0)]);
    assert_eq!(
        partes(&[("north", "true"), ("east", "false"), ("west", "false")]).len(),
        // poste + norte + (west=false cumple `true|false`)
        3
    );
    assert_eq!(
        partes(&[("north", "false"), ("east", "true")]),
        vec![
            ("block/oak_fence_post".to_string(), 0),
            ("block/oak_fence_side".to_string(), 90)
        ]
    );
    // Cara que solo tiene una parte con giro: el lado (oeste del modelo) del
    // travesaño girado 90° queda al norte.
    assert_eq!(
        tex(
            &r,
            "minecraft:oak_fence[east=true,north=false]",
            Cara::Norte
        )
        .as_deref(),
        Some("block/oak_planks")
    );
    assert_eq!(
        tex(
            &r,
            "minecraft:oak_fence[east=false,north=false,west=false]",
            Cara::Sur
        ),
        None
    );
}

#[test]
fn particula_cruz_y_tintes() {
    let r = recursos();
    assert_eq!(
        tex(&r, "minecraft:water[level=0]", Cara::Arriba).as_deref(),
        Some("block/water_still")
    );
    assert_eq!(
        tex(&r, "minecraft:poppy", Cara::Cruz).as_deref(),
        Some("block/poppy")
    );
    let t = atlas::mapa_tintes(&r);
    assert_eq!(t.get("block/grass_block_top"), Some(&tintes::HIERBA));
    assert_eq!(
        t.get("block/grass_block_side_overlay"),
        Some(&tintes::HIERBA)
    );
    assert_eq!(t.get("block/water_still"), Some(&tintes::AGUA));
    assert_eq!(t.get("block/grass_block_side"), None);
    assert_eq!(atlas::tinte_de("birch_leaves"), Some(tintes::ABEDUL));
    assert_eq!(atlas::tinte_de("oak_leaves"), Some(tintes::FOLLAJE));
    assert_eq!(atlas::tinte_de("vine"), Some(tintes::FOLLAJE));
    // El juego no tinta el caldero de lava aunque herede `tintindex`.
    assert_eq!(atlas::tinte_de("lava_cauldron"), None);
    assert_eq!(atlas::tinte_de("cherry_leaves"), None);
}

// ---------------------------------------------------------------- atlas

fn png_de(ancho: u32, alto: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut px = Vec::new();
    for y in 0..alto {
        for x in 0..ancho {
            px.extend_from_slice(&f(x, y));
        }
    }
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, ancho, alto);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(&px).unwrap();
    }
    out
}

fn decodificar_atlas(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut d = png::Decoder::new(std::io::Cursor::new(png_bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; d.output_buffer_size().unwrap()];
    let info = d.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    (info.width, info.height, buf)
}

#[test]
fn dimensiones_potencia_de_dos() {
    assert_eq!(atlas::dimensiones(1), (32, 32, 1));
    assert_eq!(atlas::dimensiones(5), (128, 64, 4));
    let (w, h, f) = atlas::dimensiones(1100);
    assert!(w.is_power_of_two() && h.is_power_of_two());
    assert!((f * (h / TAM_CELDA)) as usize >= 1100);
    assert_eq!((w, h), (2048, 1024));
}

#[test]
fn atlas_con_relleno_animadas_escaladas_y_tinte() {
    let mut pngs = BTreeMap::new();
    // Gradiente: el píxel (x, y) vale (x*16, y*16).
    pngs.insert(
        "block/a".to_string(),
        png_de(16, 16, |x, y| [(x * 16) as u8, (y * 16) as u8, 0, 255]),
    );
    // Animada 16×48: tres fotogramas; el primero, rojo.
    pngs.insert(
        "block/b_anim".to_string(),
        png_de(16, 48, |_, y| {
            if y < 16 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        }),
    );
    // 32×32 (pack de alta resolución): se escala.
    pngs.insert(
        "block/c_32".to_string(),
        png_de(32, 32, |_, _| [10, 20, 30, 128]),
    );
    // Tintada: blanca × hierba.
    pngs.insert(
        "block/d_hierba".to_string(),
        png_de(16, 16, |_, _| [255, 255, 255, 255]),
    );
    pngs.insert("block/roto".to_string(), b"no es un png".to_vec());
    let tintes = HashMap::from([("block/d_hierba".to_string(), tintes::HIERBA)]);
    let a = atlas::montar("prueba", 1, &pngs, &tintes).unwrap();
    let m = &a.meta;
    assert_eq!(m.texturas.len(), 4);
    assert_eq!(m.texturas["block/a"], 0);
    assert_eq!(m.texturas["block/d_hierba"], 3);
    assert!(!m.texturas.contains_key("block/roto"));
    assert_eq!((m.ancho, m.alto, m.celdas_por_fila), (64, 64, 2));
    assert_eq!(m.colores[1], [255, 0, 0]);
    assert_eq!(m.colores[2], [10, 20, 30]);
    assert_eq!(m.colores[3], tintes::HIERBA);

    let (w, h, px) = decodificar_atlas(&a.png);
    assert_eq!((w, h), (64, 64));
    let en = |x: u32, y: u32| -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    };
    // Celda 0: la textura empieza en (RELLENO, RELLENO).
    assert_eq!(en(RELLENO, RELLENO), [0, 0, 0, 255]);
    assert_eq!(en(RELLENO + 15, RELLENO + 3), [240, 48, 0, 255]);
    // Relleno: la textura repetida (a la izquierda del borde, la columna 15).
    assert_eq!(en(RELLENO - 1, RELLENO), [240, 0, 0, 255]);
    assert_eq!(en(0, 0), [128, 128, 0, 255]);
    assert_eq!(en(RELLENO + TAM_TEXTURA, RELLENO), [0, 0, 0, 255]);
    // Celda 1 (animada): solo el primer fotograma.
    for y in 0..TAM_CELDA {
        assert_eq!(en(TAM_CELDA + 5, y), [255, 0, 0, 255]);
    }
    let d = m.datos();
    assert_eq!(d.celda_uv, [0.5, 0.5]);
    assert_eq!(d.niveles_mip, 3);
}

// ---------------------------------------------------------------- versiones

fn jar_falso(ruta: &Path, con_assets: bool) {
    let f = std::fs::File::create(ruta).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let o =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    if con_assets {
        z.start_file(ENTRADA_PRUEBA, o).unwrap();
        z.write_all(br#"{ "variants": { "": { "model": "minecraft:block/stone" } } }"#)
            .unwrap();
        z.start_file("assets/minecraft/models/block/stone.json", o)
            .unwrap();
        z.write_all(br#"{ "parent": "minecraft:block/cube_all", "textures": { "all": "minecraft:block/stone" } }"#)
            .unwrap();
        z.start_file("assets/minecraft/models/block/cube_all.json", o)
            .unwrap();
        z.write_all(CUBE_ALL.as_bytes()).unwrap();
        z.start_file("assets/minecraft/models/block/cube.json", o)
            .unwrap();
        z.write_all(CUBE.as_bytes()).unwrap();
        z.start_file("assets/minecraft/textures/block/stone.png", o)
            .unwrap();
        z.write_all(&png_de(16, 16, |_, _| [120, 120, 120, 255]))
            .unwrap();
    } else {
        z.start_file("net/minecraft/Main.class", o).unwrap();
        z.write_all(b"x").unwrap();
    }
    z.finish().unwrap();
}

fn version_falsa(mc: &Path, id: &str, json: &str, jar: Option<bool>) {
    let d = mc.join("versions").join(id);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{id}.json")), json).unwrap();
    if let Some(con_assets) = jar {
        jar_falso(&d.join(format!("{id}.jar")), con_assets);
    }
}

fn mc_falso() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let mc = t.path();
    let rel = |fecha: &str| format!(r#"{{ "type": "release", "releaseTime": "{fecha}" }}"#);
    version_falsa(mc, "1.20.1", &rel("2023-06-12T00:00:00+00:00"), Some(true));
    version_falsa(mc, "1.21.8", &rel("2025-07-17T00:00:00+00:00"), Some(true));
    version_falsa(mc, "1.21.11", &rel("2025-12-09T00:00:00+00:00"), None);
    version_falsa(
        mc,
        "26.1-snapshot-1",
        r#"{ "type": "snapshot", "releaseTime": "2026-01-01T00:00:00+00:00" }"#,
        Some(true),
    );
    version_falsa(
        mc,
        "1.21.8-OptiFine",
        r#"{ "type": "release", "inheritsFrom": "1.21.8", "releaseTime": "2025-11-30T00:00:00+00:00" }"#,
        Some(true),
    );
    version_falsa(
        mc,
        "fabric-loader",
        &rel("2026-02-01T00:00:00+00:00"),
        Some(false),
    );
    t
}

#[test]
fn detecta_versiones_con_jar_y_ordena_releases_primero() {
    let t = mc_falso();
    let v = versiones::listar(&[t.path().to_path_buf()]);
    let ids: Vec<&str> = v.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(
        ids,
        ["1.21.8", "1.20.1", "26.1-snapshot-1", "1.21.8-OptiFine"]
    );
    assert!(v[0].es_release && v[1].es_release);
    assert!(!v[2].es_release && !v[3].es_release);
    assert!(v[0].ruta.ends_with("1.21.8.jar"));
    let json = serde_json::to_value(&v[0]).unwrap();
    assert_eq!(json["esRelease"], true);
    assert!(json.get("fecha").is_none());
}

#[test]
fn ajustes_version_elegida_cache_e_invalidacion() {
    let mc = mc_falso();
    let datos = tempfile::tempdir().unwrap();
    let cache = datos.path().join("cache");
    let config = datos.path().join("config");
    let g = GestorTexturas::nuevo(&cache, &config);
    assert_eq!(g.ajustes(), Ajustes::default());
    g.guardar_ajustes(Ajustes {
        version_minecraft: None,
        ruta_minecraft: Some(mc.path().to_string_lossy().into_owned()),
    })
    .unwrap();
    assert_eq!(g.version_elegida().unwrap().id, "1.21.8");

    let a = g.preparar().unwrap().expect("atlas");
    assert_eq!(a.meta.version, "1.21.8");
    assert!(cache.join("texturas/1.21.8/atlas.png").is_file());
    assert!(cache.join("texturas/1.21.8/atlas.json").is_file());
    let t = a.textura("minecraft:stone", Cara::Arriba).unwrap();
    assert_eq!(t.color, [120, 120, 120]);
    assert_eq!(g.listo().unwrap().meta.version, "1.21.8");

    // Otra versión: se descarta el activo y se prepara la nueva.
    g.guardar_ajustes(Ajustes {
        version_minecraft: Some("1.20.1".into()),
        ruta_minecraft: Some(mc.path().to_string_lossy().into_owned()),
    })
    .unwrap();
    assert!(g.listo().is_none());
    assert_eq!(g.preparar().unwrap().unwrap().meta.version, "1.20.1");

    // Los ajustes se releen del disco; una versión que no existe cae a la
    // primera de la lista.
    let g2 = GestorTexturas::nuevo(&cache, &config);
    assert_eq!(g2.ajustes().version_minecraft.as_deref(), Some("1.20.1"));
    // Segunda vez: sale de la caché (aunque no haya PNG en el jar, no se lee).
    assert_eq!(g2.preparar().unwrap().unwrap().meta.version, "1.20.1");
    let json = std::fs::read_to_string(config.join("ajustes.json")).unwrap();
    assert!(json.contains("\"versionMinecraft\""));
    assert!(json.contains("\"rutaMinecraft\""));
    g2.guardar_ajustes(Ajustes {
        version_minecraft: Some("9.9".into()),
        ruta_minecraft: Some(mc.path().to_string_lossy().into_owned()),
    })
    .unwrap();
    assert_eq!(g2.version_elegida().unwrap().id, "1.21.8");

    // Sin ningún jar: no hay atlas y no es un error.
    let vacio = tempfile::tempdir().unwrap();
    g2.guardar_ajustes(Ajustes {
        version_minecraft: None,
        ruta_minecraft: Some(vacio.path().to_string_lossy().into_owned()),
    })
    .unwrap();
    assert!(g2.preparar().unwrap().is_none());
}

// ---------------------------------------------------------------- malla

struct FuenteFalsa;

impl FuenteTexturas for FuenteFalsa {
    fn textura(&self, estado: &str, cara: Cara) -> Option<TexturaCara> {
        (estado == "minecraft:stone").then_some(TexturaCara {
            celda: 7,
            rotacion: if cara == Cara::Norte { 90 } else { 0 },
            color: [1, 2, 3],
        })
    }
    fn datos_atlas(&self) -> DatosAtlas {
        atlas::montar("x", 0, &BTreeMap::new(), &HashMap::new())
            .unwrap()
            .meta
            .datos()
    }
}

fn mundo_simple() -> Mundo {
    use crate::schematic::modelo::{Dimensiones, Formato, Region, Schematic, caja_englobante};
    let regiones = vec![Region {
        nombre: "r".into(),
        origen: [0, 0, 0],
        tamano: Dimensiones {
            ancho: 2,
            alto: 1,
            largo: 1,
        },
        paleta: vec![
            "minecraft:air".into(),
            "minecraft:stone".into(),
            "minecraft:dirt".into(),
        ],
        bloques: vec![1, 2],
    }];
    Mundo::desde_schematic(&Schematic {
        formato: Formato::Litematic,
        nombre: None,
        autor: None,
        descripcion: None,
        version_datos: None,
        fecha_creacion: None,
        dimensiones: caja_englobante(&regiones),
        regiones,
    })
    .unwrap()
}

#[test]
fn malla_rellena_textura_rotacion_y_atlas() {
    let mundo = mundo_simple();
    let m = malla::mallar_con_texturas(&mundo, &malla::Opciones::default(), Some(&FuenteFalsa))
        .unwrap();
    let piedra: Vec<_> = m
        .materiales
        .iter()
        .filter(|x| x.nombre == "minecraft:stone")
        .collect();
    assert!(!piedra.is_empty());
    assert!(
        piedra
            .iter()
            .all(|x| x.textura == Some(7) && x.color == [1, 2, 3])
    );
    assert!(
        piedra
            .iter()
            .any(|x| x.cara == Cara::Norte && x.rotacion == 90)
    );
    let tierra = m
        .materiales
        .iter()
        .find(|x| x.nombre == "minecraft:dirt")
        .unwrap();
    assert_eq!(tierra.textura, None);
    assert_eq!(tierra.rotacion, 0);

    let bin = malla::binario::serializar(&m);
    let (cab, _) = malla::binario::leer(&bin).unwrap();
    assert_eq!(cab.json["atlas"]["version"], "x");
    assert_eq!(cab.json["atlas"]["celdasPorFila"], 1);
    assert_eq!(cab.json["atlas"]["tamCelda"], 32);
    assert_eq!(cab.json["atlas"]["relleno"], 8);
    assert_eq!(cab.json["atlas"]["nivelesMip"], 3);
    let mat = cab.json["materiales"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["nombre"] == "minecraft:stone")
        .unwrap();
    assert_eq!(mat["textura"], 7);
    assert!(mat["rotacion"].is_number());

    // Sin fuente: atlas null y sin texturas.
    let m = malla::mallar(&mundo, &malla::Opciones::default()).unwrap();
    let (cab, _) = malla::binario::leer(&malla::binario::serializar(&m)).unwrap();
    assert!(cab.json["atlas"].is_null());
    assert!(m.materiales.iter().all(|x| x.textura.is_none()));
}

// ---------------------------------------------------------------- jar real

/// Jar de Minecraft para las pruebas de desarrollo: `LITEMATICA3D_MC_VERSION`
/// si está, si no 1.21.11 si tiene jar, y si no la primera release instalada.
fn jar_real() -> VersionMinecraft {
    let v = versiones::listar(&versiones::carpetas_por_defecto());
    let pedida = std::env::var("LITEMATICA3D_MC_VERSION").ok();
    let elegida = match pedida {
        Some(p) => v.iter().find(|x| x.id == p).cloned(),
        None => v
            .iter()
            .find(|x| x.id == "1.21.11")
            .or_else(|| v.first())
            .cloned(),
    };
    elegida.expect("no hay ningún jar de Minecraft instalado")
}

#[test]
#[ignore = "necesita un jar de Minecraft instalado"]
fn atlas_con_jar_real() {
    for x in versiones::listar(&versiones::carpetas_por_defecto()) {
        println!("instalada: {} (release: {}) {}", x.id, x.es_release, x.ruta);
    }
    let v = jar_real();
    let inicio = Instant::now();
    let bytes = std::fs::metadata(&v.ruta).unwrap().len();
    let (activo, a) = generar(Path::new(&v.ruta), &v.id, bytes).unwrap();
    let tiempo = inicio.elapsed();
    let m = &activo.meta;
    println!(
        "versión {}: {} texturas, atlas {}×{} ({} KiB), {} blockstates, {} modelos, generado en {:.0?}",
        v.id,
        m.texturas.len(),
        m.ancho,
        m.alto,
        activo.png.len() / 1024,
        activo.recursos.n_blockstates(),
        activo.recursos.n_modelos(),
        tiempo
    );
    if let Ok(dir) = std::env::var("LITEMATICA3D_VOLCAR_ATLAS") {
        std::fs::write(PathBuf::from(dir).join("atlas.png"), activo.png.as_ref()).unwrap();
    }
    assert!(m.ancho.is_power_of_two() && m.alto.is_power_of_two());
    assert!(m.texturas.len() > 900);
    assert_eq!(a.texturas.len(), m.texturas.len());

    let celda = |estado: &str, cara: Cara| -> (String, TexturaCara) {
        let r: CaraResuelta = activo
            .recursos
            .resolver(estado, cara)
            .unwrap_or_else(|| panic!("{estado} {cara:?} sin textura"));
        let t = activo
            .textura(estado, cara)
            .unwrap_or_else(|| panic!("{estado} {cara:?}: {} no está en el atlas", r.textura));
        (r.textura, t)
    };
    let casos: &[(&str, Cara, &str)] = &[
        ("minecraft:stone", Cara::Arriba, "block/stone"),
        ("minecraft:oak_log[axis=x]", Cara::Este, "block/oak_log_top"),
        ("minecraft:oak_log[axis=x]", Cara::Arriba, "block/oak_log"),
        ("minecraft:oak_log[axis=x]", Cara::Norte, "block/oak_log"),
        (
            "minecraft:grass_block[snowy=false]",
            Cara::Arriba,
            "block/grass_block_top",
        ),
        (
            "minecraft:grass_block[snowy=false]",
            Cara::Norte,
            "block/grass_block_side",
        ),
        (
            "minecraft:grass_block[snowy=false]",
            Cara::Abajo,
            "block/dirt",
        ),
        (
            "minecraft:oak_stairs[facing=north,half=bottom,shape=straight]",
            Cara::Arriba,
            "block/oak_planks",
        ),
        ("minecraft:glass", Cara::Sur, "block/glass"),
        ("minecraft:water", Cara::Arriba, "block/water_still"),
        ("minecraft:poppy", Cara::Cruz, "block/poppy"),
        (
            "minecraft:furnace[facing=east,lit=false]",
            Cara::Este,
            "block/furnace_front",
        ),
        (
            "minecraft:oak_fence[east=true,north=false,south=false,west=false]",
            Cara::Arriba,
            "block/oak_planks",
        ),
    ];
    for (estado, cara, esperado) in casos {
        let (textura, t) = celda(estado, *cara);
        assert_eq!(&textura, esperado, "{estado} {cara:?}");
        println!(
            "{estado} {cara:?} → {textura} celda {} giro {}",
            t.celda, t.rotacion
        );
    }
    // La hierba de arriba va tintada de verde.
    let (_, hierba) = celda("minecraft:grass_block[snowy=false]", Cara::Arriba);
    assert!(hierba.color[1] > hierba.color[0] && hierba.color[1] > hierba.color[2]);
    // Agua azul y lava naranja (sin el tinte del caldero).
    let (_, agua) = celda("minecraft:water", Cara::Arriba);
    assert!(agua.color[2] > agua.color[0]);
    let (_, lava) = celda("minecraft:lava", Cara::Arriba);
    assert!(
        lava.color[0] > 180 && lava.color[2] < 60,
        "lava {:?}",
        lava.color
    );
    // Tronco tumbado: la corteza de los lados va girada.
    let (_, lado) = celda("minecraft:oak_log[axis=x]", Cara::Norte);
    assert!(lado.rotacion == 90 || lado.rotacion == 270);
}

#[test]
#[ignore = "desarrollo: regenera datos/colores_bloques.json desde el jar instalado"]
fn generar_colores_bloques() {
    let v = jar_real();
    let bytes = std::fs::metadata(&v.ruta).unwrap().len();
    let (activo, a) = generar(Path::new(&v.ruta), &v.id, bytes).unwrap();
    let tabla = tabla_colores(&activo, &a);
    assert!(tabla.colores.len() > 900);
    for b in ["stone", "oak_log", "grass_block", "water", "glass", "poppy"] {
        assert!(tabla.colores.contains_key(&format!("minecraft:{b}")), "{b}");
    }
    let mut json = serde_json::to_string(&tabla).unwrap();
    // Una entrada por línea para que los diffs se lean bien.
    json = json
        .replace("],\"", "],\n\"")
        .replace("{\"minecraft", "{\n\"minecraft");
    json.push('\n');
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("datos/colores_bloques.json");
    std::fs::write(&ruta, json).unwrap();
    println!(
        "{} colores de {} en {}",
        tabla.colores.len(),
        v.id,
        ruta.display()
    );
}

// ---------------------------------------------------------------- nombres

const LANG_ES: &str = r#"{
    "block.minecraft.stone": "Piedra",
    "block.minecraft.redstone_wire": "Polvo de redstone (cable)",
    "item.minecraft.redstone": "Polvo de redstone",
    "block.minecraft.oak_sign": "Cartel de roble (bloque)",
    "item.minecraft.oak_sign": "Cartel de roble",
    "block.minecraft.banner.base.black": "Campo negro",
    "item.minecraft.potion.effect.water": "Botella de agua",
    "itemGroup.buildingBlocks": "Bloques de construcción"
}"#;

/// Hash (de mentira, no se comprueba) con el que va el idioma en los assets.
const HASH_LANG: &str = "ab12cd34ef56ab12cd34ef56ab12cd34ef56ab12";

/// `.minecraft` con 1.21.8 (índice «26» con `es_es.json`), una OptiFine que
/// hereda de ella y 1.20.1 apuntando a un índice que no existe.
fn mc_con_idioma(hash: &str) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let mc = t.path();
    version_falsa(
        mc,
        "1.21.8",
        r#"{ "type": "release", "assetIndex": { "id": "26" } }"#,
        Some(true),
    );
    version_falsa(
        mc,
        "1.21.8-OptiFine",
        r#"{ "type": "release", "inheritsFrom": "1.21.8" }"#,
        Some(true),
    );
    version_falsa(
        mc,
        "1.20.1",
        r#"{ "type": "release", "assetIndex": { "id": "5" } }"#,
        Some(true),
    );
    let indices = mc.join("assets").join("indexes");
    std::fs::create_dir_all(&indices).unwrap();
    std::fs::write(
        indices.join("26.json"),
        format!(
            r#"{{ "objects": {{
                "minecraft/lang/en_us.json": {{ "hash": "{HASH_LANG}", "size": 1 }},
                "minecraft/lang/es_es.json": {{ "hash": "{hash}", "size": {} }}
            }} }}"#,
            LANG_ES.len()
        ),
    )
    .unwrap();
    let objetos = mc.join("assets").join("objects").join(&HASH_LANG[..2]);
    std::fs::create_dir_all(&objetos).unwrap();
    std::fs::write(objetos.join(HASH_LANG), LANG_ES).unwrap();
    t
}

fn jar_en(mc: &Path, id: &str) -> PathBuf {
    mc.join("versions").join(id).join(format!("{id}.jar"))
}

#[test]
fn nombres_desde_el_indice_de_assets() {
    let t = mc_con_idioma(HASH_LANG);
    let n = nombres::leer(&jar_en(t.path(), "1.21.8"));
    assert_eq!(n.get("minecraft:stone").map(String::as_str), Some("Piedra"));
    // Si están las dos claves gana la de ítem.
    assert_eq!(
        n.get("minecraft:oak_sign").map(String::as_str),
        Some("Cartel de roble")
    );
    assert_eq!(
        n.get("minecraft:redstone").map(String::as_str),
        Some("Polvo de redstone")
    );
    assert!(n.contains_key("minecraft:redstone_wire"));
    // Las claves con más puntos no son ítems.
    assert!(!n.keys().any(|k| k.contains('.')));
    assert_eq!(n.len(), 4);

    // La OptiFine no trae `assetIndex`: se sigue `inheritsFrom`.
    assert_eq!(nombres::leer(&jar_en(t.path(), "1.21.8-OptiFine")), n);
    // Índice que no existe: vacío, sin error.
    assert!(nombres::leer(&jar_en(t.path(), "1.20.1")).is_empty());
    assert!(nombres::leer(&t.path().join("no/existe.jar")).is_empty());
}

#[test]
fn nombres_con_un_hash_raro_no_salen_de_objects() {
    let t = mc_con_idioma("../../../../versions/1.21.8/1.21.8");
    assert!(nombres::leer(&jar_en(t.path(), "1.21.8")).is_empty());
}

#[test]
fn nombres_dentro_del_jar() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path().join("versions").join("1.12.2");
    std::fs::create_dir_all(&d).unwrap();
    let jar = d.join("1.12.2.jar");
    let mut z = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
    let o = zip::write::SimpleFileOptions::default();
    z.start_file("assets/minecraft/lang/es_es.json", o).unwrap();
    z.write_all(LANG_ES.as_bytes()).unwrap();
    z.finish().unwrap();
    assert_eq!(nombres::leer(&jar).len(), 4);
}

#[test]
fn nombres_se_cachean_en_disco_por_version() {
    let t = mc_con_idioma(HASH_LANG);
    let cache = tempfile::tempdir().unwrap();
    let v = versiones::listar(&[t.path().to_path_buf()])
        .into_iter()
        .find(|v| v.id == "1.21.8")
        .unwrap();
    let primera = nombres_con_cache(cache.path(), &v);
    assert_eq!(primera.len(), 4);
    let ruta = cache.path().join("1.21.8").join(nombres::ARCHIVO_CACHE);
    assert!(ruta.is_file());
    // Aunque desaparezca el idioma de los assets, sale de la caché.
    std::fs::write(
        t.path()
            .join("assets/objects")
            .join(&HASH_LANG[..2])
            .join(HASH_LANG),
        "{}",
    )
    .unwrap();
    assert_eq!(nombres_con_cache(cache.path(), &v), primera);

    // Sin idioma no se guarda nada (se vuelve a mirar la próxima vez).
    let sin = versiones::listar(&[t.path().to_path_buf()])
        .into_iter()
        .find(|v| v.id == "1.20.1")
        .unwrap();
    assert!(nombres_con_cache(cache.path(), &sin).is_empty());
    assert!(!cache.path().join("1.20.1").exists());
}

#[test]
#[ignore = "necesita Minecraft instalado con los assets descargados"]
fn nombres_con_minecraft_real() {
    let v = jar_real();
    let n = nombres::leer(Path::new(&v.ruta));
    println!("{}: {} nombres", v.id, n.len());
    for id in [
        "minecraft:stone",
        "minecraft:oak_planks",
        "minecraft:redstone",
    ] {
        println!("{id} -> {:?}", n.get(id));
    }
    assert!(n.len() > 1000);
}
