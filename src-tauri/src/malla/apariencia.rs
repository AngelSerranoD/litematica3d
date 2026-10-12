// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Caras, tabla de materiales y puntos de enganche de la apariencia.
//!
//! Un material es la pareja (estado visual, cara). Su color de respaldo sale
//! de la tabla de colores medios `datos/colores_bloques.json` ([`color_de`]) y
//! su textura, de una [`FuenteTexturas`] (el atlas de la versión de Minecraft
//! elegida, ver `crate::texturas`) mediante [`textura_de`].

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// Cara de un bloque. Las seis primeras coinciden con el índice de normal que
/// se manda por vértice (ver `docs/visor.md`); `Cruz` es la de los planos
/// cruzados de las plantas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Cara {
    /// +Y
    Arriba = 0,
    /// -Y
    Abajo = 1,
    /// -Z
    Norte = 2,
    /// +Z
    Sur = 3,
    /// -X
    Oeste = 4,
    /// +X
    Este = 5,
    /// Planos cruzados (plantas): normales diagonales 6..9.
    Cruz = 6,
}

impl Cara {
    /// Las seis caras de un cubo, en el orden de su índice.
    pub const CUBO: [Cara; 6] = [
        Cara::Arriba,
        Cara::Abajo,
        Cara::Norte,
        Cara::Sur,
        Cara::Oeste,
        Cara::Este,
    ];

    /// Eje perpendicular (0 = X, 1 = Y, 2 = Z). `Cruz` no tiene: devuelve 1.
    pub fn eje(self) -> usize {
        match self {
            Cara::Oeste | Cara::Este => 0,
            Cara::Arriba | Cara::Abajo | Cara::Cruz => 1,
            Cara::Norte | Cara::Sur => 2,
        }
    }

    /// ¿Mira hacia el sentido positivo de su eje?
    pub fn positiva(self) -> bool {
        matches!(self, Cara::Arriba | Cara::Sur | Cara::Este)
    }

    /// Cara del otro lado (la del vecino que toca a esta). `Cruz` → `Cruz`.
    pub fn opuesta(self) -> Cara {
        match self {
            Cara::Arriba => Cara::Abajo,
            Cara::Abajo => Cara::Arriba,
            Cara::Norte => Cara::Sur,
            Cara::Sur => Cara::Norte,
            Cara::Oeste => Cara::Este,
            Cara::Este => Cara::Oeste,
            Cara::Cruz => Cara::Cruz,
        }
    }

    /// Desplazamiento hasta el vecino que toca esta cara.
    pub fn delta(self) -> [i32; 3] {
        let mut d = [0; 3];
        d[self.eje()] = if self.positiva() { 1 } else { -1 };
        d
    }
}

/// Cómo se pinta un material: decide la pasada de render en el frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Tipo {
    /// Sólido, sin transparencia.
    Opaco,
    /// Con huecos de alfa 0/1 (hojas, plantas, raíles): alphaTest, sin mezcla.
    Recorte,
    /// Semitransparente (cristal, agua, hielo): mezcla alfa, se pinta al final.
    Translucido,
}

/// Fila de la tabla de materiales que recibe el frontend.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    /// Block state sin las propiedades que no cambian el aspecto
    /// (ver [`clave_visual`]).
    pub estado: String,
    /// Identificador sin propiedades: `minecraft:oak_log`.
    pub nombre: String,
    pub cara: Cara,
    pub tipo: Tipo,
    /// Color de respaldo cuando no hay textura. Con textura, es su color medio.
    pub color: [u8; 3],
    /// Celda del atlas de texturas, si la hay.
    pub textura: Option<u32>,
    /// Giro de la textura en la cara, en grados (0, 90, 180, 270), en sentido
    /// horario mirando la cara desde fuera (troncos tumbados, etc.).
    pub rotacion: u16,
}

/// Textura de una cara ya resuelta contra el atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TexturaCara {
    /// Celda del atlas.
    pub celda: u32,
    /// Giro en grados (ver [`Material::rotacion`]).
    pub rotacion: u16,
    /// Color medio de la textura (con el tinte aplicado).
    pub color: [u8; 3],
}

/// Datos del atlas que necesita el shader; viajan en la cabecera JSON de la
/// malla (`atlas`, ver `docs/visor.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatosAtlas {
    /// Versión de Minecraft de la que salen las texturas. Si no coincide con
    /// la del PNG que tiene cargado el frontend, hay que volver a pedirlo.
    pub version: String,
    /// Tamaño del PNG en píxeles (potencias de dos).
    pub ancho: u32,
    pub alto: u32,
    pub celdas_por_fila: u32,
    /// Lado de la celda en píxeles (textura + relleno a cada lado).
    pub tam_celda: u32,
    /// Lado de la textura dentro de la celda, en píxeles.
    pub tam_textura: u32,
    /// Píxeles de relleno a cada lado de la textura (la textura repetida).
    pub relleno: u32,
    /// Lado de la celda en UV del atlas: `[tamCelda / ancho, tamCelda / alto]`.
    pub celda_uv: [f32; 2],
    /// Niveles de mipmap útiles sin sangrado entre celdas: `log2(relleno)`.
    pub niveles_mip: u32,
}

/// De dónde salen las texturas de los materiales (el atlas activo).
pub trait FuenteTexturas: Sync {
    /// Textura de un estado visual en una cara, si la hay.
    fn textura(&self, estado: &str, cara: Cara) -> Option<TexturaCara>;
    /// Datos del atlas para la cabecera de la malla.
    fn datos_atlas(&self) -> DatosAtlas;
}

/// Propiedades que nunca cambian el aspecto de un bloque y que, si se
/// conservaran, partirían en materiales distintos bloques que se ven iguales
/// (hojas con `distance` distinta, escaleras con o sin agua…).
const PROPIEDADES_INVISIBLES: [&str; 3] = ["distance", "persistent", "waterlogged"];

/// Nombre sin propiedades: `minecraft:oak_stairs[facing=north]` → `minecraft:oak_stairs`.
pub fn nombre_de(estado: &str) -> &str {
    estado.split_once('[').map_or(estado, |(n, _)| n)
}

/// Valor de una propiedad del block state, si la tiene.
pub fn propiedad<'a>(estado: &'a str, clave: &str) -> Option<&'a str> {
    let (_, resto) = estado.split_once('[')?;
    let resto = resto.strip_suffix(']').unwrap_or(resto);
    resto.split(',').find_map(|par| {
        let (k, v) = par.split_once('=')?;
        (k == clave).then_some(v)
    })
}

/// Estado que identifica el aspecto: quita [`PROPIEDADES_INVISIBLES`] y, en
/// los fluidos, `level` (el agua se dibuja siempre como bloque lleno).
pub fn clave_visual(estado: &str) -> String {
    let nombre = nombre_de(estado);
    let Some((_, resto)) = estado.split_once('[') else {
        return estado.to_string();
    };
    let fluido = matches!(nombre, "minecraft:water" | "minecraft:lava");
    let resto = resto.strip_suffix(']').unwrap_or(resto);
    let props: Vec<&str> = resto
        .split(',')
        .filter(|par| {
            let k = par.split_once('=').map_or(*par, |(k, _)| k);
            !PROPIEDADES_INVISIBLES.contains(&k) && !(fluido && k == "level")
        })
        .collect();
    if props.is_empty() {
        nombre.to_string()
    } else {
        format!("{nombre}[{}]", props.join(","))
    }
}

/// Tabla de colores medios incluida en el binario (datos derivados de las
/// texturas vanilla; la genera la prueba ignorada
/// `texturas::pruebas::generar_colores_bloques`).
const JSON_COLORES: &str = include_str!("../../datos/colores_bloques.json");

/// Versión del esquema de `colores_bloques.json`.
pub const ESQUEMA_COLORES: u32 = 1;

/// Contenido de `colores_bloques.json`.
#[derive(Debug, Serialize, Deserialize)]
pub struct TablaColores {
    pub esquema: u32,
    /// Versión de Minecraft de la que salen los colores.
    pub version: Option<String>,
    /// `minecraft:stone` → RGB.
    pub colores: std::collections::BTreeMap<String, [u8; 3]>,
}

static COLORES: LazyLock<HashMap<String, [u8; 3]>> =
    LazyLock::new(
        || match serde_json::from_str::<TablaColores>(JSON_COLORES) {
            Ok(t) if t.esquema == ESQUEMA_COLORES => t.colores.into_iter().collect(),
            _ => HashMap::new(),
        },
    );

/// PUNTO DE ENGANCHE (colores): color medio de un bloque por su nombre sin
/// propiedades, de la tabla `colores_bloques.json`. Lo que no está en ella
/// (bloques de mods, IDs desconocidos) cae a [`color_hash`].
pub fn color_de(nombre: &str) -> [u8; 3] {
    COLORES
        .get(nombre)
        .copied()
        .unwrap_or_else(|| color_hash(nombre))
}

/// Color estable a partir del nombre: FNV-1a llevado a un tono con saturación
/// y luz moderadas.
pub fn color_hash(nombre: &str) -> [u8; 3] {
    let mut h: u32 = 0x811c_9dc5;
    for b in nombre.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    let tono = (h % 360) as f32;
    let saturacion = 0.35 + ((h >> 9) % 30) as f32 / 100.0;
    let luz = 0.40 + ((h >> 17) % 25) as f32 / 100.0;
    hsl_a_rgb(tono, saturacion, luz)
}

/// PUNTO DE ENGANCHE (texturas): celda del atlas para un estado visual y una
/// cara, o `None` si no hay atlas cargado o el bloque no tiene textura (se
/// usa el color).
pub fn textura_de(
    fuente: Option<&dyn FuenteTexturas>,
    estado: &str,
    cara: Cara,
) -> Option<TexturaCara> {
    fuente?.textura(estado, cara)
}

fn hsl_a_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let canal = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    [canal(r), canal(g), canal(b)]
}

/// Máximo de materiales: el índice viaja como u16 por vértice.
pub const MAX_MATERIALES: usize = u16::MAX as usize + 1;

/// Tabla de materiales en construcción, sin repetidos.
#[derive(Default)]
pub struct TablaMateriales<'a> {
    indices: HashMap<(String, Cara), u16>,
    pub materiales: Vec<Material>,
    /// Atlas activo, si lo hay.
    pub fuente: Option<&'a dyn FuenteTexturas>,
}

impl<'a> TablaMateriales<'a> {
    /// Tabla vacía que toma las texturas de `fuente`.
    pub fn con_texturas(fuente: Option<&'a dyn FuenteTexturas>) -> Self {
        TablaMateriales {
            fuente,
            ..Default::default()
        }
    }

    /// Índice del material (estado visual, cara); lo crea si no existe.
    /// `None` si la tabla ya está llena.
    pub fn indice(&mut self, clave: &str, cara: Cara, tipo: Tipo) -> Option<u16> {
        if let Some(&i) = self.indices.get(&(clave.to_string(), cara)) {
            return Some(i);
        }
        if self.materiales.len() >= MAX_MATERIALES {
            return None;
        }
        let i = self.materiales.len() as u16;
        let nombre = nombre_de(clave);
        let textura = textura_de(self.fuente, clave, cara);
        self.materiales.push(Material {
            estado: clave.to_string(),
            nombre: nombre.to_string(),
            cara,
            tipo,
            color: textura.map_or_else(|| color_de(nombre), |t| t.color),
            textura: textura.map(|t| t.celda),
            rotacion: textura.map_or(0, |t| t.rotacion),
        });
        self.indices.insert((clave.to_string(), cara), i);
        Some(i)
    }
}
