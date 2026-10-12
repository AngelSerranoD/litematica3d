// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Texturas y ajustes de Minecraft: comandos de Rust y la caché del atlas en el
// frontend (docs/visor.md, sección 11). Aquí no se usa Three.js: la textura de
// GPU la crea el visor a partir del ImageBitmap, así este módulo queda en el
// trozo principal y los ajustes no arrastran a Three.

import { invoke } from "@tauri-apps/api/core";
import { normalizarError } from "./biblioteca";
import type { DatosAtlas } from "./visor";

export interface VersionMinecraft {
  id: string;
  ruta: string;
  esRelease: boolean;
}

export interface Ajustes {
  /** Versión elegida, o `null` para la primera de la lista. */
  versionMinecraft: string | null;
  rutaMinecraft?: string;
}

async function llamar<T>(orden: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(orden, args);
  } catch (e) {
    throw normalizarError(e);
  }
}

export const versionesMinecraft = () => llamar<VersionMinecraft[]>("versiones_minecraft");
export const ajustes = () => llamar<Ajustes>("ajustes");
export const guardarAjustes = (a: Ajustes) => llamar<null>("guardar_ajustes", { ajustes: a });
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
