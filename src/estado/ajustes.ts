// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";
import {
  AJUSTES_POR_DEFECTO,
  ajustes as leerAjustes,
  atlasInfo,
  carpetaMinecraftPorDefecto,
  guardarAjustes,
  versionesMinecraft,
  type Ajustes,
  type Tema,
  type VersionMinecraft,
} from "../api/atlas";
import { normalizarError } from "../api/llamar";
import type { DatosAtlas } from "../api/visor";
import { texturasCambiadas } from "./texturas";

/**
 * Ajustes de la app: tema, carpeta `.minecraft`, versión de las texturas e
 * incluir fluidos en los materiales. Viven en `ajustes.json` (lo gestiona
 * Rust, docs/visor.md 11.1); aquí está la copia del frontend.
 *
 * Cada cambio se guarda al momento (no hay botón «Guardar»): el tema y los
 * fluidos se aplican antes de que vuelva Rust y, si Rust lo rechaza, se
 * deshacen. Los guardados van en cola, uno detrás de otro.
 */

/** Estado del atlas de texturas de la versión elegida. */
export type EstadoAtlas =
  | { tipo: "comprobando" }
  | { tipo: "sinJar" }
  | { tipo: "preparando" }
  | { tipo: "listo"; datos: DatosAtlas };

interface EstadoAjustesStore {
  abierto: boolean;
  /** Leyendo las versiones instaladas (al abrir o al cambiar de carpeta). */
  cargando: boolean;
  /** Hay un guardado en marcha. */
  guardando: boolean;
  error: string | null;
  versiones: VersionMinecraft[];
  /** Carpeta `.minecraft` que se usa sin elegir ninguna (`null` si no hay). */
  carpetaPorDefecto: string | null;
  /** Ajustes en vigor (los de por defecto hasta que llegan de Rust). */
  guardados: Ajustes;
  atlas: EstadoAtlas;

  /** Lee los ajustes al arrancar y aplica el tema. */
  iniciar: () => Promise<void>;
  abrir: () => Promise<void>;
  cerrar: () => void;
  /** Cambia uno o varios ajustes y los guarda. */
  cambiar: (cambios: Partial<Ajustes>) => Promise<void>;
}

/** Versión que se usa de verdad: la guardada si está instalada; si no, la primera. */
export function versionEfectiva(versiones: VersionMinecraft[], guardada: string | null): string | null {
  if (guardada && versiones.some((v) => v.id === guardada)) return guardada;
  return versiones[0]?.id ?? null;
}

// ── Tema ──

/** Copia del tema en localStorage solo para pintar el primer fotograma sin parpadeo. */
const CLAVE_TEMA = "schematiclab.tema";

/** Pone `data-tema` en <html> (sin él, manda `prefers-color-scheme`). */
function aplicarTema(tema: Tema): void {
  const raiz = document.documentElement;
  if (tema === "sistema") delete raiz.dataset.tema;
  else raiz.dataset.tema = tema;
  try {
    localStorage.setItem(CLAVE_TEMA, tema);
  } catch {
    // Sin almacenamiento: solo se pierde el primer fotograma.
  }
}

/** Tema de la última sesión (antes de que respondan los ajustes de Rust). */
export function aplicarTemaRecordado(): void {
  let tema: string | null = null;
  try {
    tema = localStorage.getItem(CLAVE_TEMA);
  } catch {
    // Sin almacenamiento: «sistema».
  }
  if (tema === "claro" || tema === "oscuro") document.documentElement.dataset.tema = tema;
}

// ── Atlas ──

/** Espera (ms) entre consultas al atlas mientras se prepara, y cuántas veces. */
const ESPERA_ATLAS = 1000;
const INTENTOS_ATLAS = 20;
/** Cambia en cada consulta nueva: la anterior deja de actualizar el estado. */
let consulta = 0;
/** Guardados en cola: el siguiente parte de lo que dejó el anterior. */
let cola: Promise<void> = Promise.resolve();

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

  /** Vuelve a leer las versiones instaladas (y el estado del atlas). */
  const leerVersiones = async () => {
    set({ cargando: true });
    try {
      const [versiones, carpetaPorDefecto] = await Promise.all([
        versionesMinecraft(),
        carpetaMinecraftPorDefecto(),
      ]);
      set({ versiones, carpetaPorDefecto, cargando: false });
      if (get().abierto) void vigilarAtlas(versiones.length > 0);
    } catch (e) {
      set({ cargando: false, error: normalizarError(e).mensaje });
    }
  };

  return {
    abierto: false,
    cargando: false,
    guardando: false,
    error: null,
    versiones: [],
    carpetaPorDefecto: null,
    guardados: AJUSTES_POR_DEFECTO,
    atlas: { tipo: "comprobando" },

    iniciar: async () => {
      try {
        const guardados = { ...AJUSTES_POR_DEFECTO, ...(await leerAjustes()) };
        set({ guardados });
        aplicarTema(guardados.tema);
      } catch (e) {
        // Sin ajustes se sigue con los de por defecto (y el tema recordado).
        console.warn("No se pudieron leer los ajustes:", e);
      }
    },

    abrir: async () => {
      if (get().abierto) return;
      set({ abierto: true, error: null });
      await leerVersiones();
    },

    cerrar: () => {
      consulta++;
      set({ abierto: false });
    },

    cambiar: (cambios) => {
      const trabajo = cola.then(async () => {
        const anteriores = get().guardados;
        const nuevos: Ajustes = { ...anteriores, ...cambios };
        // `rutaMinecraft: undefined` vuelve a la carpeta habitual.
        if ("rutaMinecraft" in cambios && !cambios.rutaMinecraft) delete nuevos.rutaMinecraft;
        set({ guardados: nuevos, guardando: true, error: null });
        if (nuevos.tema !== anteriores.tema) aplicarTema(nuevos.tema);
        try {
          const cambianTexturas = await guardarAjustes(nuevos);
          set({ guardando: false });
          if (nuevos.rutaMinecraft !== anteriores.rutaMinecraft) await leerVersiones();
          if (cambianTexturas) {
            texturasCambiadas();
            void vigilarAtlas(get().versiones.length > 0);
          }
        } catch (e) {
          set({ guardados: anteriores, guardando: false, error: normalizarError(e).mensaje });
          if (nuevos.tema !== anteriores.tema) aplicarTema(anteriores.tema);
        }
      });
      cola = trabajo;
      return trabajo;
    },
  };
});
