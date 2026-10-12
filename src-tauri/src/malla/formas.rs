// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Clasificación de cada block state en una forma: nada, cubo completo o
//! modelo aproximado con cajas y planos cruzados (coordenadas en 1/16 de
//! bloque, como los modelos de Minecraft).
//!
//! No es exhaustiva: lo que no aparece aquí se dibuja como cubo opaco, que es
//! lo correcto para la gran mayoría de bloques. Los modelos JSON completos
//! quedan como mejora futura (ver `docs/visor.md`).

use super::apariencia::{Tipo, nombre_de, propiedad};

/// Pieza de un modelo, en 1/16 de bloque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pieza {
    /// Caja alineada con los ejes: `min` < `max` en los tres ejes, 0..=16.
    Caja { min: [u8; 3], max: [u8; 3] },
    /// Dos planos verticales cruzados en diagonal, de esquina a esquina.
    Cruz,
}

/// Forma de un block state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Forma {
    /// Sin geometría (aires, vacío de estructura, luz, barrera).
    Nada,
    /// Cubo completo. Si es `Opaco` oculta las caras de sus vecinos; si no,
    /// solo oculta la cara compartida con otro bloque del mismo nombre.
    Cubo(Tipo),
    /// Modelo aproximado: no oculta caras de nadie.
    Modelo(Tipo, Vec<Pieza>),
}

const fn caja(min: [u8; 3], max: [u8; 3]) -> Pieza {
    Pieza::Caja { min, max }
}

/// Gira una pieza descrita «mirando al norte» hacia `facing` (sobre el eje Y).
fn girar(p: Pieza, facing: &str) -> Pieza {
    let Pieza::Caja { min, max } = p else {
        return p;
    };
    // Un giro de 90° norte → este lleva (x, z) a (16 - z, x).
    let vueltas = match facing {
        "east" => 1,
        "south" => 2,
        "west" => 3,
        _ => 0,
    };
    let (mut a, mut b) = (min, max);
    for _ in 0..vueltas {
        let (x0, x1) = (16 - b[2], 16 - a[2]);
        let (z0, z1) = (a[0], b[0]);
        a = [x0, a[1], z0];
        b = [x1, b[1], z1];
    }
    caja(a, b)
}

/// Pieza «hacia el norte» girada hacia cada lado horizontal cuyo valor de
/// propiedad cumpla `activo`.
fn brazos(estado: &str, norte: &[Pieza], activo: impl Fn(&str) -> bool) -> Vec<Pieza> {
    let mut v = Vec::new();
    for lado in ["north", "east", "south", "west"] {
        if propiedad(estado, lado).is_some_and(&activo) {
            v.extend(norte.iter().map(|&p| girar(p, lado)));
        }
    }
    v
}

fn facing(estado: &str) -> &str {
    propiedad(estado, "facing").unwrap_or("north")
}

/// Bloques sin geometría.
const INVISIBLES: &[&str] = &[
    "air",
    "cave_air",
    "void_air",
    "structure_void",
    "light",
    "barrier",
    "moving_piston",
];

/// Cubos semitransparentes (se pintan en la pasada translúcida).
const TRANSLUCIDOS: &[&str] = &[
    "water",
    "bubble_column",
    "glass",
    "ice",
    "frosted_ice",
    "slime_block",
    "honey_block",
];

/// Cubos con recortes de alfa.
const RECORTE: &[&str] = &["spawner", "trial_spawner", "mangrove_roots", "scaffolding"];

/// Plantas y similares que se dibujan como dos planos cruzados.
const CRUZADAS: &[&str] = &[
    "short_grass",
    "grass",
    "fern",
    "dead_bush",
    "bush",
    "firefly_bush",
    "short_dry_grass",
    "tall_dry_grass",
    "tall_grass",
    "large_fern",
    "sunflower",
    "lilac",
    "rose_bush",
    "peony",
    "dandelion",
    "poppy",
    "blue_orchid",
    "allium",
    "azure_bluet",
    "oxeye_daisy",
    "cornflower",
    "lily_of_the_valley",
    "wither_rose",
    "torchflower",
    "closed_eyeblossom",
    "open_eyeblossom",
    "cactus_flower",
    "pitcher_plant",
    "brown_mushroom",
    "red_mushroom",
    "crimson_fungus",
    "warped_fungus",
    "crimson_roots",
    "warped_roots",
    "nether_sprouts",
    "hanging_roots",
    "sweet_berry_bush",
    "cobweb",
    "sugar_cane",
    "wheat",
    "carrots",
    "potatoes",
    "beetroots",
    "nether_wart",
    "kelp",
    "kelp_plant",
    "seagrass",
    "tall_seagrass",
    "mangrove_propagule",
    "bamboo_sapling",
    "twisting_vines",
    "twisting_vines_plant",
    "weeping_vines",
    "weeping_vines_plant",
    "cave_vines",
    "cave_vines_plant",
    "spore_blossom",
    "pale_hanging_moss",
    "melon_stem",
    "pumpkin_stem",
    "attached_melon_stem",
    "attached_pumpkin_stem",
    "torchflower_crop",
    "pitcher_crop",
];

/// Sufijos que también van como planos cruzados.
const SUFIJOS_CRUZADOS: &[&str] = &[
    "_sapling",
    "_tulip",
    "_coral",
    "_coral_fan",
    "_coral_wall_fan",
];

/// Láminas planas sobre el suelo (1/16 de alto).
const PLANAS: &[&str] = &[
    "pink_petals",
    "wildflowers",
    "leaf_litter",
    "lily_pad",
    "redstone_wire",
];

/// Láminas pegadas a los lados según `north`/`east`/…/`up`/`down`.
const ENREDADERAS: &[&str] = &["vine", "glow_lichen", "sculk_vein", "resin_clump"];

/// Forma de un block state normalizado (`ns:nombre[props]`).
pub fn forma_de(estado: &str) -> Forma {
    let completo = nombre_de(estado);
    // Los bloques de mods se dibujan como cubo; las reglas son de vanilla.
    let Some(n) = completo.strip_prefix("minecraft:") else {
        return Forma::Cubo(Tipo::Opaco);
    };
    if INVISIBLES.contains(&n) {
        return Forma::Nada;
    }
    if TRANSLUCIDOS.contains(&n) || n.ends_with("_stained_glass") || n == "tinted_glass" {
        return Forma::Cubo(Tipo::Translucido);
    }
    if n.ends_with("_leaves") || RECORTE.contains(&n) {
        return Forma::Cubo(Tipo::Recorte);
    }
    if CRUZADAS.contains(&n) || SUFIJOS_CRUZADOS.iter().any(|s| n.ends_with(s)) {
        return Forma::Modelo(Tipo::Recorte, vec![Pieza::Cruz]);
    }
    if let Some(f) = forma_modelo(estado, n) {
        return f;
    }
    Forma::Cubo(Tipo::Opaco)
}

/// Modelos aproximados con cajas. `None`: no es un caso conocido.
fn forma_modelo(estado: &str, n: &str) -> Option<Forma> {
    let opaco = |piezas: Vec<Pieza>| Some(Forma::Modelo(Tipo::Opaco, piezas));
    let recorte = |piezas: Vec<Pieza>| Some(Forma::Modelo(Tipo::Recorte, piezas));

    if n.ends_with("_slab") {
        return match propiedad(estado, "type") {
            Some("top") => opaco(vec![caja([0, 8, 0], [16, 16, 16])]),
            Some("double") => Some(Forma::Cubo(Tipo::Opaco)),
            _ => opaco(vec![caja([0, 0, 0], [16, 8, 16])]),
        };
    }
    if n.ends_with("_stairs") {
        // Base de losa + escalón en la mitad hacia la que mira (`facing`).
        // Se ignora `shape` (esquinas interiores y exteriores).
        let arriba = propiedad(estado, "half") == Some("top");
        let (base, escalon) = if arriba {
            ((8, 16), (0, 8))
        } else {
            ((0, 8), (8, 16))
        };
        return opaco(vec![
            caja([0, base.0, 0], [16, base.1, 16]),
            girar(caja([0, escalon.0, 0], [16, escalon.1, 8]), facing(estado)),
        ]);
    }
    if n.ends_with("_fence_gate") {
        let hoja = girar(caja([0, 5, 7], [16, 16, 9]), facing(estado));
        return opaco(vec![hoja]);
    }
    if n.ends_with("_fence") {
        let mut v = vec![caja([6, 0, 6], [10, 16, 10])];
        v.extend(brazos(
            estado,
            &[caja([7, 12, 0], [9, 15, 6]), caja([7, 6, 0], [9, 9, 6])],
            |x| x == "true",
        ));
        return opaco(v);
    }
    if n.ends_with("_wall") {
        let poste = propiedad(estado, "up") != Some("false");
        let fondo = if poste { 4 } else { 8 };
        let mut v = Vec::new();
        if poste {
            v.push(caja([4, 0, 4], [12, 16, 12]));
        }
        v.extend(brazos(estado, &[caja([5, 0, 0], [11, 14, fondo])], |x| {
            matches!(x, "low" | "true")
        }));
        v.extend(brazos(estado, &[caja([5, 0, 0], [11, 16, fondo])], |x| {
            x == "tall"
        }));
        return opaco(v);
    }
    if n == "iron_bars" || n == "glass_pane" || n.ends_with("_glass_pane") {
        let mut v = vec![caja([7, 0, 7], [9, 16, 9])];
        v.extend(brazos(estado, &[caja([7, 0, 0], [9, 16, 7])], |x| {
            x == "true"
        }));
        let tipo = if n == "iron_bars" {
            Tipo::Recorte
        } else {
            Tipo::Translucido
        };
        return Some(Forma::Modelo(tipo, v));
    }
    if n.ends_with("_carpet") {
        return opaco(vec![caja([0, 0, 0], [16, 1, 16])]);
    }
    if n.ends_with("_pressure_plate") {
        return opaco(vec![caja([1, 0, 1], [15, 1, 15])]);
    }
    if n == "snow" {
        let capas: u8 = propiedad(estado, "layers")
            .and_then(|c| c.parse().ok())
            .unwrap_or(1)
            .clamp(1, 8);
        if capas == 8 {
            return Some(Forma::Cubo(Tipo::Opaco));
        }
        return opaco(vec![caja([0, 0, 0], [16, capas * 2, 16])]);
    }
    if n.ends_with("_wall_torch") || n == "wall_torch" {
        // Pegada a la pared opuesta a `facing`; se ignora la inclinación.
        return opaco(vec![girar(caja([7, 3, 11], [9, 13, 15]), facing(estado))]);
    }
    if n == "torch" || n.ends_with("_torch") {
        return opaco(vec![caja([7, 0, 7], [9, 10, 9])]);
    }
    if PLANAS.contains(&n) || n.ends_with("rail") {
        return recorte(vec![caja([0, 0, 0], [16, 1, 16])]);
    }
    if ENREDADERAS.contains(&n) {
        let mut v = brazos(estado, &[caja([0, 0, 0], [16, 16, 1])], |x| x == "true");
        if propiedad(estado, "up") == Some("true") {
            v.push(caja([0, 15, 0], [16, 16, 16]));
        }
        if propiedad(estado, "down") == Some("true") {
            v.push(caja([0, 0, 0], [16, 1, 16]));
        }
        if v.is_empty() {
            v.push(caja([0, 0, 15], [16, 16, 16]));
        }
        return recorte(v);
    }
    if n.ends_with("_door") {
        // Cerrada; `open` y `hinge` se ignoran.
        return opaco(vec![girar(caja([0, 0, 13], [16, 16, 16]), facing(estado))]);
    }
    if n.ends_with("_trapdoor") {
        let pieza = if propiedad(estado, "open") == Some("true") {
            girar(caja([0, 0, 13], [16, 16, 16]), facing(estado))
        } else if propiedad(estado, "half") == Some("top") {
            caja([0, 13, 0], [16, 16, 16])
        } else {
            caja([0, 0, 0], [16, 3, 16])
        };
        return opaco(vec![pieza]);
    }
    if n == "ladder" {
        return recorte(vec![girar(caja([0, 0, 13], [16, 16, 16]), facing(estado))]);
    }
    if n.ends_with("_button") || n == "lever" {
        let (ancho, fondo) = if n == "lever" {
            ((5, 11), 10)
        } else {
            ((5, 11), 14)
        };
        let alto = 16 - fondo;
        let pieza = match propiedad(estado, "face") {
            Some("floor") => girar(caja([ancho.0, 0, 6], [ancho.1, alto, 10]), facing(estado)),
            Some("ceiling") => girar(caja([ancho.0, fondo, 6], [ancho.1, 16, 10]), facing(estado)),
            _ => girar(caja([ancho.0, 6, fondo], [ancho.1, 10, 16]), facing(estado)),
        };
        return opaco(vec![pieza]);
    }
    if n.ends_with("_wall_sign") || n.ends_with("_wall_hanging_sign") {
        return opaco(vec![girar(caja([0, 4, 14], [16, 12, 16]), facing(estado))]);
    }
    if n.ends_with("_hanging_sign") {
        return opaco(vec![caja([1, 0, 7], [15, 10, 9])]);
    }
    if n.ends_with("_sign") {
        // `rotation` 0..15: se redondea a norte-sur o este-oeste.
        let giro: u8 = propiedad(estado, "rotation")
            .and_then(|r| r.parse().ok())
            .unwrap_or(0);
        let lado = if ((giro + 2) / 4) % 2 == 1 {
            "east"
        } else {
            "north"
        };
        return opaco(vec![
            girar(caja([0, 7, 7], [16, 16, 9]), lado),
            caja([7, 0, 7], [9, 7, 9]),
        ]);
    }
    if n.ends_with("_bed") {
        return opaco(vec![caja([0, 0, 0], [16, 9, 16])]);
    }
    if matches!(n, "chest" | "trapped_chest" | "ender_chest") || n.ends_with("copper_chest") {
        return opaco(vec![caja([1, 0, 1], [15, 14, 15])]);
    }
    // `jack_o_lantern` es un bloque entero: no basta con el sufijo.
    if n == "lantern" || n == "soul_lantern" || n.ends_with("copper_lantern") {
        let colgada = propiedad(estado, "hanging") == Some("true");
        let (y0, y1) = if colgada { (1, 8) } else { (0, 7) };
        return opaco(vec![caja([5, y0, 5], [11, y1, 11])]);
    }
    if n == "chain" || n.ends_with("_chain") || n.ends_with("_rod") {
        let pieza = match propiedad(estado, "axis").or_else(|| propiedad(estado, "facing")) {
            Some("x" | "east" | "west") => caja([0, 7, 7], [16, 9, 9]),
            Some("z" | "north" | "south") => caja([7, 7, 0], [9, 9, 16]),
            _ => caja([7, 0, 7], [9, 16, 9]),
        };
        return opaco(vec![pieza]);
    }
    if n == "flower_pot" || n.starts_with("potted_") {
        return opaco(vec![caja([5, 0, 5], [11, 6, 11])]);
    }
    if (n.ends_with("_head") && n != "piston_head") || n.ends_with("_skull") {
        return opaco(vec![caja([4, 0, 4], [12, 8, 12])]);
    }
    match n {
        "cactus" => opaco(vec![caja([1, 0, 1], [15, 16, 15])]),
        "farmland" | "dirt_path" => opaco(vec![caja([0, 0, 0], [16, 15, 16])]),
        "enchanting_table" => opaco(vec![caja([0, 0, 0], [16, 12, 16])]),
        "cake" => opaco(vec![caja([1, 0, 1], [15, 8, 15])]),
        "daylight_detector" => opaco(vec![caja([0, 0, 0], [16, 6, 16])]),
        "campfire" | "soul_campfire" => opaco(vec![caja([0, 0, 0], [16, 7, 16])]),
        "stonecutter" => opaco(vec![caja([0, 0, 0], [16, 9, 16])]),
        "repeater" | "comparator" => opaco(vec![caja([0, 0, 0], [16, 2, 16])]),
        _ => None,
    }
}
