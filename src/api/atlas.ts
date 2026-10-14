// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Texturas y ajustes de Minecraft: comandos de Rust y la caché del atlas en el
// frontend (docs/visor.md, sección 11). Aquí no se usa Three.js: la textura de
// GPU la crea el visor a partir del ImageBitmap, así este módulo queda en el
// trozo principal y los ajustes no arrastran a Three.

import { llamar } from "./llamar";
import type { DatosAtlas } from "./visor";

export interface VersionMinecraft {
  id: string;
  ruta: string;
  esRelease: boolean;
}

/** Tema de la interfaz; «sistema» sigue `prefers-color-scheme`. */
export type Tema = "claro" | "oscuro" | "sistema";

/** `ajustes.json` (docs/visor.md, sección 11.1). */
export interface Ajustes {
  /** Versión elegida, o `null` para la automática (la primera de la lista). */
  versionMinecraft: string | null;
  /** Carpeta `.minecraft` elegida a mano; sin ella, la habitual. */
  rutaMinecraft?: string;
  tema: Tema;
  /** Contar agua y lava en la lista de materiales. */
  incluirFluidos: boolean;
}

export const AJUSTES_POR_DEFECTO: Ajustes = {
  versionMinecraft: null,
  tema: "sistema",
  incluirFluidos: false,
};

export const versionesMinecraft = () => llamar<VersionMinecraft[]>("versiones_minecraft");
export const ajustes = () => llamar<Ajustes>("ajustes");
/**
 * Guarda los ajustes. `true` si cambian las texturas (la versión que se usa
 * de verdad ya no es la misma): hay que llamar a `texturasCambiadas()`.
 * Una carpeta `.minecraft` que no vale da un error `carpetaMinecraft`.
 */
export const guardarAjustes = (a: Ajustes) => llamar<boolean>("guardar_ajustes", { ajustes: a });
/** Carpeta `.minecraft` que se usa sin elegir ninguna, o `null` si no hay. */
export const carpetaMinecraftPorDefecto = () =>
  llamar<string | null>("carpeta_minecraft_por_defecto");
export const atlasInfo = () => llamar<DatosAtlas | null>("atlas_info");

/** Atlas ya decodificado: la imagen (v = 0 abajo) y sus datos. */
export interface AtlasCargado {
  info: DatosAtlas;
  imagen: ImageBitmap;
}

/** Un solo atlas en memoria para toda la app. */
let cargado: AtlasCargado | null = null;
let enCurso: Promise<AtlasCargado | null> | null = null;
/** Sube con cada invalidación: descarta lo que estaba en vuelo. */
let generacion = 0;

/** Tira el atlas en memoria (al cambiar de versión o de carpeta de Minecraft). */
export function invalidarAtlas(): void {
  generacion++;
  cargado?.imagen.close();
  cargado = null;
  enCurso = null;
}

async function descargar(): Promise<AtlasCargado | null> {
  const bytes = await llamar<ArrayBuffer | Uint8Array | number[]>("atlas");
  const buffer = bytes instanceof ArrayBuffer ? bytes : new Uint8Array(bytes);
  if (buffer.byteLength === 0) return null; // sin jar: colores planos
  // flipY aquí (no en la textura): con ImageBitmap Three ignora texture.flipY.
  // Así v = 0 es el borde de abajo del PNG, como supone la fórmula de 11.2.
  const imagen = await createImageBitmap(new Blob([buffer], { type: "image/png" }), {
    imageOrientation: "flipY",
    premultiplyAlpha: "none",
    colorSpaceConversion: "none",
  });
  // Justo después de `atlas`: describe el atlas que acaba de llegar.
  const info = await atlasInfo();
  if (!info) {
    imagen.close();
    return null;
  }
  return { info, imagen };
}

/**
 * Atlas de la versión activa, cacheado. Con `version`, exige esa (la de la
 * malla): si la caché tiene otra, lo vuelve a pedir. `null` si no hay jar.
 * La primera vez tarda lo que tarde en generarse.
 */
export function obtenerAtlas(version?: string): Promise<AtlasCargado | null> {
  if (cargado && (!version || cargado.info.version === version)) return Promise.resolve(cargado);
  if (enCurso) return enCurso;
  const mia = generacion;
  const peticion = descargar().then((a) => {
    if (mia !== generacion) {
      a?.imagen.close();
      return obtenerAtlas(version); // se invalidó mientras tanto: otra vez
    }
    cargado = a;
    return a;
  });
  enCurso = peticion;
  const limpiar = () => {
    if (enCurso === peticion) enCurso = null;
  };
  peticion.then(limpiar, limpiar);
  return peticion;
}

/** Textura de una cara de un block state en el atlas activo. */
export interface CeldaAtlas {
  celda: number;
  /** Giro en grados (0, 90, 180 o 270), como `MaterialMalla.rotacion`. */
  rotacion: number;
  /** Color medio de la textura (con el tinte aplicado). */
  color: [number, number, number];
}

/** Orden de las caras en cada fila de `CeldasAtlas.celdas` (el de `Cara` en Rust). */
export const CARAS_ATLAS = ["arriba", "abajo", "norte", "sur", "oeste", "este", "cruz"] as const;

interface CeldasAtlas {
  /** Atlas con el que se resolvieron: su `version` debe casar con el cargado. */
  atlas: DatosAtlas;
  /** Una fila por estado pedido (7 caras, `CARAS_ATLAS`); vacía si el estado no es válido. */
  celdas: (CeldaAtlas | null)[][];
}

/** Como mucho tantos estados por llamada a `celdasAtlas`. */
export const MAX_CELDAS_ATLAS = 256;

/**
 * Celdas del atlas activo para block states cualesquiera (estén o no en el
 * schematic): iconos de los parecidos y previsualización de los pendientes.
 * `null` si el atlas aún no está listo o no hay jar (no genera nada).
 */
export const celdasAtlas = (estados: string[]) =>
  llamar<CeldasAtlas | null>("celdas_atlas", { estados });

/**
 * Color plano de block states cualesquiera, el de la malla sin atlas
 * (previsualización de los pendientes sin jar). `null` donde el estado no es
 * válido. Como mucho `MAX_CELDAS_ATLAS` estados por llamada.
 */
export const coloresBloques = (estados: string[]) =>
  llamar<([number, number, number] | null)[]>("colores_bloques", { estados });
