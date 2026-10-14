// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Forma de cada bloque, para no proponer nunca un cambio que altere la
//! geometría: una losa solo se cambia por otra losa, una escalera por otra
//! escalera… y lo que no encaja en ninguna forma conocida es [`FormaBloque::Otro`]
//! (nunca se propone cambiar un «otro» por un bloque entero).
//!
//! Cómo se deduce:
//! 1. Por el sufijo del id ([`forma_por_sufijo`]): `_slab`, `_stairs`,
//!    `_wall`, `_fence_gate`, `_fence`, `_trapdoor`, `_door`, `_button`,
//!    `_pressure_plate`, `_pane`/`_bars`, `_carpet`.
//! 2. Si no tiene sufijo de forma, ¿es un cubo completo?
//!    - Con jar ([`forma_con_recursos`]): todos los elementos de los modelos
//!      de su estado por defecto van de 0 a 16 (ver
//!      `Recursos::es_cubo_completo`). Es la fuente fiable.
//!    - Sin jar ([`forma_sin_jar`]): el visor lo dibuja como cubo
//!      (`malla::formas`) y el nombre no está en [`NO_CUBICOS`] ni contiene
//!      ninguno de [`FRAGMENTOS_NO_CUBICOS`]. Ante la duda, `Otro`.

use serde::{Deserialize, Serialize};

use crate::malla::formas::{Forma, forma_de};
use crate::texturas::modelos::Recursos;

/// Forma de un bloque a efectos de sugerir parecidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FormaBloque {
    /// Cubo completo (piedra, tablones, troncos, lana, cristal…).
    Entero,
    Losa,
    Escalera,
    Muro,
    Valla,
    PuertaValla,
    Puerta,
    Trampilla,
    Boton,
    PlacaPresion,
    /// Paneles de cristal y barrotes (`*_pane`, `iron_bars`, `*_bars`).
    Panel,
    Alfombra,
    /// Cualquier otra cosa: no se mezcla con ninguna de las anteriores.
    Otro,
}

impl FormaBloque {
    /// Nombre en español para la interfaz.
    pub fn nombre(self) -> &'static str {
        match self {
            FormaBloque::Entero => "bloque entero",
            FormaBloque::Losa => "losa",
            FormaBloque::Escalera => "escalera",
            FormaBloque::Muro => "muro",
            FormaBloque::Valla => "valla",
            FormaBloque::PuertaValla => "puerta de valla",
            FormaBloque::Puerta => "puerta",
            FormaBloque::Trampilla => "trampilla",
            FormaBloque::Boton => "botón",
            FormaBloque::PlacaPresion => "placa de presión",
            FormaBloque::Panel => "panel",
            FormaBloque::Alfombra => "alfombra",
            FormaBloque::Otro => "otro",
        }
    }
}

/// Sufijos de id y su forma, en el orden en que se prueban (`_fence_gate`
/// antes que `_fence`; `_trapdoor` antes que `_door`).
pub const SUFIJOS: &[(&str, FormaBloque)] = &[
    ("_slab", FormaBloque::Losa),
    ("_stairs", FormaBloque::Escalera),
    ("_wall", FormaBloque::Muro),
    ("_fence_gate", FormaBloque::PuertaValla),
    ("_fence", FormaBloque::Valla),
    ("_trapdoor", FormaBloque::Trampilla),
    ("_door", FormaBloque::Puerta),
    ("_button", FormaBloque::Boton),
    ("_pressure_plate", FormaBloque::PlacaPresion),
    ("_pane", FormaBloque::Panel),
    ("_bars", FormaBloque::Panel),
    ("_carpet", FormaBloque::Alfombra),
];

/// Bloques que el visor dibuja como cubo pero no lo son (o no del todo).
/// Solo se usa sin jar.
pub const NO_CUBICOS: &[&str] = &[
    "azalea",
    "flowering_azalea",
    "cactus",
    "cake",
    "composter",
    "dirt_path",
    "farmland",
    "snow",
    "scaffolding",
    "lectern",
    "grindstone",
    "stonecutter",
    "enchanting_table",
    "end_portal_frame",
    "daylight_detector",
    "comparator",
    "repeater",
    "brewing_stand",
    "conduit",
    "dragon_egg",
    "pointed_dripstone",
    "melon_stem",
    "pumpkin_stem",
    "attached_melon_stem",
    "bamboo",
    "cocoa",
    "frogspawn",
    "turtle_egg",
    "sniffer_egg",
    "sea_pickle",
    "lever",
    "tripwire",
    "tripwire_hook",
    "nether_wart",
    "sweet_berry_bush",
    "chorus_plant",
    "chorus_flower",
    "spore_blossom",
    "hanging_roots",
    "kelp",
    "seagrass",
    "lily_of_the_valley",
    "poppy",
    "dandelion",
    "allium",
    "azure_bluet",
    "oxeye_daisy",
    "cornflower",
    "wither_rose",
    "blue_orchid",
    "torchflower",
    "open_eyeblossom",
    "closed_eyeblossom",
    "sunflower",
    "lilac",
    "rose_bush",
    "peony",
    "pitcher_plant",
    "wheat",
    "carrots",
    "potatoes",
    "beetroots",
    "end_rod",
    "lightning_rod",
    "ladder",
    "heavy_core",
    "decorated_pot",
    "big_dripleaf",
    "small_dripleaf",
    "copper_golem_statue",
    "sculk_sensor",
    "calibrated_sculk_sensor",
    "sculk_shrieker",
    "dried_ghast",
    "chiseled_bookshelf",
    "item_frame",
    "glow_item_frame",
    "water",
    "lava",
    "bubble_column",
    "fire",
    "soul_fire",
    "end_portal",
    "end_gateway",
    "nether_portal",
];

/// Fragmentos de nombre que delatan un bloque no cúbico (sin jar). Se
/// comprueban después de [`NO_CUBICOS`] y con excepciones (ver
/// [`parece_no_cubico`]).
pub const FRAGMENTOS_NO_CUBICOS: &[&str] = &[
    "torch",
    "lantern",
    "sign",
    "banner",
    "_head",
    "skull",
    "candle",
    "chain",
    "_bed",
    "anvil",
    "bell",
    "campfire",
    "chest",
    "cauldron",
    "hopper",
    "flower_pot",
    "potted_",
    "rail",
    "_sapling",
    "_bud",
    "amethyst_cluster",
    "_plant",
    "_crop",
    "vines",
    "vine",
    "lichen",
    "_vein",
    "grass",
    "fern",
    "bush",
    "petals",
    "_roots",
    "mushroom",
    "fungus",
    "coral",
    "sprouts",
    "_shelf",
    "leaf_litter",
    "wildflowers",
    "cactus_flower",
    "resin_clump",
    "redstone_wire",
    "pickle",
    "dripleaf",
    "shulker_box",
    "_stem",
];

/// ¿El nombre (sin espacio de nombres) delata un bloque no cúbico? Las
/// excepciones son cubos cuyo nombre contiene un fragmento de la lista:
/// bloques `*_block` (`brown_mushroom_block`, `dead_tube_coral_block`,
/// `grass_block`…), `mushroom_stem`, troncos del Nether (`crimson_stem`,
/// `stripped_warped_stem`), las raíces de mangle, `jack_o_lantern` y
/// `sea_lantern`.
pub fn parece_no_cubico(n: &str) -> bool {
    if NO_CUBICOS.contains(&n) {
        return true;
    }
    let excepcion = n.ends_with("_block")
        || n == "mushroom_stem"
        || n.ends_with("mangrove_roots")
        || n == "jack_o_lantern"
        || n == "sea_lantern"
        || n.ends_with("crimson_stem")
        || n.ends_with("warped_stem");
    !excepcion && FRAGMENTOS_NO_CUBICOS.iter().any(|f| n.contains(f))
}

/// Quita `minecraft:` y las propiedades.
fn nombre_corto(id: &str) -> &str {
    let id = id.split_once('[').map_or(id, |(n, _)| n);
    id.strip_prefix("minecraft:").unwrap_or(id)
}

/// Forma por el sufijo del id (`minecraft:oak_stairs` o `oak_stairs`), o
/// `None` si no tiene sufijo de forma.
pub fn forma_por_sufijo(id: &str) -> Option<FormaBloque> {
    let n = nombre_corto(id);
    if n == "iron_bars" {
        return Some(FormaBloque::Panel);
    }
    SUFIJOS
        .iter()
        .find(|(s, _)| n.ends_with(s))
        .map(|&(_, f)| f)
}

/// Forma sin jar: sufijo y, si no lo hay, la heurística de cubo del módulo.
/// Los bloques de mods son `Otro`.
pub fn forma_sin_jar(id: &str) -> FormaBloque {
    if let Some(f) = forma_por_sufijo(id) {
        return f;
    }
    let completo = id.split_once('[').map_or(id, |(n, _)| n);
    let Some(n) = completo.strip_prefix("minecraft:") else {
        return FormaBloque::Otro;
    };
    let cubo_en_visor = matches!(forma_de(completo), Forma::Cubo(_));
    if cubo_en_visor && !parece_no_cubico(n) {
        FormaBloque::Entero
    } else {
        FormaBloque::Otro
    }
}

/// Forma con los blockstates y modelos del jar: sufijo y, si no lo hay,
/// `Entero` solo si el estado por defecto (`props_defecto`) es un cubo
/// completo en sus modelos. `nombre` va sin espacio de nombres.
pub fn forma_con_recursos(
    recursos: &Recursos,
    nombre: &str,
    props_defecto: &[(&str, &str)],
) -> FormaBloque {
    if let Some(f) = forma_por_sufijo(nombre) {
        return f;
    }
    if recursos.es_cubo_completo(nombre, props_defecto) {
        FormaBloque::Entero
    } else {
        FormaBloque::Otro
    }
}
