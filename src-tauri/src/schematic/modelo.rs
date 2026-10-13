// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Modelo común al que se traducen los cuatro formatos de schematic.

use serde::Serialize;

/// Formato de origen del archivo, detectado por su contenido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Formato {
    Litematic,
    Sponge,
    Mcedit,
    Estructura,
}

impl Formato {
    /// Texto con el que se guarda en la base de datos y se manda al frontend.
    pub fn como_texto(self) -> &'static str {
        match self {
            Formato::Litematic => "litematic",
            Formato::Sponge => "sponge",
            Formato::Mcedit => "mcedit",
            Formato::Estructura => "estructura",
        }
    }

    pub fn desde_texto(texto: &str) -> Option<Self> {
        match texto {
            "litematic" => Some(Formato::Litematic),
            "sponge" => Some(Formato::Sponge),
            "mcedit" => Some(Formato::Mcedit),
            "estructura" => Some(Formato::Estructura),
            _ => None,
        }
    }

    /// Extensión habitual, para archivos que llegan sin extensión.
    pub fn extension_habitual(self) -> &'static str {
        match self {
            Formato::Litematic => "litematic",
            Formato::Sponge => "schem",
            Formato::Mcedit => "schematic",
            Formato::Estructura => "nbt",
        }
    }
}

/// Tamaño de una caja en bloques (siempre positivo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dimensiones {
    pub ancho: u32,
    pub alto: u32,
    pub largo: u32,
}

impl Dimensiones {
    /// Número de posiciones de la caja.
    pub fn volumen(&self) -> u64 {
        u64::from(self.ancho) * u64::from(self.alto) * u64::from(self.largo)
    }
}

/// Una región: caja con su paleta de block states y un índice por posición.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub nombre: String,
    /// Esquina mínima, en coordenadas absolutas del schematic.
    pub origen: [i32; 3],
    pub tamano: Dimensiones,
    /// Block states normalizados. Si hay posiciones vacías, el índice 0 es aire.
    pub paleta: Vec<String>,
    /// Índice en `paleta` de cada posición, en orden `(y*largo + z)*ancho + x`.
    #[serde(skip)]
    pub bloques: Vec<u32>,
    /// Datos de la región que el visor no interpreta pero que se conservan al
    /// reescribirla como .litematic. Solo los rellena el lector de .litematic.
    #[serde(skip)]
    pub extras: ExtrasRegion,
}

/// NBT crudo de una región .litematic: entidades de bloque, entidades y ticks
/// pendientes. `None` significa que el original no lo traía (o que venía de
/// otro formato); al escribir se sustituye por una lista vacía.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtrasRegion {
    pub entidades_bloque: Option<fastnbt::Value>,
    pub entidades: Option<fastnbt::Value>,
    pub ticks_bloque: Option<fastnbt::Value>,
    pub ticks_fluido: Option<fastnbt::Value>,
}

impl Region {
    /// Índice lineal de una posición relativa a la esquina mínima.
    pub fn indice(&self, x: u32, y: u32, z: u32) -> Option<usize> {
        let t = &self.tamano;
        if x >= t.ancho || y >= t.alto || z >= t.largo {
            return None;
        }
        let i =
            (u64::from(y) * u64::from(t.largo) + u64::from(z)) * u64::from(t.ancho) + u64::from(x);
        usize::try_from(i).ok()
    }

    /// Block state de una posición relativa a la esquina mínima.
    pub fn bloque(&self, x: u32, y: u32, z: u32) -> Option<&str> {
        let i = self.indice(x, y, z)?;
        let p = *self.bloques.get(i)? as usize;
        self.paleta.get(p).map(String::as_str)
    }

    /// Posiciones ocupadas por algo que no es aire.
    pub fn total_bloques(&self) -> u64 {
        let solidos: Vec<bool> = self.paleta.iter().map(|s| !es_aire(s)).collect();
        self.bloques
            .iter()
            .filter(|&&p| solidos.get(p as usize).copied().unwrap_or(false))
            .count() as u64
    }
}

/// Schematic ya traducido al modelo común.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Schematic {
    pub formato: Formato,
    pub nombre: Option<String>,
    pub autor: Option<String>,
    pub descripcion: Option<String>,
    pub version_datos: Option<i32>,
    /// Milisegundos desde 1970-01-01 UTC.
    pub fecha_creacion: Option<i64>,
    /// Caja que engloba todas las regiones.
    pub dimensiones: Dimensiones,
    pub regiones: Vec<Region>,
}

impl Schematic {
    /// Posiciones que no son aire, sumando todas las regiones.
    pub fn total_bloques(&self) -> u64 {
        self.regiones.iter().map(Region::total_bloques).sum()
    }
}

/// ¿Es uno de los tres tipos de aire de Minecraft?
pub fn es_aire(estado: &str) -> bool {
    let nombre = estado.split('[').next().unwrap_or(estado);
    matches!(
        nombre,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    )
}

/// Caja que engloba un conjunto de regiones (ancho, alto, largo).
pub fn caja_englobante(regiones: &[Region]) -> Dimensiones {
    if regiones.is_empty() {
        return Dimensiones {
            ancho: 0,
            alto: 0,
            largo: 0,
        };
    }
    let mut min = [i64::MAX; 3];
    let mut max = [i64::MIN; 3];
    for r in regiones {
        let t = [r.tamano.ancho, r.tamano.alto, r.tamano.largo];
        for eje in 0..3 {
            let a = i64::from(r.origen[eje]);
            min[eje] = min[eje].min(a);
            max[eje] = max[eje].max(a + i64::from(t[eje]));
        }
    }
    let lado = |eje: usize| u32::try_from(max[eje] - min[eje]).unwrap_or(u32::MAX);
    Dimensiones {
        ancho: lado(0),
        alto: lado(1),
        largo: lado(2),
    }
}
