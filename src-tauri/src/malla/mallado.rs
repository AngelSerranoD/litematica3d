// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Mallado por chunks de 16³: eliminación de caras ocultas, greedy meshing de
//! las caras de cubos completos y cajas/planos de los modelos aproximados.
//! Los chunks se mallan en paralelo (rayon); cada uno solo lee su chunk y la
//! capa pegada de sus seis vecinos.

use std::collections::HashMap;

use rayon::prelude::*;

use super::apariencia::{
    Cara, DatosAtlas, FuenteTexturas, Material, TablaMateriales, Tipo, clave_visual, nombre_de,
};
use super::formas::{Forma, Pieza, forma_de};
use super::mundo::{LADO, Mundo, indice_local};

/// Opciones del mallado.
#[derive(Debug, Clone, Copy)]
pub struct Opciones {
    /// Fusionar caras coplanarias del mismo material (activado por defecto;
    /// apagarlo solo sirve para pruebas y comparaciones).
    pub greedy: bool,
    /// Solo las capas Y de este rango (coordenadas del modelo, inclusivo).
    /// Lo de fuera cuenta como aire, así el corte queda cerrado con caras.
    pub capas: Option<[i32; 2]>,
}

impl Default for Opciones {
    fn default() -> Self {
        Opciones {
            greedy: true,
            capas: None,
        }
    }
}

/// Malla de un chunk. Posiciones y UV en 1/16 de bloque (ver `docs/visor.md`).
#[derive(Debug, Clone, Default)]
pub struct ChunkMalla {
    /// Esquina mínima del chunk en coordenadas del modelo (múltiplo de 16).
    pub origen: [i32; 3],
    /// Primera y última capa Y (modelo, inclusivas) con geometría.
    pub y_min: i32,
    pub y_max: i32,
    /// x, y, z por vértice, relativas a `origen`, en 1/16 de bloque (0..=256).
    pub posiciones: Vec<u16>,
    /// Índice de normal por vértice: 0..5 caras de cubo, 6..9 diagonales.
    pub normales: Vec<u8>,
    /// u, v por vértice en 1/16 de bloque (se repite cada 16).
    pub uvs: Vec<u16>,
    /// Índice en la tabla de materiales por vértice.
    pub materiales: Vec<u16>,
    /// Triángulos: primero los opacos y de recorte, después los translúcidos.
    pub indices: Vec<u32>,
    /// Cuántos de `indices` son opacos o de recorte.
    pub indices_opacos: u32,
}

impl ChunkMalla {
    pub fn vertices(&self) -> usize {
        self.normales.len()
    }
}

/// Resultado completo del mallado.
#[derive(Debug, Clone)]
pub struct Malla {
    /// Esquina mínima del modelo en coordenadas del schematic.
    pub origen: [i32; 3],
    pub tamano: [u32; 3],
    pub chunks: Vec<ChunkMalla>,
    pub materiales: Vec<Material>,
    pub paleta: Vec<String>,
    pub total_bloques: u64,
    /// Atlas con el que se resolvieron las texturas, o `None` (solo colores).
    pub atlas: Option<DatosAtlas>,
}

/// Lo que el mallador necesita saber de cada entrada de la paleta.
#[derive(Debug, Clone, Default)]
struct Info {
    /// Cubo completo (opaco o no).
    cubo: bool,
    /// Qué parte de cada cara del bloque tapa al vecino de ese lado, como en
    /// Minecraft: 16 filas (eje v de la cara) de 16 bits (eje u), en 1/16.
    /// Cubo opaco: todo; modelo opaco: lo que sus cajas pegan al borde;
    /// transparentes y de recorte: nada.
    cubre: [[u16; 16]; 6],
    /// Bit `1 << cara` si `cubre[cara]` está completa.
    llenas: u8,
    /// Cubos no opacos: identificador del nombre; oculta solo a sus iguales.
    grupo: u32,
    translucido: bool,
    /// Material por cara (`Cara as usize`, 0..=6).
    mat: [u16; 7],
    /// Caras de las cajas del modelo que pueden verse (sin las tapadas por
    /// otra caja del mismo modelo).
    caras: Vec<CaraCaja>,
    /// Lleva planos cruzados.
    cruz: bool,
}

/// Cara de una caja de un modelo, en 1/16 de bloque.
#[derive(Debug, Clone, Copy)]
struct CaraCaja {
    cara: Cara,
    min: [u8; 3],
    max: [u8; 3],
    /// Está en el borde del bloque: puede taparla el vecino.
    en_borde: bool,
}

/// Eje de una cara y los otros dos en orden cíclico tras él (u, v).
#[inline]
fn ejes_de(cara: Cara) -> (usize, usize, usize) {
    let e = cara.eje();
    (e, (e + 1) % 3, (e + 2) % 3)
}

/// Bits a..b (b <= 16).
#[inline]
fn bits(a: u8, b: u8) -> u16 {
    ((1u32 << b) - (1u32 << a)) as u16
}

/// ¿`cubre` tapa el rectángulo [u0,u1)×[v0,v1)?
#[inline]
fn tapa(cubre: &[u16; 16], u0: u8, u1: u8, v0: u8, v1: u8) -> bool {
    let m = bits(u0, u1);
    cubre[v0 as usize..v1 as usize].iter().all(|&f| f & m == m)
}

/// Lo que las cajas de un modelo opaco tapan en cada cara del bloque.
fn cobertura(piezas: &[Pieza]) -> [[u16; 16]; 6] {
    let mut c = [[0u16; 16]; 6];
    for p in piezas {
        let Pieza::Caja { min, max } = *p else {
            continue;
        };
        for cara in Cara::CUBO {
            let (e, ue, ve) = ejes_de(cara);
            let en_borde = if cara.positiva() {
                max[e] == 16
            } else {
                min[e] == 0
            };
            if en_borde {
                for fila in &mut c[cara as usize][min[ve] as usize..max[ve] as usize] {
                    *fila |= bits(min[ue], max[ue]);
                }
            }
        }
    }
    c
}

/// Caras de las cajas de un modelo, quitando las que tapa por completo otra
/// caja del mismo modelo (el fondo del escalón, el extremo de un travesaño…).
fn caras_de_cajas(piezas: &[Pieza]) -> Vec<CaraCaja> {
    let cajas: Vec<([u8; 3], [u8; 3])> = piezas
        .iter()
        .filter_map(|p| match *p {
            Pieza::Caja { min, max } => Some((min, max)),
            Pieza::Cruz => None,
        })
        .collect();
    let mut v = Vec::new();
    for (i, &(min, max)) in cajas.iter().enumerate() {
        for cara in Cara::CUBO {
            let (e, ue, ve) = ejes_de(cara);
            let plano = if cara.positiva() { max[e] } else { min[e] };
            let tapada = cajas.iter().enumerate().any(|(j, &(a, b))| {
                let detras = if cara.positiva() {
                    a[e] <= plano && plano < b[e]
                } else {
                    a[e] < plano && plano <= b[e]
                };
                j != i
                    && detras
                    && a[ue] <= min[ue]
                    && max[ue] <= b[ue]
                    && a[ve] <= min[ve]
                    && max[ve] <= b[ve]
            });
            if !tapada {
                let en_borde = if cara.positiva() {
                    max[e] == 16
                } else {
                    min[e] == 0
                };
                v.push(CaraCaja {
                    cara,
                    min,
                    max,
                    en_borde,
                });
            }
        }
    }
    v
}

fn infos_de(paleta: &[String], tabla: &mut TablaMateriales) -> Result<Vec<Info>, String> {
    let lleno = || "el schematic tiene demasiados materiales distintos".to_string();
    let mut grupos: HashMap<&str, u32> = HashMap::new();
    let mut infos = Vec::with_capacity(paleta.len());
    for (i, estado) in paleta.iter().enumerate() {
        let forma = if i == 0 {
            Forma::Nada
        } else {
            forma_de(estado)
        };
        let clave = clave_visual(estado);
        let mut info = Info::default();
        match forma {
            Forma::Nada => {}
            Forma::Cubo(tipo) => {
                info.cubo = true;
                info.translucido = tipo == Tipo::Translucido;
                if tipo == Tipo::Opaco {
                    info.cubre = [[u16::MAX; 16]; 6];
                } else {
                    let n = grupos.len() as u32 + 1;
                    info.grupo = *grupos.entry(nombre_de(estado)).or_insert(n);
                }
                for cara in Cara::CUBO {
                    info.mat[cara as usize] = tabla.indice(&clave, cara, tipo).ok_or_else(lleno)?;
                }
            }
            Forma::Modelo(tipo, piezas) => {
                info.translucido = tipo == Tipo::Translucido;
                let cruz = piezas.contains(&Pieza::Cruz);
                let cajas = piezas.iter().any(|p| matches!(p, Pieza::Caja { .. }));
                if cajas {
                    for cara in Cara::CUBO {
                        info.mat[cara as usize] =
                            tabla.indice(&clave, cara, tipo).ok_or_else(lleno)?;
                    }
                }
                if cruz {
                    info.mat[Cara::Cruz as usize] =
                        tabla.indice(&clave, Cara::Cruz, tipo).ok_or_else(lleno)?;
                }
                info.cruz = cruz;
                info.caras = caras_de_cajas(&piezas);
                if tipo == Tipo::Opaco {
                    info.cubre = cobertura(&piezas);
                }
            }
        }
        for cara in Cara::CUBO {
            if info.cubre[cara as usize].iter().all(|&f| f == u16::MAX) {
                info.llenas |= 1 << cara as u8;
            }
        }
        infos.push(info);
    }
    Ok(infos)
}

/// Malla todo el mundo sin texturas (solo colores). Error (texto para el
/// usuario) si hay más de 65 536 materiales distintos.
pub fn mallar(mundo: &Mundo, opciones: &Opciones) -> Result<Malla, String> {
    mallar_con_texturas(mundo, opciones, None)
}

/// Como [`mallar`], tomando la textura de cada material de `fuente` (el atlas
/// activo), si la hay.
pub fn mallar_con_texturas(
    mundo: &Mundo,
    opciones: &Opciones,
    fuente: Option<&dyn FuenteTexturas>,
) -> Result<Malla, String> {
    let mut tabla = TablaMateriales::con_texturas(fuente);
    let infos = infos_de(&mundo.paleta, &mut tabla)?;
    let chunks: Vec<ChunkMalla> = mundo
        .claves_chunks()
        .par_iter()
        .filter_map(|&c| mallar_chunk(mundo, &infos, c, opciones))
        .collect();
    Ok(Malla {
        origen: mundo.origen,
        tamano: mundo.tamano,
        chunks,
        materiales: tabla.materiales,
        paleta: mundo.paleta.clone(),
        total_bloques: mundo.total_bloques(),
        atlas: fuente.map(|f| f.datos_atlas()),
    })
}

/// Lado de la rejilla con borde: 16 + una capa de cada vecino.
const P: usize = 18;

/// Índice en la rejilla con borde; x, y, z en -1..=16.
#[inline]
fn pi(x: i32, y: i32, z: i32) -> usize {
    (((y + 1) as usize * P) + (z + 1) as usize) * P + (x + 1) as usize
}

/// Copia el chunk y la capa pegada de sus seis vecinos a una rejilla 18³.
/// Las esquinas y aristas del borde quedan como aire (no se consultan).
fn rejilla(mundo: &Mundo, clave: [i32; 3], capas: Option<[i32; 2]>) -> Vec<u32> {
    let mut celdas = vec![0u32; P * P * P];
    let dentro = |cy: i32, ly: i32| match capas {
        Some([a, b]) => {
            let y = cy * LADO + ly;
            a <= y && y <= b
        }
        None => true,
    };
    if let Some(c) = mundo.chunk(clave) {
        for y in 0..16 {
            if !dentro(clave[1], y as i32) {
                continue;
            }
            for z in 0..16 {
                for x in 0..16 {
                    celdas[pi(x as i32, y as i32, z as i32)] = c[indice_local(x, y, z)];
                }
            }
        }
    }
    for cara in Cara::CUBO {
        let d = cara.delta();
        let vecino = [clave[0] + d[0], clave[1] + d[1], clave[2] + d[2]];
        let Some(c) = mundo.chunk(vecino) else {
            continue;
        };
        let eje = cara.eje();
        // Capa del vecino pegada a este chunk y su posición en la rejilla.
        let (local, borde) = if cara.positiva() { (0, 16) } else { (15, -1) };
        for a in 0..16i32 {
            for b in 0..16i32 {
                let mut l = [a, b, 0];
                let mut g = [a, b, 0];
                // Los dos ejes libres en orden cíclico tras `eje`.
                let (u, v) = ((eje + 1) % 3, (eje + 2) % 3);
                l[eje] = local;
                l[u] = a;
                l[v] = b;
                g[eje] = borde;
                g[u] = a;
                g[v] = b;
                if !dentro(vecino[1], l[1]) {
                    continue;
                }
                celdas[pi(g[0], g[1], g[2])] =
                    c[indice_local(l[0] as usize, l[1] as usize, l[2] as usize)];
            }
        }
    }
    celdas
}

/// UV de un vértice de una cara de cubo/caja, en 1/16 de bloque: u hacia la
/// derecha y v hacia arriba mirando la cara desde fuera (en las caras de
/// arriba y abajo, v hacia el norte y hacia el sur). Depende solo de la
/// posición, así que una caja parcial (losa) toma la parte de la textura que le
/// corresponde, como en Minecraft.
#[inline]
fn uv_de(cara: Cara, p: [u16; 3]) -> [u16; 2] {
    let [x, y, z] = p;
    match cara {
        Cara::Este => [256 - z, y],
        Cara::Oeste => [z, y],
        Cara::Sur => [x, y],
        Cara::Norte => [256 - x, y],
        Cara::Arriba => [x, 256 - z],
        Cara::Abajo | Cara::Cruz => [x, z],
    }
}

/// Esquinas de un rectángulo perpendicular a `cara` en el plano `plano`, con
/// los otros dos ejes (orden cíclico tras el eje de la cara) en [u0,u1]×[v0,v1].
/// Orden antihorario visto desde fuera (cara frontal en Three.js).
#[inline]
fn rectangulo(cara: Cara, plano: u16, u0: u16, u1: u16, v0: u16, v1: u16) -> [[u16; 3]; 4] {
    let eje = cara.eje();
    let (ue, ve) = ((eje + 1) % 3, (eje + 2) % 3);
    let punto = |u: u16, v: u16| {
        let mut p = [0u16; 3];
        p[eje] = plano;
        p[ue] = u;
        p[ve] = v;
        p
    };
    if cara.positiva() {
        [punto(u0, v0), punto(u1, v0), punto(u1, v1), punto(u0, v1)]
    } else {
        [punto(u0, v0), punto(u0, v1), punto(u1, v1), punto(u1, v0)]
    }
}

/// Acumulador de la malla de un chunk.
#[derive(Default)]
struct Constructor {
    m: ChunkMalla,
    translucidos: Vec<u32>,
    /// Capas (locales al chunk) con geometría.
    capa_min: i32,
    capa_max: i32,
}

impl Constructor {
    fn nuevo() -> Self {
        Constructor {
            capa_min: i32::MAX,
            capa_max: i32::MIN,
            ..Default::default()
        }
    }

    #[inline]
    fn quad(
        &mut self,
        esquinas: [[u16; 3]; 4],
        normal: u8,
        uvs: [[u16; 2]; 4],
        material: u16,
        translucido: bool,
        capas: (i32, i32),
    ) {
        let base = self.m.normales.len() as u32;
        for (p, uv) in esquinas.iter().zip(uvs) {
            self.m.posiciones.extend_from_slice(p);
            self.m.uvs.extend_from_slice(&uv);
        }
        self.m.normales.extend_from_slice(&[normal; 4]);
        self.m.materiales.extend_from_slice(&[material; 4]);
        let tri = [base, base + 1, base + 2, base, base + 2, base + 3];
        if translucido {
            self.translucidos.extend_from_slice(&tri);
        } else {
            self.m.indices.extend_from_slice(&tri);
        }
        self.capa_min = self.capa_min.min(capas.0);
        self.capa_max = self.capa_max.max(capas.1);
    }

    /// Cara de cubo o caja: UV calculadas a partir de la posición.
    #[inline]
    fn cara(&mut self, cara: Cara, esquinas: [[u16; 3]; 4], info: &Info, capas: (i32, i32)) {
        let uvs = esquinas.map(|p| uv_de(cara, p));
        let mat = info.mat[cara as usize];
        self.quad(esquinas, cara as u8, uvs, mat, info.translucido, capas);
    }

    fn terminar(mut self, clave: [i32; 3]) -> Option<ChunkMalla> {
        if self.m.normales.is_empty() {
            return None;
        }
        self.m.indices_opacos = self.m.indices.len() as u32;
        self.m.indices.append(&mut self.translucidos);
        let o = clave.map(|c| c * LADO);
        self.m.origen = o;
        self.m.y_min = o[1] + self.capa_min;
        self.m.y_max = o[1] + self.capa_max;
        Some(self.m)
    }
}

fn mallar_chunk(
    mundo: &Mundo,
    infos: &[Info],
    clave: [i32; 3],
    opciones: &Opciones,
) -> Option<ChunkMalla> {
    let celdas = rejilla(mundo, clave, opciones.capas);
    let info = |p: u32| infos.get(p as usize);
    let mut k = Constructor::nuevo();
    let mut mascara = [0u32; 256];

    // 1) Caras de cubos completos, capa a capa y con greedy meshing.
    for cara in Cara::CUBO {
        let eje = cara.eje();
        let (ue, ve) = ((eje + 1) % 3, (eje + 2) % 3);
        let d = cara.delta();
        let opuesta = 1u8 << cara.opuesta() as u8;
        for capa in 0..16i32 {
            let mut alguna = false;
            for v in 0..16i32 {
                for u in 0..16i32 {
                    let mut p = [0i32; 3];
                    p[eje] = capa;
                    p[ue] = u;
                    p[ve] = v;
                    let id = celdas[pi(p[0], p[1], p[2])];
                    let mut m = 0;
                    if let Some(a) = info(id).filter(|a| a.cubo) {
                        let nid = celdas[pi(p[0] + d[0], p[1] + d[1], p[2] + d[2])];
                        let visible = match info(nid) {
                            Some(b) => {
                                b.llenas & opuesta == 0 && !(a.grupo != 0 && a.grupo == b.grupo)
                            }
                            None => true,
                        };
                        if visible {
                            // material + 1, así 0 queda para «sin cara».
                            // Un material tiene un solo tipo, así que fusionar
                            // por material nunca mezcla pasadas de render.
                            m = u32::from(a.mat[cara as usize]) + 1;
                            alguna = true;
                        }
                    }
                    mascara[(v * 16 + u) as usize] = m;
                }
            }
            if !alguna {
                continue;
            }
            let plano = ((capa + i32::from(cara.positiva())) * 16) as u16;
            for v in 0..16usize {
                let mut u = 0usize;
                while u < 16 {
                    let m = mascara[v * 16 + u];
                    if m == 0 {
                        u += 1;
                        continue;
                    }
                    let mut w = 1;
                    let mut h = 1;
                    if opciones.greedy {
                        while u + w < 16 && mascara[v * 16 + u + w] == m {
                            w += 1;
                        }
                        'alto: while v + h < 16 {
                            for i in 0..w {
                                if mascara[(v + h) * 16 + u + i] != m {
                                    break 'alto;
                                }
                            }
                            h += 1;
                        }
                    }
                    for fila in v..v + h {
                        mascara[fila * 16 + u..fila * 16 + u + w].fill(0);
                    }
                    let esquinas = rectangulo(
                        cara,
                        plano,
                        (u * 16) as u16,
                        ((u + w) * 16) as u16,
                        (v * 16) as u16,
                        ((v + h) * 16) as u16,
                    );
                    // Capas Y que cubre el rectángulo.
                    let capas = match eje {
                        1 => (capa, capa),
                        0 => (u as i32, (u + w) as i32 - 1), // ue = Y
                        _ => (v as i32, (v + h) as i32 - 1), // eje Z: ve = Y
                    };
                    let id = {
                        let mut p = [0i32; 3];
                        p[eje] = capa;
                        p[ue] = u as i32;
                        p[ve] = v as i32;
                        celdas[pi(p[0], p[1], p[2])]
                    };
                    if let Some(a) = info(id) {
                        k.cara(cara, esquinas, a, capas);
                    }
                    u += w;
                }
            }
        }
    }

    // 2) Modelos aproximados: cajas y planos cruzados, sin fusionar.
    for y in 0..16i32 {
        for z in 0..16i32 {
            for x in 0..16i32 {
                let Some(a) = info(celdas[pi(x, y, z)]) else {
                    continue;
                };
                let base = [x as u16 * 16, y as u16 * 16, z as u16 * 16];
                for c in &a.caras {
                    let (e, ue, ve) = ejes_de(c.cara);
                    if c.en_borde {
                        let d = c.cara.delta();
                        let nid = celdas[pi(x + d[0], y + d[1], z + d[2])];
                        let opuesta = c.cara.opuesta() as usize;
                        let tapada = info(nid).is_some_and(|b| {
                            tapa(
                                &b.cubre[opuesta],
                                c.min[ue],
                                c.max[ue],
                                c.min[ve],
                                c.max[ve],
                            )
                        });
                        if tapada {
                            continue;
                        }
                    }
                    let plano = base[e]
                        + u16::from(if c.cara.positiva() {
                            c.max[e]
                        } else {
                            c.min[e]
                        });
                    let esquinas = rectangulo(
                        c.cara,
                        plano,
                        base[ue] + u16::from(c.min[ue]),
                        base[ue] + u16::from(c.max[ue]),
                        base[ve] + u16::from(c.min[ve]),
                        base[ve] + u16::from(c.max[ve]),
                    );
                    k.cara(c.cara, esquinas, a, (y, y));
                }
                if a.cruz {
                    cruz(&mut k, base, a, y);
                }
            }
        }
    }
    k.terminar(clave)
}

/// Dos planos verticales en diagonal, cada uno con sus dos lados (cuatro
/// quads). Normales: 6 = (+1,0,+1), 7 = (-1,0,-1), 8 = (+1,0,-1), 9 = (-1,0,+1).
fn cruz(k: &mut Constructor, b: [u16; 3], info: &Info, y: i32) {
    let [x0, y0, z0] = b;
    let (x1, y1, z1) = (x0 + 16, y0 + 16, z0 + 16);
    let mat = info.mat[Cara::Cruz as usize];
    let uvs = [[0, y0], [16, y0], [16, y1], [0, y1]];
    let atras = |mut e: [[u16; 3]; 4], mut u: [[u16; 2]; 4]| {
        e.reverse();
        u.reverse();
        (e, u)
    };
    // Plano A, de (x0,z0) a (x1,z1): de frente mira a (-1,0,+1).
    let a = [[x0, y0, z0], [x1, y0, z1], [x1, y1, z1], [x0, y1, z0]];
    // Plano B, de (x1,z0) a (x0,z1): de frente mira a (-1,0,-1).
    let bb = [[x1, y0, z0], [x0, y0, z1], [x0, y1, z1], [x1, y1, z0]];
    k.quad(a, 9, uvs, mat, info.translucido, (y, y));
    let (e, u) = atras(a, uvs);
    k.quad(e, 8, u, mat, info.translucido, (y, y));
    k.quad(bb, 7, uvs, mat, info.translucido, (y, y));
    let (e, u) = atras(bb, uvs);
    k.quad(e, 6, u, mat, info.translucido, (y, y));
}
