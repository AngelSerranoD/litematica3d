// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Escritura del modelo común como .litematic (NBT comprimido con gzip), el
//! formato del mod Litematica, versión 6 del esquema.
//!
//! Qué se conserva:
//! - Bloques, paletas, posición y tamaño de cada región, nombre, autor,
//!   descripción, fecha de creación y `MinecraftDataVersion`.
//! - `TileEntities`, `Entities`, `PendingBlockTicks` y `PendingFluidTicks` tal
//!   cual venían, pero solo si el original era un .litematic (el lector los
//!   guarda en [`ExtrasRegion`]). Si el original era Sponge, MCEdit o una
//!   estructura, esos datos no llegan al modelo y se escriben listas vacías:
//!   cofres con contenido, carteles, entidades… se pierden.
//!
//! Qué no se conserva: la miniatura `PreviewImageData` y la etiqueta
//! `SubVersion` (Litematica las trata como opcionales). Los tamaños se escriben
//! siempre positivos, con `Position` en la esquina mínima: es equivalente para
//! Litematica, porque guarda bloques, entidades y ticks relativos a esa esquina.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::Compression;
use flate2::write::GzEncoder;
use serde::Serialize;
use serde::ser::SerializeMap;

use super::litematic::bits_por_bloque;
use super::modelo::{ExtrasRegion, Region, Schematic, caja_englobante};

/// Versión del esquema .litematic que se escribe.
pub const VERSION_LITEMATIC: i32 = 6;

/// `MinecraftDataVersion` cuando ni el schematic ni el llamador dan una:
/// 3955 es Minecraft 1.21.1.
pub const VERSION_DATOS_POR_DEFECTO: i32 = 3955;

/// Ajustes de la escritura.
#[derive(Debug, Clone, Copy, Default)]
pub struct OpcionesLitematic {
    /// `MinecraftDataVersion` que se usa si el schematic no trae la suya (p. ej.
    /// la de la versión de texturas activa). Si tampoco hay, se usa
    /// [`VERSION_DATOS_POR_DEFECTO`].
    pub version_datos: Option<i32>,
    /// Instante de la modificación, en milisegundos desde 1970-01-01 UTC. Si
    /// falta se usa el reloj del sistema (las pruebas lo fijan).
    pub ahora_ms: Option<i64>,
}

/// Errores al escribir un .litematic.
#[derive(Debug, thiserror::Error)]
pub enum ErrorEscritura {
    /// El modelo no es coherente (bloques fuera de la paleta, tamaños…).
    #[error("{0}")]
    Invalido(String),
    /// Fallo del codificador NBT.
    #[error("no se pudo codificar el NBT ({0})")]
    Nbt(String),
    /// Fallo al escribir los bytes.
    #[error("no se pudo escribir el archivo ({0})")]
    Io(#[from] std::io::Error),
}

/// Serializa el schematic a los bytes de un archivo .litematic.
pub fn escribir_litematic(
    schematic: &Schematic,
    opciones: &OpcionesLitematic,
) -> Result<Vec<u8>, ErrorEscritura> {
    escribir_litematic_en(schematic, opciones, Vec::new())
}

/// Serializa el schematic como .litematic en `destino` y lo devuelve con el
/// gzip ya cerrado (para que el llamador pueda, p. ej., hacer `sync_all` de un
/// archivo temporal antes de renombrarlo).
pub fn escribir_litematic_en<W: Write>(
    schematic: &Schematic,
    opciones: &OpcionesLitematic,
    destino: W,
) -> Result<W, ErrorEscritura> {
    let archivo = preparar(schematic, opciones)?;
    let mut gz = GzEncoder::new(destino, Compression::default());
    fastnbt::to_writer(&mut gz, &archivo).map_err(|e| ErrorEscritura::Nbt(e.to_string()))?;
    Ok(gz.finish()?)
}

// ---------------------------------------------------------------- estructura NBT

#[derive(Serialize)]
struct ArchivoSalida<'a> {
    #[serde(rename = "MinecraftDataVersion")]
    version_datos: i32,
    #[serde(rename = "Version")]
    version: i32,
    #[serde(rename = "Metadata")]
    metadatos: MetadatosSalida<'a>,
    #[serde(rename = "Regions")]
    regiones: RegionesSalida<'a>,
}

#[derive(Serialize)]
struct MetadatosSalida<'a> {
    #[serde(rename = "Name")]
    nombre: &'a str,
    #[serde(rename = "Author")]
    autor: &'a str,
    #[serde(rename = "Description")]
    descripcion: &'a str,
    #[serde(rename = "TimeCreated")]
    creado: i64,
    #[serde(rename = "TimeModified")]
    modificado: i64,
    #[serde(rename = "EnclosingSize")]
    caja: Vector,
    #[serde(rename = "TotalBlocks")]
    total_bloques: i32,
    #[serde(rename = "TotalVolume")]
    volumen_total: i32,
    #[serde(rename = "RegionCount")]
    num_regiones: i32,
}

#[derive(Serialize)]
struct Vector {
    x: i32,
    y: i32,
    z: i32,
}

/// Compound de regiones en el mismo orden que el modelo (un HashMap lo
/// barajaría y Litematica las lista en el orden del archivo).
struct RegionesSalida<'a>(Vec<(String, RegionSalida<'a>)>);

impl Serialize for RegionesSalida<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut mapa = s.serialize_map(Some(self.0.len()))?;
        for (nombre, region) in &self.0 {
            mapa.serialize_entry(nombre, region)?;
        }
        mapa.end()
    }
}

#[derive(Serialize)]
struct RegionSalida<'a> {
    #[serde(rename = "Position")]
    posicion: Vector,
    #[serde(rename = "Size")]
    tamano: Vector,
    #[serde(rename = "BlockStatePalette")]
    paleta: Vec<EntradaSalida<'a>>,
    #[serde(rename = "BlockStates")]
    estados: fastnbt::LongArray,
    #[serde(rename = "TileEntities")]
    entidades_bloque: &'a fastnbt::Value,
    #[serde(rename = "Entities")]
    entidades: &'a fastnbt::Value,
    #[serde(rename = "PendingBlockTicks")]
    ticks_bloque: &'a fastnbt::Value,
    #[serde(rename = "PendingFluidTicks")]
    ticks_fluido: &'a fastnbt::Value,
}

#[derive(Serialize)]
struct EntradaSalida<'a> {
    #[serde(rename = "Name")]
    nombre: &'a str,
    #[serde(rename = "Properties", skip_serializing_if = "Option::is_none")]
    propiedades: Option<BTreeMap<&'a str, &'a str>>,
}

/// Lista NBT vacía para los datos que el modelo no trae.
static LISTA_VACIA: fastnbt::Value = fastnbt::Value::List(Vec::new());

// ---------------------------------------------------------------- traducción

fn preparar<'a>(
    s: &'a Schematic,
    opciones: &OpcionesLitematic,
) -> Result<ArchivoSalida<'a>, ErrorEscritura> {
    if s.regiones.is_empty() {
        return Err(ErrorEscritura::Invalido(
            "el schematic no tiene ninguna región".into(),
        ));
    }
    let ahora = opciones.ahora_ms.unwrap_or_else(ahora_ms);

    let mut usados = HashSet::new();
    let mut regiones = Vec::with_capacity(s.regiones.len());
    for r in &s.regiones {
        let nombre = nombre_unico(&r.nombre, &mut usados);
        regiones.push((nombre, preparar_region(r)?));
    }

    let caja = caja_englobante(&s.regiones);
    let volumen: u64 = s.regiones.iter().map(|r| r.tamano.volumen()).sum();
    Ok(ArchivoSalida {
        version_datos: s
            .version_datos
            .or(opciones.version_datos)
            .unwrap_or(VERSION_DATOS_POR_DEFECTO),
        version: VERSION_LITEMATIC,
        metadatos: MetadatosSalida {
            nombre: s.nombre.as_deref().unwrap_or(""),
            autor: s.autor.as_deref().unwrap_or(""),
            descripcion: s.descripcion.as_deref().unwrap_or(""),
            creado: s.fecha_creacion.unwrap_or(ahora),
            modificado: ahora,
            caja: Vector {
                x: a_int(caja.ancho)?,
                y: a_int(caja.alto)?,
                z: a_int(caja.largo)?,
            },
            total_bloques: a_int(s.total_bloques())?,
            volumen_total: a_int(volumen)?,
            num_regiones: a_int(s.regiones.len())?,
        },
        regiones: RegionesSalida(regiones),
    })
}

fn preparar_region(r: &Region) -> Result<RegionSalida<'_>, ErrorEscritura> {
    let que = format!("la región «{}»", r.nombre);
    let t = &r.tamano;
    if t.ancho == 0 || t.alto == 0 || t.largo == 0 {
        return Err(ErrorEscritura::Invalido(format!("{que} está vacía")));
    }
    let volumen = t.volumen();
    if r.bloques.len() as u64 != volumen {
        return Err(ErrorEscritura::Invalido(format!(
            "{que} no tiene un bloque por posición"
        )));
    }

    // Paleta de salida: aire en el 0 (como Litematica) y luego solo los
    // estados que se usan, sin repetir.
    let mut usado = vec![false; r.paleta.len()];
    for &p in &r.bloques {
        match usado.get_mut(p as usize) {
            Some(u) => *u = true,
            None => {
                return Err(ErrorEscritura::Invalido(format!(
                    "{que} usa un bloque que no está en su paleta"
                )));
            }
        }
    }
    let mut paleta: Vec<&str> = vec!["minecraft:air"];
    let mut indices: HashMap<&str, u32> = HashMap::from([("minecraft:air", 0)]);
    let mut remapeo = vec![0u32; r.paleta.len()];
    for (i, estado) in r.paleta.iter().enumerate() {
        if !usado[i] {
            continue;
        }
        remapeo[i] = *indices.entry(estado.as_str()).or_insert_with(|| {
            paleta.push(estado.as_str());
            (paleta.len() - 1) as u32
        });
    }

    let bits = bits_por_bloque(paleta.len());
    let estados = empaquetar(
        r.bloques.iter().map(|&p| u64::from(remapeo[p as usize])),
        r.bloques.len(),
        bits,
    )
    .ok_or_else(|| {
        ErrorEscritura::Invalido(format!("no hay memoria suficiente para guardar {que}"))
    })?;

    let ExtrasRegion {
        entidades_bloque,
        entidades,
        ticks_bloque,
        ticks_fluido,
    } = &r.extras;
    Ok(RegionSalida {
        posicion: Vector {
            x: r.origen[0],
            y: r.origen[1],
            z: r.origen[2],
        },
        tamano: Vector {
            x: a_int(t.ancho)?,
            y: a_int(t.alto)?,
            z: a_int(t.largo)?,
        },
        paleta: paleta.into_iter().map(entrada_paleta).collect(),
        estados: fastnbt::LongArray::new(estados),
        entidades_bloque: o_lista_vacia(entidades_bloque),
        entidades: o_lista_vacia(entidades),
        ticks_bloque: o_lista_vacia(ticks_bloque),
        ticks_fluido: o_lista_vacia(ticks_fluido),
    })
}

fn o_lista_vacia(v: &Option<fastnbt::Value>) -> &fastnbt::Value {
    v.as_ref().unwrap_or(&LISTA_VACIA)
}

/// Separa `ns:nombre[a=1,b=2]` en `Name` y `Properties`.
fn entrada_paleta(estado: &str) -> EntradaSalida<'_> {
    let Some((nombre, resto)) = estado.split_once('[') else {
        return EntradaSalida {
            nombre: estado,
            propiedades: None,
        };
    };
    let resto = resto.strip_suffix(']').unwrap_or(resto);
    let propiedades: BTreeMap<&str, &str> = resto
        .split(',')
        .filter(|par| !par.is_empty())
        .map(|par| par.split_once('=').unwrap_or((par, "")))
        .collect();
    EntradaSalida {
        nombre,
        propiedades: (!propiedades.is_empty()).then_some(propiedades),
    }
}

/// Nombre de región sin repetir (las claves de un compound NBT son únicas).
fn nombre_unico(nombre: &str, usados: &mut HashSet<String>) -> String {
    let mut candidato = nombre.to_string();
    let mut n = 1;
    while usados.contains(&candidato) {
        n += 1;
        candidato = format!("{nombre} ({n})");
    }
    usados.insert(candidato.clone());
    candidato
}

fn a_int<T: TryInto<i32> + Copy + std::fmt::Display>(v: T) -> Result<i32, ErrorEscritura> {
    v.try_into().map_err(|_| {
        ErrorEscritura::Invalido(format!(
            "el valor {v} no cabe en un entero de 32 bits del formato"
        ))
    })
}

fn ahora_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

// ---------------------------------------------------------------- empaquetado

/// Inverso de `valor_empaquetado`: valores de `bits` bits seguidos, sin
/// huecos, y uno puede quedar partido entre dos longs consecutivos (como el
/// `LitematicaBitArray`). Devuelve `None` si no hay memoria para el array.
fn empaquetar(valores: impl Iterator<Item = u64>, cantidad: usize, bits: u32) -> Option<Vec<i64>> {
    debug_assert!((1..=64).contains(&bits));
    let mascara: u64 = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    let total =
        usize::try_from((cantidad as u64).checked_mul(u64::from(bits))?.div_ceil(64)).ok()?;
    let mut longs: Vec<u64> = Vec::new();
    longs.try_reserve_exact(total).ok()?;
    longs.resize(total, 0);
    for (i, v) in valores.take(cantidad).enumerate() {
        let v = v & mascara;
        let inicio = i as u64 * u64::from(bits);
        let s = (inicio >> 6) as usize;
        let desp = (inicio & 63) as u32;
        longs[s] |= v << desp;
        if desp + bits > 64 {
            longs[s + 1] |= v >> (64 - desp);
        }
    }
    Some(longs.into_iter().map(|l| l as i64).collect())
}

/// Empaqueta valores como `BlockStates` de .litematic (pública para los tests).
pub fn escribir_empaquetado(valores: &[u64], bits: u32) -> Vec<i64> {
    empaquetar(valores.iter().copied(), valores.len(), bits.clamp(1, 64)).unwrap_or_default()
}

#[cfg(test)]
mod pruebas {
    use std::io::Read;

    use fastnbt::Value;

    use super::*;
    use crate::schematic::modelo::{Dimensiones, Formato};
    use crate::schematic::{cargar, leer_empaquetado};

    fn opciones() -> OpcionesLitematic {
        OpcionesLitematic {
            version_datos: None,
            ahora_ms: Some(1_700_000_000_000),
        }
    }

    /// Región con `n` estados distintos repartidos de forma poco regular.
    fn region_con_estados(nombre: &str, origen: [i32; 3], tam: [u32; 3], n: usize) -> Region {
        let paleta: Vec<String> = (0..n)
            .map(|i| match i {
                0 => "minecraft:stone".to_string(),
                1 => "minecraft:air".to_string(),
                _ => format!("minecraft:wool_{i}[color=c{},lit=false]", i % 7),
            })
            .collect();
        let volumen = (tam[0] * tam[1] * tam[2]) as usize;
        let bloques = (0..volumen)
            .map(|i| ((i * 7919 + i / 3) % n) as u32)
            .collect();
        Region {
            nombre: nombre.to_string(),
            origen,
            tamano: Dimensiones {
                ancho: tam[0],
                alto: tam[1],
                largo: tam[2],
            },
            paleta,
            bloques,
            extras: ExtrasRegion::default(),
        }
    }

    fn schem(regiones: Vec<Region>) -> Schematic {
        Schematic {
            formato: Formato::Sponge,
            nombre: Some("Prueba".into()),
            autor: None,
            descripcion: None,
            version_datos: None,
            fecha_creacion: None,
            dimensiones: caja_englobante(&regiones),
            regiones,
        }
    }

    fn mismo_contenido(a: &Schematic, b: &Schematic) {
        assert_eq!(a.dimensiones, b.dimensiones);
        assert_eq!(a.regiones.len(), b.regiones.len());
        for (ra, rb) in a.regiones.iter().zip(&b.regiones) {
            assert_eq!(ra.origen, rb.origen, "origen de {}", ra.nombre);
            assert_eq!(ra.tamano, rb.tamano, "tamaño de {}", ra.nombre);
            let t = ra.tamano;
            for y in 0..t.alto {
                for z in 0..t.largo {
                    for x in 0..t.ancho {
                        assert_eq!(ra.bloque(x, y, z), rb.bloque(x, y, z), "({x},{y},{z})");
                    }
                }
            }
        }
    }

    fn raiz(bytes: &[u8]) -> HashMap<String, Value> {
        let mut nbt = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .read_to_end(&mut nbt)
            .unwrap();
        fastnbt::from_bytes(&nbt).unwrap()
    }

    fn compuesto<'a>(v: &'a Value, que: &str) -> &'a HashMap<String, Value> {
        match v {
            Value::Compound(m) => m,
            otro => panic!("{que} no es un compound: {otro:?}"),
        }
    }

    fn lista<'a>(v: &'a Value, que: &str) -> &'a [Value] {
        match v {
            Value::List(l) => l,
            otro => panic!("{que} no es una lista: {otro:?}"),
        }
    }

    #[test]
    fn empaquetado_es_inverso_del_lector() {
        for bits in 1..=64u32 {
            let tope = if bits == 64 {
                u64::MAX
            } else {
                (1u64 << bits) - 1
            };
            let valores: Vec<u64> = (0..301u64)
                .map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(i as u32) & tope)
                .collect();
            let longs = escribir_empaquetado(&valores, bits);
            assert_eq!(longs.len() as u64, (301 * u64::from(bits)).div_ceil(64));
            assert_eq!(
                leer_empaquetado(&longs, bits, valores.len()).unwrap(),
                valores,
                "bits={bits}"
            );
        }
    }

    #[test]
    fn valor_partido_entre_dos_longs() {
        // Con 7 bits, el índice 9 ocupa los bits 63..70: 1 en el primer long y 6 en el segundo.
        let mut valores = vec![0u64; 10];
        valores[9] = 0b101_1011;
        let longs = escribir_empaquetado(&valores, 7);
        assert_eq!(longs.len(), 2);
        assert_eq!((longs[0] as u64) >> 63, 1);
        assert_eq!(longs[1] as u64, 0b10_1101);
    }

    #[test]
    fn ida_y_vuelta_con_5_y_7_bits() {
        // 20 estados + aire → 5 bits; 100 estados + aire → 7 bits; tamaños impares
        // para que muchos valores crucen de un long al siguiente.
        let original = schem(vec![
            region_con_estados("cinco", [0, 0, 0], [7, 5, 3], 20),
            region_con_estados("siete", [-4, 2, 9], [5, 3, 7], 100),
        ]);
        let bytes = escribir_litematic(&original, &opciones()).unwrap();

        let r = raiz(&bytes);
        let regiones = compuesto(&r["Regions"], "Regions");
        for (nombre, bits) in [("cinco", 5), ("siete", 7)] {
            let reg = compuesto(&regiones[nombre], nombre);
            let pal = lista(&reg["BlockStatePalette"], "BlockStatePalette");
            assert_eq!(bits_por_bloque(pal.len()), bits, "{nombre}");
        }

        let leido = cargar(&bytes).unwrap();
        assert_eq!(leido.formato, Formato::Litematic);
        assert_eq!(leido.nombre.as_deref(), Some("Prueba"));
        assert_eq!(leido.version_datos, Some(VERSION_DATOS_POR_DEFECTO));
        assert_eq!(leido.fecha_creacion, Some(1_700_000_000_000));
        mismo_contenido(&original, &leido);
    }

    #[test]
    fn version_de_datos_del_llamador_si_el_modelo_no_la_trae() {
        let mut s = schem(vec![region_con_estados("r", [0, 0, 0], [2, 2, 2], 3)]);
        let op = OpcionesLitematic {
            version_datos: Some(4189),
            ..opciones()
        };
        let leer = |s: &Schematic| cargar(&escribir_litematic(s, &op).unwrap()).unwrap();
        assert_eq!(leer(&s).version_datos, Some(4189));
        s.version_datos = Some(3700);
        assert_eq!(leer(&s).version_datos, Some(3700));
    }

    #[test]
    fn paleta_compacta_con_aire_en_el_cero() {
        let mut r = region_con_estados("r", [0, 0, 0], [2, 1, 1], 4);
        r.paleta = vec![
            "minecraft:dirt".into(),
            "minecraft:sin_usar".into(),
            "minecraft:oak_stairs[facing=north,half=top]".into(),
            "minecraft:dirt".into(),
        ];
        r.bloques = vec![2, 3];
        let bytes = escribir_litematic(&schem(vec![r]), &opciones()).unwrap();
        let raiz = raiz(&bytes);
        let reg = compuesto(&compuesto(&raiz["Regions"], "Regions")["r"], "r");
        let pal = lista(&reg["BlockStatePalette"], "paleta");
        let nombres: Vec<&Value> = pal
            .iter()
            .map(|e| &compuesto(e, "entrada")["Name"])
            .collect();
        let esperados = ["minecraft:air", "minecraft:oak_stairs", "minecraft:dirt"]
            .map(|s| Value::String(s.into()));
        assert_eq!(nombres, esperados.iter().collect::<Vec<_>>());
        let props = compuesto(&compuesto(&pal[1], "escalera")["Properties"], "Properties");
        assert_eq!(props["facing"], Value::String("north".into()));
        assert_eq!(props["half"], Value::String("top".into()));
        assert!(!compuesto(&pal[0], "aire").contains_key("Properties"));
    }

    #[test]
    fn conserva_entidades_y_ticks() {
        let cofre = Value::Compound(HashMap::from([
            ("id".to_string(), Value::String("minecraft:chest".into())),
            ("x".to_string(), Value::Int(1)),
            ("y".to_string(), Value::Int(0)),
            ("z".to_string(), Value::Int(1)),
            ("Items".to_string(), Value::List(vec![])),
        ]));
        let entidad = Value::Compound(HashMap::from([
            (
                "id".to_string(),
                Value::String("minecraft:armor_stand".into()),
            ),
            ("Pos".to_string(), Value::List(vec![Value::Double(0.5); 3])),
            (
                "UUID".to_string(),
                Value::IntArray(fastnbt::IntArray::new(vec![1, 2, 3, 4])),
            ),
        ]));
        let tick = Value::Compound(HashMap::from([
            (
                "Block".to_string(),
                Value::String("minecraft:repeater".into()),
            ),
            ("Time".to_string(), Value::Int(2)),
            ("Priority".to_string(), Value::Int(0)),
            ("SubTick".to_string(), Value::Long(5)),
        ]));
        let mut r = region_con_estados("r", [0, 0, 0], [3, 2, 3], 5);
        r.extras = ExtrasRegion {
            entidades_bloque: Some(Value::List(vec![cofre])),
            entidades: Some(Value::List(vec![entidad])),
            ticks_bloque: Some(Value::List(vec![tick])),
            ticks_fluido: None,
        };
        let original = schem(vec![r]);
        let leido = cargar(&escribir_litematic(&original, &opciones()).unwrap()).unwrap();
        let ex = &leido.regiones[0].extras;
        let ex0 = &original.regiones[0].extras;
        assert_eq!(ex.entidades_bloque, ex0.entidades_bloque);
        assert_eq!(ex.entidades, ex0.entidades);
        assert_eq!(ex.ticks_bloque, ex0.ticks_bloque);
        // Lo que no venía se escribe como lista vacía.
        assert_eq!(ex.ticks_fluido, Some(Value::List(vec![])));
    }

    #[test]
    fn nombres_de_region_repetidos_no_se_pisan() {
        let original = schem(vec![
            region_con_estados("igual", [0, 0, 0], [2, 2, 2], 3),
            region_con_estados("igual (2)", [9, 0, 0], [1, 1, 1], 2),
            region_con_estados("igual", [5, 0, 0], [3, 1, 2], 6),
        ]);
        let leido = cargar(&escribir_litematic(&original, &opciones()).unwrap()).unwrap();
        let nombres: Vec<&str> = leido.regiones.iter().map(|r| r.nombre.as_str()).collect();
        assert_eq!(nombres, ["igual", "igual (2)", "igual (3)"]);
        mismo_contenido(&original, &leido);
    }

    #[test]
    fn modelo_incoherente_da_error() {
        let mut r = region_con_estados("r", [0, 0, 0], [2, 2, 2], 3);
        r.bloques[3] = 99;
        assert!(matches!(
            escribir_litematic(&schem(vec![r]), &opciones()),
            Err(ErrorEscritura::Invalido(_))
        ));
        let mut r = region_con_estados("r", [0, 0, 0], [2, 2, 2], 3);
        r.bloques.pop();
        assert!(escribir_litematic(&schem(vec![r]), &opciones()).is_err());
        assert!(escribir_litematic(&schem(vec![]), &opciones()).is_err());
    }
}
