// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";
import type { BloqueSeleccionado, Vec3 } from "../api/visor";

/**
 * Estado del visor 3D: corte por capas, selección y encargos a la escena.
 *
 * Todo lo que se lee con selectores es un número, un booleano o una
 * referencia que solo cambia cuando cambia el dato (`seleccion`): ningún
 * selector construye arrays u objetos nuevos, que harían repintar en bucle.
 * El rango efectivo se pide con `capaMin` y `capaMax` por separado.
 */

/** Pulgar del deslizador que se movió por última vez. */
export type Pulgar = "desde" | "hasta";

export interface Seleccion {
  /** Celda en coordenadas del modelo. */
  celda: Vec3;
  /** `null` mientras llega la respuesta de `bloque_en`. */
  bloque: BloqueSeleccionado | null;
}

interface EstadoVisor {
  /** Número de capas del modelo (alto en bloques). */
  alto: number;
  /** Rango elegido con los dos pulgares (coordenadas del modelo, inclusivo). */
  desde: number;
  hasta: number;
  /** «Solo esta capa»: el rango se reduce a `capa`. */
  soloCapa: boolean;
  capa: number;
  activo: Pulgar;
  /** Se está arrastrando el deslizador: no se remalla hasta soltar. */
  arrastrando: boolean;
  seleccion: Seleccion | null;
  /** Cambia en cada petición de encuadre (botón o tecla F). */
  encuadre: number;
  /** Panel de rendimiento (solo en desarrollo). */
  rendimiento: boolean;

  reiniciar: (alto: number) => void;
  fijarDesde: (capa: number) => void;
  fijarHasta: (capa: number) => void;
  fijarCapa: (capa: number) => void;
  alternarSoloCapa: () => void;
  fijarArrastre: (arrastrando: boolean) => void;
  seleccionar: (celda: Vec3) => void;
  completarSeleccion: (celda: Vec3, bloque: BloqueSeleccionado | null) => void;
  limpiarSeleccion: () => void;
  pedirEncuadre: () => void;
  alternarRendimiento: () => void;
}

const acotar = (v: number, min: number, max: number) => Math.min(Math.max(Math.round(v), min), max);

export const useVisor = create<EstadoVisor>()((set, get) => ({
  alto: 1,
  desde: 0,
  hasta: 0,
  soloCapa: false,
  capa: 0,
  activo: "hasta",
  arrastrando: false,
  seleccion: null,
  encuadre: 0,
  rendimiento: false,

  reiniciar: (alto) => {
    const max = Math.max(alto - 1, 0);
    set({
      alto: Math.max(alto, 1),
      desde: 0,
      hasta: max,
      soloCapa: false,
      capa: max,
      activo: "hasta",
      arrastrando: false,
      seleccion: null,
    });
  },
  fijarDesde: (capa) => set((s) => ({ desde: acotar(capa, 0, s.hasta), activo: "desde" })),
  fijarHasta: (capa) => set((s) => ({ hasta: acotar(capa, s.desde, s.alto - 1), activo: "hasta" })),
  fijarCapa: (capa) => set((s) => ({ capa: acotar(capa, 0, s.alto - 1) })),
  alternarSoloCapa: () => {
    const s = get();
    if (s.soloCapa) set({ soloCapa: false });
    else set({ soloCapa: true, capa: s.activo === "desde" ? s.desde : s.hasta });
  },
  fijarArrastre: (arrastrando) => set({ arrastrando }),
  seleccionar: (celda) => set({ seleccion: { celda, bloque: null } }),
  completarSeleccion: (celda, bloque) => {
    const actual = get().seleccion;
    // Respuesta tardía de un clic anterior: se descarta.
    if (!actual || actual.celda.some((v, i) => v !== celda[i])) return;
    set({ seleccion: bloque ? { celda, bloque } : null });
  },
  limpiarSeleccion: () => {
    if (get().seleccion) set({ seleccion: null });
  },
  pedirEncuadre: () => set((s) => ({ encuadre: s.encuadre + 1 })),
  alternarRendimiento: () => set((s) => ({ rendimiento: !s.rendimiento })),
}));

/** Primera capa visible (selector atómico). */
export const capaMin = (s: EstadoVisor) => (s.soloCapa ? s.capa : s.desde);
/** Última capa visible, inclusiva (selector atómico). */
export const capaMax = (s: EstadoVisor) => (s.soloCapa ? s.capa : s.hasta);
