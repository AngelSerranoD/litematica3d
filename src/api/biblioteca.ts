// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { invoke } from "@tauri-apps/api/core";
import { gruposDeMiles } from "../componentes/formato";

/** Dimensiones de un schematic en bloques (ejes de Minecraft: X, Y, Z). */
export interface Dimensiones {
  ancho: number;
  alto: number;
  largo: number;
}

export type Formato = "litematic" | "sponge" | "mcedit" | "estructura";

export interface Entrada {
  id: string;
  nombre: string;
  autor: string | null;
  descripcion: string | null;
  formato: Formato;
  dimensiones: Dimensiones;
  totalBloques: number;
  regiones: number;
  versionDatos: number | null;
  /** Milisegundos desde la época Unix. */
  fechaCreacion: number | null;
  fechaImportacion: number;
  nombreOriginal: string | null;
  tamanoArchivo: number;
  /** Miniatura isométrica, o `null` si aún no se ha generado (docs/miniaturas.md). */
  miniatura: Miniatura | null;
}

/** PNG de la miniatura guardado junto al schematic en la biblioteca. */
export interface Miniatura {
  /** Ruta absoluta del PNG (se enseña con el protocolo `asset`). */
  ruta: string;
  /** Milisegundos desde la época Unix: cuándo se escribió. */
  fecha: number;
  /** Clave de las texturas con que se generó; `null` = obsoleta. */
  texturas: string | null;
}

export interface Region {
  nombre: string;
  origen: [number, number, number];
  tamano: Dimensiones;
  tamanoPaleta: number;
  totalBloques: number;
}

export interface Detalle {
  entrada: Entrada;
  regiones: Region[];
}

export type TipoError =
  | "archivoCorrupto"
  | "demasiadoGrande"
  | "formatoDesconocido"
  | "noEncontrado"
  | "regionNoEncontrada"
  | "archivoPerdido"
  | "nombreInvalido"
  | "io"
  | "baseDatos"
  | "papelera"
  | "versionBiblioteca"
  | "carpetaMinecraft"
  | "miniaturaInvalida"
  | "edicionInvalida"
  | "versionNoEncontrada"
  | "interno";

/** Error de los comandos de Rust; `mensaje` viene en español, listo para enseñar. */
export interface ErrorApp {
  tipo: TipoError;
  mensaje: string;
}

/** Qué contar en la lista de materiales. */
export interface OpcionesMateriales {
  /** Contar agua y lava (solo fuentes) como cubos. */
  incluirFluidos: boolean;
  /** Solo esta región; `null`, todas. */
  region: string | null;
  /** Rango de alturas inclusivo, en coordenadas Y del schematic (las de `Region.origen`). */
  yMin: number | null;
  yMax: number | null;
  /**
   * Rango de capas `[desde, hasta]` inclusivo en coordenadas del MODELO (las
   * del visor y de `malla`); el núcleo suma el origen Y. `null`, sin límite.
   */
  capas?: [number, number] | null;
}

/** Un ítem de la lista de materiales con su desglose. */
export interface Material {
  /** ID del ítem (`minecraft:…`); `litematica3d:desconocido` agrupa los IDs sin traducción. */
  item: string;
  cantidad: number;
  tamanoStack: number;
  /** `stacks × tamanoStack + sueltos = cantidad`. */
  stacks: number;
  sueltos: number;
  /** Cajas de shulker llenas (27 stacks) y lo que queda fuera de ellas. */
  shulkers: number;
  stacksEnShulker: number;
  sueltosEnShulker: number;
}

export interface ListaMateriales {
  /** De más a menos cantidad. */
  materiales: Material[];
  totalItems: number;
  /** Huecos de inventario necesarios. */
  totalStacks: number;
  /** Cajas de shulker para llevarlo todo mezclado. */
  totalShulkers: number;
  /** Bloques con ID antiguo sin traducción. */
  desconocidos: number;
}

export type ResultadoImportacion =
  | { estado: "importado"; entrada: Entrada }
  | { estado: "duplicado"; entrada: Entrada }
  | { estado: "error"; ruta: string; error: ErrorApp };

/** Convierte lo que rechace `invoke` en un ErrorApp (tipo «interno» si no lo es). */
export function normalizarError(e: unknown): ErrorApp {
  if (
    typeof e === "object" &&
    e !== null &&
    "tipo" in e &&
    "mensaje" in e &&
    typeof e.tipo === "string" &&
    typeof e.mensaje === "string"
  ) {
    return e as ErrorApp;
  }
  const mensaje =
    typeof e === "string" ? e : e instanceof Error ? e.message : "Error inesperado.";
  return { tipo: "interno", mensaje };
}

async function llamar<T>(orden: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(orden, args);
  } catch (e) {
    throw normalizarError(e);
  }
}

export const importar = (rutas: string[]) =>
  llamar<ResultadoImportacion[]>("importar", { rutas });
export const listar = () => llamar<Entrada[]>("listar");
export const obtener = (id: string) => llamar<Detalle>("obtener", { id });
export const renombrar = (id: string, nombre: string) =>
  llamar<Entrada>("renombrar", { id, nombre });
export const eliminar = (id: string) => llamar<null>("eliminar", { id });
export const materiales = (id: string, opciones: OpcionesMateriales) =>
  llamar<ListaMateriales>("materiales", { id, opciones });

/**
 * Número en texto con los mismos grupos que `Cifra`, separados por un espacio
 * fino irrompible: «2 932», «12 345».
 */
export const formatoNumero = (n: number): string => gruposDeMiles(n).join(" ");

/** «9 × 31 × 9»: ancho × alto × largo. */
export function formatoDimensiones({ ancho, alto, largo }: Dimensiones): string {
  return `${ancho} × ${alto} × ${largo}`;
}

const NOMBRES_FORMATO: Record<Formato, string> = {
  litematic: "Litematica (.litematic)",
  sponge: "Sponge (.schem)",
  mcedit: "MCEdit (.schematic)",
  estructura: "Estructura (.nbt)",
};

export const nombreFormato = (f: Formato): string => NOMBRES_FORMATO[f];
