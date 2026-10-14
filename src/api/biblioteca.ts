// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { listen } from "@tauri-apps/api/event";
import { llamar } from "./llamar";

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

interface Detalle {
  entrada: Entrada;
  regiones: Region[];
}

type TipoError =
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
interface OpcionesMateriales {
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
  /** ID del ítem (`minecraft:…`); `schematiclab:desconocido` agrupa los IDs sin traducción. */
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

export const importar = (rutas: string[]) =>
  llamar<ResultadoImportacion[]>("importar", { rutas });
export const listar = () => llamar<Entrada[]>("listar");
export const obtener = (id: string) => llamar<Detalle>("obtener", { id });
export const renombrar = (id: string, nombre: string) =>
  llamar<Entrada>("renombrar", { id, nombre });
export const eliminar = (id: string) => llamar<null>("eliminar", { id });
export const materiales = (id: string, opciones: OpcionesMateriales) =>
  llamar<ListaMateriales>("materiales", { id, opciones });

/** Evento con que Rust avisa de un archivo abierto con la app ya en marcha. */
export const EVENTO_ABRIR_ARCHIVO = "abrir-archivo";

/** Ruta del archivo con que se arrancó la app (doble clic en el Explorador), si lo hubo. */
export const archivoPendiente = () => llamar<string | null>("archivo_pendiente");

/** Escucha los archivos que llegan con la app abierta. Devuelve la función de baja. */
export const alAbrirArchivo = (cb: (ruta: string) => void): Promise<() => void> =>
  listen<string>(EVENTO_ABRIR_ARCHIVO, (e) => cb(e.payload));
