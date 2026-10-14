// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Edición de schematics con historial para deshacer (docs/edicion.md):
// aplicar cambios a la copia de la biblioteca, ver y restaurar versiones
// anteriores y exportar como .litematic.

import type { Entrada, Formato } from "./biblioteca";
import { llamar } from "./llamar";

/**
 * Cambia el bloque de una posición. Los block states van como texto canónico
 * (`minecraft:oak_stairs[facing=north,half=bottom]`, el de `bloque_en`).
 */
export interface Cambio {
  /** Nombre de la región (`Region.nombre` o `BloqueSeleccionado.region`). */
  region: string;
  /** Coordenadas del schematic (las de `BloqueSeleccionado.posicion`), no las del modelo. */
  x: number;
  y: number;
  z: number;
  /** `minecraft:air` para quitar el bloque. */
  nuevoEstado: string;
}

/** Cambia todas las apariciones de un block state exacto, en todas las regiones. */
export interface Reemplazo {
  estadoOrigen: string;
  estadoDestino: string;
}

/** Versiones que guarda el historial por schematic (`biblioteca/historial.rs`). */
export const MAX_VERSIONES = 20;

/** Qué sustituyó a una versión del historial. */
type Motivo = "aplicar" | "restaurar";

/** Versión anterior de un schematic. */
export interface Version {
  /** Identificador para `restaurarVersion` (marca en ms, única en el schematic). */
  version: number;
  /** Milisegundos desde la época Unix: cuándo dejó de ser la actual. */
  fecha: number;
  /** Formato del archivo guardado (el original puede no ser .litematic). */
  formato: Formato;
  motivo: Motivo;
  /** Posiciones que cambió la operación que la sustituyó (0 al restaurar). */
  cambios: number;
  totalBloques: number;
  tamanoArchivo: number;
}

/** Resultado de aplicar cambios o restaurar una versión. */
export interface Edicion {
  /** Entrada al día; su miniatura queda obsoleta (`texturas: null`). */
  entrada: Entrada;
  /** Posiciones que han cambiado de verdad (0 al restaurar). */
  cambios: number;
  /** Versión donde quedó lo de antes (deshacer = restaurarla); `null` si nada cambió. */
  version: number | null;
}

/**
 * Aplica los reemplazos (en orden) y después los cambios puntuales (gana el
 * último en cada posición). Guarda como .litematic; lo anterior va al historial.
 */
export const aplicarCambios = (id: string, cambios: Cambio[], reemplazos: Reemplazo[] = []) =>
  llamar<Edicion>("aplicar_cambios", { id, cambios, reemplazos });

/** Versiones anteriores, de la más reciente a la más antigua (máximo `MAX_VERSIONES`). */
export const historial = (id: string) => llamar<Version[]>("historial", { id });

/** Vuelve a una versión; la actual pasa antes al historial (para rehacer). */
export const restaurarVersion = (id: string, version: number) =>
  llamar<Edicion>("restaurar_version", { id, version });

/**
 * Pide dónde exportar y escribe la copia actual como .litematic. Devuelve la
 * ruta elegida, o `null` si el usuario cancela.
 */
export const exportarSchematic = (id: string, nombre: string) =>
  llamar<string | null>("exportar_schematic", { id, nombre });
