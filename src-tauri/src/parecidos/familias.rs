// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Familias de material: los bloques que son «lo mismo» salvo la forma y el
//! acabado (`deepslate`, `cobbled_deepslate`, `polished_deepslate_stairs`,
//! `deepslate_brick_wall`… son la familia `deepslate`), y la **variante** de
//! cada uno dentro de su familia (`pulido`, `ladrillo`, `pulido+ladrillo`…),
//! que sirve para emparejar bloques entre familias distintas.
//!
//! Cómo se deduce la familia de un id (sin el `minecraft:`):
//! 1. Se quita el sufijo de forma ([`forma::SUFIJOS`]: `_slab`, `_stairs`,
//!    `_wall`…). Las alfombras de color pasan a la lana de su color
//!    (`white_carpet` → `white_wool`).
//! 2. Maderas ([`ESPECIES`]): tablones, troncos, madera y sus descortezados
//!    se agrupan por especie (`oak_planks`, `stripped_oak_log`, `oak_fence`
//!    → familia `oak`). Las formas de madera (`oak_stairs`) son de tablones.
//!    Hojas, brotes, carteles… no son madera: cada uno es su propia familia.
//! 3. El resto: se quitan las palabras de acabado ([`PALABRAS`]: `cobbled`,
//!    `polished`, `chiseled`, `cracked`, `smooth`, `cut`, `mossy`, `waxed`,
//!    `gilded`, `bricks`/`brick`, `tiles`/`tile`, `pillar`, `grate`, `bulb`, y
//!    `block` sin variante) estén donde estén; lo que queda es la familia y lo
//!    quitado, la variante (`cracked_polished_blackstone_bricks` →
//!    `blackstone`, `agrietado+pulido+ladrillo`).
//! 4. Excepciones ([`RENOMBRES`]): `bricks` (ladrillo de arcilla) → `bricks`;
//!    `nether_bricks` y `red_nether_bricks` son familias propias (no
//!    `nether`); `cobblestone` es la variante `adoquín` de `stone` (como
//!    `cobbled_deepslate` de `deepslate`); `dark_prismarine` es la variante
//!    `oscuro` de `prismarine`; las placas con peso son de hierro (`heavy`) y
//!    de oro (`light`). `quartz` (`quartz_block`, `quartz_bricks`,
//!    `smooth_quartz`…), `sandstone`, `red_sandstone`, `prismarine`,
//!    `end_stone` (`end_stone_bricks`), `purpur` y el cobre por grado de
//!    oxidación (`exposed_copper`, con `waxed` como variante `encerado`)
//!    salen solos de las reglas.
//!
//! Los ids de otros namespaces siguen las mismas reglas y la familia lleva el
//! namespace delante (`mod:granito`).

use serde::Serialize;

use super::forma::{self, FormaBloque};

/// Especies de madera, las compuestas antes (`dark_oak` antes que `oak`).
pub const ESPECIES: &[&str] = &[
    "dark_oak", "pale_oak", "oak", "spruce", "birch", "jungle", "acacia", "mangrove", "cherry",
    "bamboo", "crimson", "warped",
];

/// Colores de tinte (alfombras → lana de su color).
pub const TINTES: &[&str] = &[
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

/// Componentes de variante en su orden canónico, con su peso al comparar
/// variantes: los de patrón (ladrillo, baldosa) pesan más; los que apenas
/// cambian el aspecto (agrietado, adoquín, musgoso) menos.
pub const COMPONENTES: &[(&str, f64)] = &[
    ("encerado", 1.0),
    ("dorado", 1.0),
    ("oscuro", 1.0),
    ("musgoso", 0.75),
    ("agrietado", 0.5),
    ("cincelado", 1.0),
    ("adoquín", 0.5),
    ("pulido", 1.0),
    ("liso", 1.0),
    ("cortado", 1.0),
    ("ladrillo", 1.5),
    ("baldosa", 1.5),
    ("pilar", 1.0),
    ("rejilla", 1.0),
    ("bombilla", 1.0),
    ("descortezado", 1.0),
    ("tablones", 1.0),
    ("tronco", 1.0),
    ("madera", 1.0),
    ("mosaico", 1.0),
];

/// Palabras de acabado que se quitan del id y su componente (`None`: se
/// quita sin dar variante, como `block` en `quartz_block`).
pub const PALABRAS: &[(&str, Option<&str>)] = &[
    ("cobbled", Some("adoquín")),
    ("polished", Some("pulido")),
    ("chiseled", Some("cincelado")),
    ("cracked", Some("agrietado")),
    ("smooth", Some("liso")),
    ("cut", Some("cortado")),
    ("mossy", Some("musgoso")),
    ("waxed", Some("encerado")),
    ("gilded", Some("dorado")),
    ("bricks", Some("ladrillo")),
    ("brick", Some("ladrillo")),
    ("tiles", Some("baldosa")),
    ("tile", Some("baldosa")),
    ("pillar", Some("pilar")),
    ("grate", Some("rejilla")),
    ("bulb", Some("bombilla")),
    ("block", None),
];

/// Raíces que se renombran tras quitar las palabras: raíz → (familia,
/// componente que se añade a la variante).
pub const RENOMBRES: &[(&str, &str, Option<&str>)] = &[
    ("", "bricks", None),
    ("nether", "nether_bricks", None),
    ("red_nether", "red_nether_bricks", None),
    ("cobblestone", "stone", Some("adoquín")),
    ("dark_prismarine", "prismarine", Some("oscuro")),
    ("heavy_weighted", "iron", None),
    ("light_weighted", "gold", None),
];

/// Pares de componentes afines: cambiar uno por otro cuesta poco (los
/// ladrillos y las baldosas son ambos sillería; pulido y liso, acabados lisos).
pub const AFINES: &[(&str, &str)] = &[("ladrillo", "baldosa"), ("pulido", "liso")];
/// Lo que cuesta un par afín (frente a la suma de sus pesos).
const COSTE_AFIN: f64 = 0.25;

/// Variante de un bloque dentro de su familia: sus componentes en el orden
/// de [`COMPONENTES`]. Vacía = la base (`deepslate`, `oak_planks` no: los
/// tablones son `tablones`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Variante(Vec<&'static str>);

impl Variante {
    fn de(mut componentes: Vec<&'static str>) -> Self {
        let orden = |c: &&str| COMPONENTES.iter().position(|(k, _)| k == c);
        componentes.sort_by_key(orden);
        componentes.dedup();
        Variante(componentes)
    }

    pub fn componentes(&self) -> &[&'static str] {
        &self.0
    }

    /// `base` o los componentes unidos con `+` (`pulido+ladrillo`).
    pub fn nombre(&self) -> String {
        if self.0.is_empty() {
            "base".into()
        } else {
            self.0.join("+")
        }
    }

    /// Coste de cambiar esta variante por `otra` (menor = más equivalente):
    /// suma de pesos de lo que no comparten, con los pares [`AFINES`] a
    /// [`COSTE_AFIN`], menos la mitad del peso de lo que comparten (así
    /// `ladrillo` → `pulido+ladrillo` gana a `ladrillo` → base). Puede ser
    /// negativo; dos variantes iguales dan el mínimo posible para esa variante.
    pub fn coste(&self, otra: &Variante) -> f64 {
        let peso = |c: &str| {
            COMPONENTES
                .iter()
                .find(|(k, _)| *k == c)
                .map_or(1.0, |(_, p)| *p)
        };
        let mut solo_a: Vec<&str> = self
            .0
            .iter()
            .copied()
            .filter(|c| !otra.0.contains(c))
            .collect();
        let mut solo_b: Vec<&str> = otra
            .0
            .iter()
            .copied()
            .filter(|c| !self.0.contains(c))
            .collect();
        let comunes: f64 = self
            .0
            .iter()
            .filter(|c| otra.0.contains(c))
            .map(|c| peso(c))
            .sum();
        let mut coste = 0.0;
        for (x, y) in AFINES {
            for (p, q) in [(x, y), (y, x)] {
                if let (Some(i), Some(j)) = (
                    solo_a.iter().position(|c| c == p),
                    solo_b.iter().position(|c| c == q),
                ) {
                    solo_a.remove(i);
                    solo_b.remove(j);
                    coste += COSTE_AFIN;
                }
            }
        }
        coste += solo_a.iter().chain(&solo_b).map(|c| peso(c)).sum::<f64>();
        coste - comunes / 2.0
    }

    /// Parecido entre 0 y 1 (1 = equivalentes).
    pub fn parecido(&self, otra: &Variante) -> f64 {
        1.0 / (1.0 + self.coste(otra).max(0.0))
    }
}

impl Serialize for Variante {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

/// Familia y variante de un bloque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pertenencia {
    pub familia: String,
    pub variante: Variante,
}

/// Familia y variante de un id o block state (se ignoran las propiedades).
pub fn familia_de(id_o_estado: &str) -> Pertenencia {
    let id = id_o_estado.split_once('[').map_or(id_o_estado, |(n, _)| n);
    let (ns, ruta) = match id.split_once(':') {
        Some(("minecraft", r)) => (None, r),
        Some((ns, r)) => (Some(ns), r),
        None => (None, id),
    };
    let (familia, variante) = familia_de_ruta(ruta);
    Pertenencia {
        familia: match ns {
            Some(ns) => format!("{ns}:{familia}"),
            None => familia,
        },
        variante,
    }
}

fn familia_de_ruta(ruta: &str) -> (String, Variante) {
    let (m, con_forma) = match forma::SUFIJOS
        .iter()
        .find_map(|(s, _)| ruta.strip_suffix(s))
    {
        Some(m) if !m.is_empty() => (m, true),
        _ => (ruta, false),
    };
    // Alfombras de color → la lana de su color.
    if con_forma && ruta.ends_with("_carpet") && TINTES.contains(&m) {
        return (format!("{m}_wool"), Variante::default());
    }
    if let Some(r) = madera(m, con_forma) {
        return r;
    }
    // El bambú planta no es la familia de madera `bamboo`.
    if m == "bamboo" {
        return ("bamboo_plant".into(), Variante::default());
    }

    let mut componentes = Vec::new();
    let mut quitada = false;
    let resto: Vec<&str> = m
        .split('_')
        .filter(|t| match PALABRAS.iter().find(|(p, _)| p == t) {
            Some((_, c)) => {
                componentes.extend(*c);
                quitada = true;
                false
            }
            None => true,
        })
        .collect();
    let raiz = resto.join("_");
    if let Some((_, familia, extra)) = RENOMBRES.iter().find(|(r, _, _)| *r == raiz)
        && (quitada || !raiz.is_empty())
    {
        componentes.extend(*extra);
        return (familia.to_string(), Variante::de(componentes));
    }
    if raiz.is_empty() {
        return (m.to_string(), Variante::default());
    }
    (raiz, Variante::de(componentes))
}

/// Familia de madera (la especie) y variante, si `m` es madera.
fn madera(m: &str, con_forma: bool) -> Option<(String, Variante)> {
    let (sin, descortezado) = match m.strip_prefix("stripped_") {
        Some(s) => (s, true),
        None => (m, false),
    };
    let especie = ESPECIES
        .iter()
        .find(|e| sin == **e || sin.strip_prefix(**e).is_some_and(|r| r.starts_with('_')))?;
    let sufijo = &sin[especie.len()..];
    let parte = match sufijo {
        "" if con_forma => "tablones",
        "_planks" => "tablones",
        "_log" | "_stem" | "_block" => "tronco",
        "_wood" | "_hyphae" => "madera",
        "_mosaic" => "mosaico",
        _ => return None,
    };
    if descortezado && !matches!(parte, "tronco" | "madera") {
        return None;
    }
    let mut c = vec![parte];
    if descortezado {
        c.push("descortezado");
    }
    Some((especie.to_string(), Variante::de(c)))
}

/// Un bloque del catálogo dentro de su familia.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Miembro {
    pub id: String,
    pub forma: FormaBloque,
    pub variante: Variante,
    pub sugerible: bool,
}

/// Prefijos que se quitan para el «id base» de los bloques de forma
/// [`FormaBloque::Otro`]: tinte, especie de madera, oxidación del cobre y
/// `soul_` (`white_candle` y `red_candle` → `candle`; `torch` y `soul_torch`
/// → `torch`; `oak_sapling` y `birch_sapling` → `sapling`).
pub fn base_otro(id: &str) -> String {
    let id = id.split_once('[').map_or(id, |(n, _)| n);
    let mut ruta = id.strip_prefix("minecraft:").unwrap_or(id);
    loop {
        let antes = ruta;
        for p in ["waxed", "exposed", "weathered", "oxidized", "soul"]
            .iter()
            .chain(TINTES)
            .chain(ESPECIES)
        {
            if let Some(r) = ruta.strip_prefix(p).and_then(|r| r.strip_prefix('_'))
                && !r.is_empty()
            {
                ruta = r;
                break;
            }
        }
        if ruta == antes {
            return ruta.to_string();
        }
    }
}
