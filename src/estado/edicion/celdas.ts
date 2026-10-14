// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Trozo del almacén de edición: celdas del atlas y colores planos de los block states.

import { celdasAtlas, coloresBloques, MAX_CELDAS_ATLAS, type CeldaAtlas } from "../../api/atlas";
import type { Rgb } from "../../api/parecidos";
import type { EstadoEdicion, Trozo } from "./tipos";

type TrozoCeldas = Pick<
  EstadoEdicion,
  "celdas" | "versionCeldas" | "colores" | "pedirCeldas" | "pedirColores"
>;

/** Colores planos en vuelo. */
const pidiendoColores = new Set<string>();
/** Estados cuyas celdas están en vuelo. */
const pidiendoCeldas = new Set<string>();
/** Estados por pedir en la próxima tanda (se juntan las peticiones de un mismo pintado). */
const colaCeldas = new Set<string>();
let tandaProgramada = false;

/** Con otras texturas, lo que quedaba por pedir ya no vale. */
export function olvidarColaCeldas() {
  colaCeldas.clear();
}

/** Caras preferidas para el icono de un bloque: el lateral identifica casi todo. */
const ORDEN_ICONO = [3, 2, 5, 4, 6, 0, 1]; // sur, norte, este, oeste, cruz, arriba, abajo

/** Celda que representa a un bloque en su icono, o `null` si no tiene textura. */
export function celdaIcono(fila: (CeldaAtlas | null)[] | undefined): CeldaAtlas | null {
  if (!fila) return null;
  for (const i of ORDEN_ICONO) {
    const c = fila[i];
    if (c) return c;
  }
  return null;
}

export const trozoCeldas: Trozo<TrozoCeldas> = (set, get) => ({
  celdas: {},
  versionCeldas: null,
  colores: {},

  pedirCeldas: (estados) => {
    const { celdas } = get();
    for (const e of estados) if (!(e in celdas) && !pidiendoCeldas.has(e)) colaCeldas.add(e);
    if (!colaCeldas.size || tandaProgramada) return;
    tandaProgramada = true;
    queueMicrotask(() => {
      tandaProgramada = false;
      const faltan = [...colaCeldas].slice(0, MAX_CELDAS_ATLAS);
      faltan.forEach((e) => {
        colaCeldas.delete(e);
        pidiendoCeldas.add(e);
      });
      // Lo que no cupo en esta tanda va en la siguiente.
      if (colaCeldas.size) get().pedirCeldas([]);
      celdasAtlas(faltan)
        .then((r) => {
          // Sin atlas todavía (o sin jar): los iconos se quedan en color plano.
          if (!r) return;
          const nuevas: Record<string, (CeldaAtlas | null)[]> = {};
          faltan.forEach((e, i) => {
            nuevas[e] = r.celdas[i] ?? [];
          });
          // Otra versión del atlas (cambiaron las texturas): lo de antes no vale.
          set((s) =>
            s.versionCeldas === r.atlas.version
              ? { celdas: { ...s.celdas, ...nuevas } }
              : { celdas: nuevas, versionCeldas: r.atlas.version },
          );
        })
        .catch((e) => console.warn("No se pudieron leer las celdas del atlas:", e))
        .finally(() => faltan.forEach((e) => pidiendoCeldas.delete(e)));
    });
  },

  pedirColores: (estados) => {
    const { colores } = get();
    const faltan = [...new Set(estados)].filter((e) => !(e in colores) && !pidiendoColores.has(e));
    for (let i = 0; i < faltan.length; i += MAX_CELDAS_ATLAS) {
      const tanda = faltan.slice(i, i + MAX_CELDAS_ATLAS);
      tanda.forEach((e) => pidiendoColores.add(e));
      coloresBloques(tanda)
        .then((r) => {
          const nuevos: Record<string, Rgb | null> = {};
          tanda.forEach((e, j) => {
            nuevos[e] = r[j] ?? null;
          });
          set((s) => ({ colores: { ...s.colores, ...nuevos } }));
        })
        .catch((e) => console.warn("No se pudieron leer los colores de los bloques:", e))
        .finally(() => tanda.forEach((e) => pidiendoColores.delete(e)));
    }
  },
});
