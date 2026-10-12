// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Formato binario de la malla que recibe el frontend (`docs/visor.md` lo
//! describe con offsets). Todo en little endian; cada sección empieza en un
//! múltiplo de 4 bytes, así en JS se crean `Uint16Array`/`Uint32Array` sobre el
//! mismo `ArrayBuffer` sin copiar.

use serde::Serialize;

use super::apariencia::{DatosAtlas, Material};
use super::mallado::{ChunkMalla, Malla};

/// Primeros cuatro bytes del binario.
pub const MAGIA: [u8; 4] = *b"L3DM";
/// Versión del formato. Cambia solo si el binario deja de ser compatible.
pub const VERSION: u32 = 1;
/// Bytes de la cabecera general.
pub const CABECERA: usize = 48;
/// Bytes de la cabecera de cada chunk.
pub const CABECERA_CHUNK: usize = 40;

/// Sección JSON del binario.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Tablas<'a> {
    materiales: &'a [Material],
    paleta: &'a [String],
    total_bloques: u64,
    /// `null` si la malla va sin texturas.
    atlas: Option<&'a DatosAtlas>,
}

fn rellenar(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(4) {
        v.push(0);
    }
}

fn u32_(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn i32_(v: &mut Vec<u8>, x: i32) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn alinear(n: usize) -> usize {
    n.div_ceil(4) * 4
}

/// Bytes de datos de un chunk (sin su cabecera), con rellenos.
fn bytes_datos(c: &ChunkMalla, bytes_indice: usize) -> usize {
    let nv = c.vertices();
    alinear(nv * 6)
        + alinear(nv)
        + nv * 4
        + alinear(nv * 2)
        + alinear(c.indices.len() * bytes_indice)
}

/// Serializa la malla al formato binario v1.
pub fn serializar(m: &Malla) -> Vec<u8> {
    let json = serde_json::to_vec(&Tablas {
        materiales: &m.materiales,
        paleta: &m.paleta,
        total_bloques: m.total_bloques,
        atlas: m.atlas.as_ref(),
    })
    .unwrap_or_else(|_| b"{}".to_vec());
    let tam: usize = CABECERA
        + alinear(json.len())
        + m.chunks
            .iter()
            .map(|c| CABECERA_CHUNK + bytes_datos(c, 4))
            .sum::<usize>();
    let mut v = Vec::with_capacity(tam);

    v.extend_from_slice(&MAGIA);
    u32_(&mut v, VERSION);
    u32_(&mut v, CABECERA as u32);
    u32_(&mut v, m.chunks.len() as u32);
    for o in m.origen {
        i32_(&mut v, o);
    }
    for t in m.tamano {
        u32_(&mut v, t);
    }
    u32_(&mut v, json.len() as u32);
    u32_(&mut v, 0); // reservado
    debug_assert_eq!(v.len(), CABECERA);
    v.extend_from_slice(&json);
    rellenar(&mut v);

    for c in &m.chunks {
        let nv = c.vertices();
        let bytes_indice = if nv <= usize::from(u16::MAX) + 1 {
            2
        } else {
            4
        };
        for o in c.origen {
            i32_(&mut v, o);
        }
        i32_(&mut v, c.y_min);
        i32_(&mut v, c.y_max);
        u32_(&mut v, nv as u32);
        u32_(&mut v, c.indices.len() as u32);
        u32_(&mut v, c.indices_opacos);
        u32_(&mut v, bytes_indice as u32);
        u32_(&mut v, bytes_datos(c, bytes_indice) as u32);
        for p in &c.posiciones {
            v.extend_from_slice(&p.to_le_bytes());
        }
        rellenar(&mut v);
        v.extend_from_slice(&c.normales);
        rellenar(&mut v);
        for uv in &c.uvs {
            v.extend_from_slice(&uv.to_le_bytes());
        }
        for m in &c.materiales {
            v.extend_from_slice(&m.to_le_bytes());
        }
        rellenar(&mut v);
        if bytes_indice == 2 {
            for &i in &c.indices {
                v.extend_from_slice(&(i as u16).to_le_bytes());
            }
        } else {
            for &i in &c.indices {
                v.extend_from_slice(&i.to_le_bytes());
            }
        }
        rellenar(&mut v);
    }
    v
}

// ---------------------------------------------------------------------------
// Lectura: espejo de `src/api/visor.ts`, para las pruebas y como referencia.
// ---------------------------------------------------------------------------

/// Cabecera general leída.
#[derive(Debug, Clone, PartialEq)]
pub struct CabeceraLeida {
    pub version: u32,
    pub n_chunks: u32,
    pub origen: [i32; 3],
    pub tamano: [u32; 3],
    pub json: serde_json::Value,
}

/// Chunk leído: cabecera y datos ya decodificados.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChunkLeido {
    pub origen: [i32; 3],
    pub y_min: i32,
    pub y_max: i32,
    pub posiciones: Vec<u16>,
    pub normales: Vec<u8>,
    pub uvs: Vec<u16>,
    pub materiales: Vec<u16>,
    pub indices: Vec<u32>,
    pub indices_opacos: u32,
    pub bytes_indice: u32,
}

struct Lector<'a> {
    b: &'a [u8],
    i: usize,
}

impl Lector<'_> {
    fn tomar(&mut self, n: usize) -> Result<&[u8], String> {
        let fin = self.i.checked_add(n).filter(|&f| f <= self.b.len());
        let fin = fin.ok_or_else(|| format!("binario truncado en el byte {}", self.i))?;
        let s = &self.b[self.i..fin];
        self.i = fin;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let s = self.tomar(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(self.u32()? as i32)
    }
    fn alinear(&mut self) {
        self.i = alinear(self.i);
    }
    fn u16s(&mut self, n: usize) -> Result<Vec<u16>, String> {
        let s = self.tomar(n * 2)?;
        let v = s
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| u16::from_le_bytes(c))
            .collect();
        self.alinear();
        Ok(v)
    }
}

/// Lee el binario entero y comprueba que los tamaños declarados cuadran.
pub fn leer(bytes: &[u8]) -> Result<(CabeceraLeida, Vec<ChunkLeido>), String> {
    let mut l = Lector { b: bytes, i: 0 };
    if l.tomar(4)? != MAGIA {
        return Err("no empieza por L3DM".into());
    }
    let version = l.u32()?;
    let tam_cabecera = l.u32()? as usize;
    let n_chunks = l.u32()?;
    let origen = [l.i32()?, l.i32()?, l.i32()?];
    let tamano = [l.u32()?, l.u32()?, l.u32()?];
    let bytes_json = l.u32()? as usize;
    l.i = tam_cabecera;
    let json = serde_json::from_slice(l.tomar(bytes_json)?).map_err(|e| e.to_string())?;
    l.alinear();
    let mut chunks = Vec::with_capacity(n_chunks as usize);
    for _ in 0..n_chunks {
        let mut c = ChunkLeido {
            origen: [l.i32()?, l.i32()?, l.i32()?],
            y_min: l.i32()?,
            y_max: l.i32()?,
            ..Default::default()
        };
        let nv = l.u32()? as usize;
        let ni = l.u32()? as usize;
        c.indices_opacos = l.u32()?;
        c.bytes_indice = l.u32()?;
        let bytes_datos = l.u32()? as usize;
        let inicio = l.i;
        c.posiciones = l.u16s(nv * 3)?;
        c.normales = l.tomar(nv)?.to_vec();
        l.alinear();
        c.uvs = l.u16s(nv * 2)?;
        c.materiales = l.u16s(nv)?;
        c.indices = if c.bytes_indice == 2 {
            l.u16s(ni)?.into_iter().map(u32::from).collect()
        } else {
            let s = l.tomar(ni * 4)?;
            s.as_chunks::<4>()
                .0
                .iter()
                .map(|&c| u32::from_le_bytes(c))
                .collect()
        };
        l.alinear();
        if l.i - inicio != bytes_datos {
            return Err(format!(
                "bytesDatos dice {bytes_datos} y el chunk ocupa {}",
                l.i - inicio
            ));
        }
        chunks.push(c);
    }
    if l.i != bytes.len() {
        return Err(format!("sobran {} bytes al final", bytes.len() - l.i));
    }
    Ok((
        CabeceraLeida {
            version,
            n_chunks,
            origen,
            tamano,
            json,
        },
        chunks,
    ))
}
