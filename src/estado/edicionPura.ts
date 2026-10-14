// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Lógica pura del modo Modificar (sin Tauri ni React), para poder probarla.

/** Lo mínimo de un cambio suelto pendiente que hace falta para contar. */
interface CambioParaContar {
  estadoAnterior: string;
}

/** Lo mínimo de un reemplazo pendiente que hace falta para contar. */
interface ReemplazoParaContar {
  estadoOrigen: string;
  estadoDestino: string;
  cantidad: number | null;
}

/**
 * Posiciones que tocarían los pendientes (los reemplazos sin cantidad no suman).
 *
 * Cuenta posiciones únicas, como `aplicar` en Rust (que aplica los reemplazos
 * y luego los cambios sueltos, y devuelve las posiciones cambiadas): un cambio
 * suelto sobre un bloque cuyo estado actual es el origen de un reemplazo
 * pendiente cae dentro de ese reemplazo, así que no suma otra vez.
 */
/** Etiqueta de la acción inversa: tras Deshacer se ofrece Rehacer y viceversa. */
export type EtiquetaInversa = "Deshacer" | "Rehacer";
export const inversaDe = (hecha: EtiquetaInversa): EtiquetaInversa =>
  hecha === "Deshacer" ? "Rehacer" : "Deshacer";

export function bloquesAfectados(
  cambios: CambioParaContar[],
  reemplazos: ReemplazoParaContar[],
): { total: number; exacto: boolean } {
  const origenes = new Set<string>();
  let total = 0;
  let exacto = true;
  for (const r of reemplazos) {
    if (r.cantidad === null) {
      exacto = false;
      continue;
    }
    total += r.cantidad;
    if (r.estadoOrigen !== r.estadoDestino) origenes.add(r.estadoOrigen);
  }
  for (const c of cambios) {
    if (!origenes.has(c.estadoAnterior)) total++;
  }
  return { total, exacto };
}

/** `minecraft:stone_stairs[facing=east]` → id y propiedades. */
function separarEstado(estado: string): { id: string; props: Map<string, string> } {
  const i = estado.indexOf("[");
  const props = new Map<string, string>();
  if (i < 0 || !estado.endsWith("]")) return { id: estado, props };
  for (const par of estado.slice(i + 1, -1).split(",")) {
    const [k, v = ""] = par.split("=");
    if (k) props.set(k, v);
  }
  return { id: estado.slice(0, i), props };
}

/** Id del bloque sin propiedades de estado. */
const idDeEstado = (estado: string) => separarEstado(estado).id;

/** Un bloque de la tabla de familia: todos sus estados presentes, sumados. */
export interface GrupoFamilia {
  /** Id del bloque (clave de la fila). */
  id: string;
  /** Estados concretos del grupo (el primero es el representante). */
  estados: string[];
  /** Posiciones que ocupan todos sus estados. */
  cantidad: number;
}

/** Agrupa los estados presentes por id de bloque, en orden de aparición. */
export function agruparMiembros(miembros: { estado: string; cantidad: number }[]): GrupoFamilia[] {
  const grupos = new Map<string, GrupoFamilia>();
  for (const m of miembros) {
    const id = idDeEstado(m.estado);
    const g = grupos.get(id);
    if (g) {
      g.estados.push(m.estado);
      g.cantidad += m.cantidad;
    } else {
      grupos.set(id, { id, estados: [m.estado], cantidad: m.cantidad });
    }
  }
  return [...grupos.values()];
}
