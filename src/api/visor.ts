// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Malla del visor 3D: decodifica el binario del comando `malla` (formato en
// docs/visor.md) a typed arrays que apuntan al mismo ArrayBuffer, sin copiar.

import { llamar } from "./llamar";

/** Versión del formato binario que entiende este decodificador. */
const VERSION_MALLA = 1;

const MAGIA = "L3DM";
const CABECERA_CHUNK = 40;

export type Vec3 = [number, number, number];

/** Cara del bloque a la que pertenece un material. */
export type CaraMaterial = "arriba" | "abajo" | "norte" | "sur" | "oeste" | "este" | "cruz";

/** Pasada de render: opaco, recorte (alphaTest) o translúcido (mezcla alfa). */
type TipoMaterial = "opaco" | "recorte" | "translucido";

/** Fila de la tabla de materiales: el atributo `material` de cada vértice apunta aquí. */
export interface MaterialMalla {
  /** Block state sin propiedades que no cambian el aspecto. */
  estado: string;
  /** Identificador sin propiedades: `minecraft:oak_log`. */
  nombre: string;
  cara: CaraMaterial;
  tipo: TipoMaterial;
  /** Color de respaldo (RGB 0..255) mientras no haya textura. */
  color: [number, number, number];
  /** Celda del atlas, o `null` (usar `color`). */
  textura: number | null;
  /** Giro de la textura en la cara, en grados (0, 90, 180 o 270). */
  rotacion: number;
}

/** Atlas de texturas con el que se resolvieron las celdas (docs/visor.md 11.1). */
export interface DatosAtlas {
  /** Versión de Minecraft del atlas. */
  version: string;
  /** Tamaño del PNG en píxeles (potencias de dos). */
  ancho: number;
  alto: number;
  celdasPorFila: number;
  /** Lado de una celda: textura más relleno a cada lado (32). */
  tamCelda: number;
  /** Lado de la textura útil, centrada en la celda (16). */
  tamTextura: number;
  /** Píxeles de relleno a cada lado (8). */
  relleno: number;
  celdaUv: [number, number];
  /** Niveles de mipmap sin sangrado entre celdas (3). */
  nivelesMip: number;
}

/** Geometría de un chunk de 16³. Todo en 1/16 de bloque, relativo a `origen`. */
export interface ChunkMalla {
  /** Esquina mínima en bloques, coordenadas del modelo. */
  origen: Vec3;
  /** Primera y última capa Y con geometría (coordenadas del modelo, inclusivas). */
  yMin: number;
  yMax: number;
  /** x, y, z por vértice (0..256 = 0..16 bloques). */
  posiciones: Uint16Array;
  /** Índice de normal por vértice (ver NORMALES). */
  normales: Uint8Array;
  /** u, v por vértice en 1/16 de bloque: el shader hace fract(uv / 16). */
  uvs: Uint16Array;
  /** Índice en `Malla.materiales` por vértice. */
  materiales: Uint16Array;
  /** Triángulos: [0, indicesOpacos) opacos y de recorte; el resto, translúcidos. */
  indices: Uint16Array | Uint32Array;
  indicesOpacos: number;
}

export interface Malla {
  /** Esquina mínima del modelo en coordenadas del schematic. */
  origen: Vec3;
  /** Ancho (X), alto (Y) y largo (Z) del modelo en bloques. */
  tamano: Vec3;
  materiales: MaterialMalla[];
  /** Paleta común de block states (índice 0 = aire). */
  paleta: string[];
  totalBloques: number;
  /** Atlas con el que se resolvieron las texturas, o `null` (solo colores). */
  atlas: DatosAtlas | null;
  chunks: ChunkMalla[];
}

/** Bloque seleccionado con el ratón. */
export interface BloqueSeleccionado {
  estado: string;
  nombre: string;
  propiedades: Record<string, string>;
  /** Posición en coordenadas del schematic. */
  posicion: Vec3;
  region: string | null;
}

/** Propiedades que nunca cambian el aspecto (`PROPIEDADES_INVISIBLES` en Rust). */
const PROPIEDADES_INVISIBLES = new Set(["distance", "persistent", "waterlogged"]);

/**
 * Estado que identifica el aspecto, como `clave_visual` en Rust
 * (src-tauri/src/malla/apariencia.rs): el de `MaterialMalla.estado`. Quita las
 * propiedades invisibles y, en el agua y la lava, `level`. Espera un estado
 * ya normalizado (namespace y propiedades en orden), como los que devuelven
 * los comandos.
 */
export function claveVisual(estado: string): string {
  const i = estado.indexOf("[");
  if (i < 0) return estado;
  const nombre = estado.slice(0, i);
  const fluido = nombre === "minecraft:water" || nombre === "minecraft:lava";
  const resto = estado.endsWith("]") ? estado.slice(i + 1, -1) : estado.slice(i + 1);
  const props = resto.split(",").filter((par) => {
    const k = par.split("=", 1)[0];
    return !PROPIEDADES_INVISIBLES.has(k) && !(fluido && k === "level");
  });
  return props.length ? `${nombre}[${props.join(",")}]` : nombre;
}

const R = Math.SQRT1_2;

/** Vector de cada índice de normal: 0..5 caras, 6..9 planos cruzados. */
export const NORMALES: readonly Vec3[] = [
  [0, 1, 0], // 0 arriba
  [0, -1, 0], // 1 abajo
  [0, 0, -1], // 2 norte
  [0, 0, 1], // 3 sur
  [-1, 0, 0], // 4 oeste
  [1, 0, 0], // 5 este
  [R, 0, R], // 6
  [-R, 0, -R], // 7
  [R, 0, -R], // 8
  [-R, 0, R], // 9
];

const alinear4 = (n: number) => (n + 3) & ~3;

/** Decodifica el binario de `malla`. Lanza Error si no es un binario válido. Se exporta para los tests. */
export function decodificarMalla(buffer: ArrayBuffer): Malla {
  const dv = new DataView(buffer);
  const magia = String.fromCharCode(
    dv.getUint8(0),
    dv.getUint8(1),
    dv.getUint8(2),
    dv.getUint8(3),
  );
  if (magia !== MAGIA) throw new Error("La malla recibida no tiene un formato válido.");
  const version = dv.getUint32(4, true);
  if (version !== VERSION_MALLA) {
    throw new Error(`Versión de malla ${version} no soportada (se esperaba ${VERSION_MALLA}).`);
  }
  const tamCabecera = dv.getUint32(8, true);
  const nChunks = dv.getUint32(12, true);
  const origen: Vec3 = [dv.getInt32(16, true), dv.getInt32(20, true), dv.getInt32(24, true)];
  const tamano: Vec3 = [dv.getUint32(28, true), dv.getUint32(32, true), dv.getUint32(36, true)];
  const bytesJson = dv.getUint32(40, true);
  const json = JSON.parse(
    new TextDecoder().decode(new Uint8Array(buffer, tamCabecera, bytesJson)),
  ) as {
    materiales: MaterialMalla[];
    paleta: string[];
    totalBloques: number;
    atlas?: DatosAtlas | null;
  };

  let o = alinear4(tamCabecera + bytesJson);
  const chunks: ChunkMalla[] = [];
  for (let i = 0; i < nChunks; i++) {
    const c: Vec3 = [dv.getInt32(o, true), dv.getInt32(o + 4, true), dv.getInt32(o + 8, true)];
    const yMin = dv.getInt32(o + 12, true);
    const yMax = dv.getInt32(o + 16, true);
    const nv = dv.getUint32(o + 20, true);
    const ni = dv.getUint32(o + 24, true);
    const indicesOpacos = dv.getUint32(o + 28, true);
    const bytesIndice = dv.getUint32(o + 32, true);
    const bytesDatos = dv.getUint32(o + 36, true);
    let p = o + CABECERA_CHUNK;
    const posiciones = new Uint16Array(buffer, p, nv * 3);
    p = alinear4(p + nv * 6);
    const normales = new Uint8Array(buffer, p, nv);
    p = alinear4(p + nv);
    const uvs = new Uint16Array(buffer, p, nv * 2);
    p += nv * 4;
    const materiales = new Uint16Array(buffer, p, nv);
    p = alinear4(p + nv * 2);
    const indices =
      bytesIndice === 2 ? new Uint16Array(buffer, p, ni) : new Uint32Array(buffer, p, ni);
    chunks.push({ origen: c, yMin, yMax, posiciones, normales, uvs, materiales, indices, indicesOpacos });
    o += CABECERA_CHUNK + bytesDatos;
  }
  return {
    origen,
    tamano,
    materiales: json.materiales.map((m) => ({ ...m, rotacion: m.rotacion ?? 0 })),
    paleta: json.paleta,
    totalBloques: json.totalBloques,
    atlas: json.atlas ?? null,
    chunks,
  };
}

/**
 * Pide la malla de un schematic de la biblioteca. `capas` = [yMin, yMax] en
 * coordenadas del modelo (inclusivo) para mallar solo esas capas con el corte
 * cerrado; sin él, el modelo entero.
 */
export async function cargarMalla(id: string, capas?: [number, number]): Promise<Malla> {
  return pedirMalla("malla", { id, capas: capas ?? null });
}

/**
 * Llama a un comando que devuelve el binario de una malla (`malla` o
 * `malla_miniatura`) y lo decodifica.
 */
export async function pedirMalla(orden: string, args: Record<string, unknown>): Promise<Malla> {
  const respuesta = await llamar<unknown>(orden, args);
  const buffer =
    respuesta instanceof ArrayBuffer
      ? respuesta
      : respuesta instanceof Uint8Array
        ? respuesta.slice().buffer
        : new Uint8Array(respuesta as number[]).buffer;
  return decodificarMalla(buffer);
}

/** Bloque en una celda del modelo (las coordenadas de los vértices), o `null` si es aire. */
export async function bloqueEn(
  id: string,
  x: number,
  y: number,
  z: number,
): Promise<BloqueSeleccionado | null> {
  return llamar<BloqueSeleccionado | null>("bloque_en", { id, x, y, z });
}

/**
 * Celda del modelo que contiene el punto de impacto de un raycast: retrocede
 * una milésima de bloque contra la normal de la cara y redondea hacia abajo.
 * (Con media unidad fallaría en caras finas como alfombras o placas.)
 * `punto` y `normal` en coordenadas del modelo (bloques).
 */
export function celdaImpactada(punto: Vec3, normal: Vec3): Vec3 {
  const e = 1e-3;
  return [
    Math.floor(punto[0] - normal[0] * e),
    Math.floor(punto[1] - normal[1] * e),
    Math.floor(punto[2] - normal[2] * e),
  ];
}

/** ¿Tiene el chunk algo dentro del rango de capas [desde, hasta]? */
export const chunkEnCapas = (c: ChunkMalla, desde: number, hasta: number): boolean =>
  c.yMax >= desde && c.yMin <= hasta;
