// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";
import {
  ajustes as leerAjustes,
  atlasInfo,
  guardarAjustes,
  invalidarAtlas,
  versionesMinecraft,
  type Ajustes,
  type VersionMinecraft,
} from "../api/atlas";
import type { DatosAtlas } from "../api/visor";

/**
 * Estado del panel de ajustes (versión de Minecraft para las texturas).
 *
 * Los selectores que usa el panel devuelven valores primitivos o referencias
 * que solo cambian al cambiar el dato: nada construye arrays nuevos.
 */

/** Estado del atlas de texturas de la versión elegida. */
export type EstadoAtlas =
  | { tipo: "comprobando" }
  | { tipo: "sinJar" }
  | { tipo: "preparando" }
  | { tipo: "listo"; datos: DatosAtlas };

interface EstadoAjustesStore {
  abierto: boolean;
  cargando: boolean;
  guardando: boolean;
  error: string | null;
  versiones: VersionMinecraft[];
  /** Ajustes guardados en disco. */
  guardados: Ajustes | null;
  /** Versión marcada en el selector (aún sin guardar). */
  elegida: string | null;
  atlas: EstadoAtlas;

  abrir: () => Promise<void>;
  cerrar: () => void;
  elegir: (id: string) => void;
  guardar: () => Promise<void>;
}

const mensajeDe = (e: unknown) =>
  typeof e === "object" && e !== null && "mensaje" in e && typeof e.mensaje === "string"
    ? e.mensaje
    : "Error inesperado.";

/** Versión que se usa de verdad: la guardada si está instalada; si no, la primera. */
export function versionEfectiva(versiones: VersionMinecraft[], guardada: string | null): string | null {
  if (guardada && versiones.some((v) => v.id === guardada)) return guardada;
  return versiones[0]?.id ?? null;
}

/** Espera (ms) entre consultas al atlas mientras se prepara, y cuántas veces. */
const ESPERA_ATLAS = 1000;
const INTENTOS_ATLAS = 20;
/** Cambia en cada consulta nueva: la anterior deja de actualizar el estado. */
let consulta = 0;

export const useAjustes = create<EstadoAjustesStore>()((set, get) => {
  /** Mira si el atlas está listo; si no, reintenta mientras se prepara. */
  const vigilarAtlas = async (hayJar: boolean) => {
    const n = ++consulta;
    if (!hayJar) {
      set({ atlas: { tipo: "sinJar" } });
      return;
    }
    set({ atlas: { tipo: "comprobando" } });
    for (let i = 0; i < INTENTOS_ATLAS; i++) {
      let datos: DatosAtlas | null = null;
      try {
        datos = await atlasInfo();
      } catch {
        // Se vuelve a intentar.
      }
      if (n !== consulta) return;
      if (datos) {
        set({ atlas: { tipo: "listo", datos } });
        return;
      }
      set({ atlas: { tipo: "preparando" } });
      await new Promise((r) => window.setTimeout(r, ESPERA_ATLAS));
      if (n !== consulta) return;
    }
  };

  return {
    abierto: false,
    cargando: false,
    guardando: false,
    error: null,
    versiones: [],
    guardados: null,
    elegida: null,
    atlas: { tipo: "comprobando" },

    abrir: async () => {
      if (get().abierto) return;
      set({ abierto: true, cargando: true, error: null });
      try {
        const [versiones, guardados] = await Promise.all([versionesMinecraft(), leerAjustes()]);
        set({
          versiones,
          guardados,
          elegida: versionEfectiva(versiones, guardados.versionMinecraft),
          cargando: false,
        });
        void vigilarAtlas(versiones.length > 0);
      } catch (e) {
        set({ cargando: false, error: mensajeDe(e) });
      }
    },

    cerrar: () => {
      consulta++;
      set({ abierto: false });
    },

    elegir: (id) => set({ elegida: id, error: null }),

    guardar: async () => {
      const { guardados, elegida, versiones, guardando } = get();
      if (guardando || !guardados || !elegida) return;
      set({ guardando: true, error: null });
      try {
        const nuevos: Ajustes = { ...guardados, versionMinecraft: elegida };
        await guardarAjustes(nuevos);
        // El atlas en memoria era de la versión anterior.
        invalidarAtlas();
        set({ guardados: nuevos, guardando: false });
        void vigilarAtlas(versiones.length > 0);
      } catch (e) {
        set({ guardando: false, error: mensajeDe(e) });
      }
    },
  };
});

/** ¿Hay un cambio de versión sin guardar? (selector atómico) */
export const hayCambios = (s: EstadoAjustesStore) =>
  s.guardados !== null &&
  s.elegida !== null &&
  s.elegida !== versionEfectiva(s.versiones, s.guardados.versionMinecraft);
