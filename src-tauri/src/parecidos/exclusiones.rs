// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Qué bloques se pueden sugerir (como origen o como destino de un cambio).
//!
//! No se sugieren nunca:
//! - [`AIRES`] y [`FLUIDOS`].
//! - [`TECNICOS`]: bloques de comandos, estructuras, barreras, luz…
//! - [`SIN_OBJETO`] y lo que empieza por [`PREFIJOS_SIN_OBJETO`] o acaba en
//!   [`SUFIJOS_SIN_OBJETO`]: bloques que solo existen colocados (macetas con
//!   planta, antorchas de pared, calderos llenos, partes de plantas…) y los
//!   infestados (idénticos a su bloque normal: la distancia de color sería 0).
//! - Por defecto, [`CON_DATOS`] y lo que acaba en [`SUFIJOS_CON_DATOS`]:
//!   bloques con entidad de bloque cuyo contenido (inventario, texto,
//!   patrones…) se perdería al cambiarlos. [`motivo_exclusion`] los marca
//!   como [`Exclusion::ConDatos`] para que quien quiera pueda admitirlos.
//!
//! Los ids van con o sin `minecraft:`; los de mods no se excluyen aquí (no
//! están en el catálogo, que es solo vanilla).

use serde::Serialize;

/// Por qué un bloque no se sugiere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Exclusion {
    Aire,
    Fluido,
    Tecnico,
    /// No existe como objeto o es un duplicado visual de otro bloque.
    SinObjeto,
    /// Tiene entidad de bloque con datos que se perderían.
    ConDatos,
}

pub const AIRES: &[&str] = &["air", "cave_air", "void_air"];

pub const FLUIDOS: &[&str] = &["water", "lava", "bubble_column"];

pub const TECNICOS: &[&str] = &[
    "barrier",
    "structure_void",
    "light",
    "jigsaw",
    "command_block",
    "chain_command_block",
    "repeating_command_block",
    "structure_block",
    "test_block",
    "test_instance_block",
    "moving_piston",
    "piston_head",
    "end_portal",
    "end_gateway",
    "nether_portal",
    "fire",
    "soul_fire",
    "frosted_ice",
];

pub const SIN_OBJETO: &[&str] = &[
    "water_cauldron",
    "lava_cauldron",
    "powder_snow_cauldron",
    "attached_melon_stem",
    "attached_pumpkin_stem",
    "melon_stem",
    "pumpkin_stem",
    "kelp_plant",
    "cave_vines_plant",
    "twisting_vines_plant",
    "weeping_vines_plant",
    "tall_seagrass",
    "big_dripleaf_stem",
    "bamboo_sapling",
    "tripwire",
    "redstone_wire",
    "wall_torch",
    "carrots",
    "potatoes",
    "beetroots",
    "cocoa",
    "sweet_berry_bush",
    "torchflower_crop",
    "pitcher_crop",
];

pub const PREFIJOS_SIN_OBJETO: &[&str] = &["potted_", "infested_"];

/// Variantes de pared (el objeto es la versión de suelo).
pub const SUFIJOS_SIN_OBJETO: &[&str] = &[
    "_wall_torch",
    "_wall_sign",
    "_wall_hanging_sign",
    "_wall_banner",
    "_wall_head",
    "_wall_skull",
    "_wall_fan",
];

pub const CON_DATOS: &[&str] = &[
    "chest",
    "trapped_chest",
    "ender_chest",
    "barrel",
    "furnace",
    "blast_furnace",
    "smoker",
    "hopper",
    "dropper",
    "dispenser",
    "crafter",
    "brewing_stand",
    "lectern",
    "jukebox",
    "chiseled_bookshelf",
    "decorated_pot",
    "beehive",
    "bee_nest",
    "spawner",
    "trial_spawner",
    "vault",
    "beacon",
    "conduit",
    "campfire",
    "soul_campfire",
    "suspicious_sand",
    "suspicious_gravel",
    "player_head",
    "shulker_box",
    "creaking_heart",
    "sculk_sensor",
    "calibrated_sculk_sensor",
    "sculk_shrieker",
    "sculk_catalyst",
];

/// Sufijos de bloques con datos: cajas de shulker de color, carteles,
/// estandartes, cabezas, estantes y estatuas.
pub const SUFIJOS_CON_DATOS: &[&str] = &[
    "_shulker_box",
    "_sign",
    "_banner",
    "_head",
    "_skull",
    "_shelf",
    "copper_golem_statue",
];

/// Por qué no se sugiere un bloque, o `None` si se puede sugerir.
pub fn motivo_exclusion(id: &str) -> Option<Exclusion> {
    let id = id.split_once('[').map_or(id, |(n, _)| n);
    let n = id.strip_prefix("minecraft:").unwrap_or(id);
    if AIRES.contains(&n) {
        Some(Exclusion::Aire)
    } else if FLUIDOS.contains(&n) {
        Some(Exclusion::Fluido)
    } else if TECNICOS.contains(&n) {
        Some(Exclusion::Tecnico)
    } else if SIN_OBJETO.contains(&n)
        || PREFIJOS_SIN_OBJETO.iter().any(|p| n.starts_with(p))
        || SUFIJOS_SIN_OBJETO.iter().any(|s| n.ends_with(s))
    {
        Some(Exclusion::SinObjeto)
    } else if CON_DATOS.contains(&n) || SUFIJOS_CON_DATOS.iter().any(|s| n.ends_with(s)) {
        Some(Exclusion::ConDatos)
    } else {
        None
    }
}

/// ¿Se puede sugerir este bloque? Con la política por defecto: tampoco los
/// que tienen datos de entidad de bloque.
pub fn es_sugerible(id: &str) -> bool {
    motivo_exclusion(id).is_none()
}
