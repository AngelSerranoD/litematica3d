// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";
import * as api from "../api/biblioteca";
import type { Entrada, ErrorApp, ResultadoImportacion } from "../api/biblioteca";

/** Tiempo que dura un aviso en pantalla. */
export const DURACION_AVISO_MS = 5000;

export interface Aviso {
  id: number;
  mensaje: string;
  /** Marca de tiempo (ms) a partir de la cual el aviso ya no se enseña. */
  caduca: number;
}

interface EstadoBiblioteca {
  entradas: Entrada[];
  /** Verdadero hasta que termina la primera carga. */
  cargando: boolean;
  error: ErrorApp | null;
  avisos: Aviso[];
  cargar: () => Promise<void>;
  importar: (rutas: string[]) => Promise<ResultadoImportacion[]>;
  renombrar: (id: string, nombre: string) => Promise<Entrada>;
  eliminar: (id: string) => Promise<void>;
  avisar: (mensaje: string) => void;
  descartarAviso: (id: number) => void;
}

let siguienteAviso = 1;

/** «3 importados · 1 duplicado · 1 con error: <mensaje>» */
export function resumenImportacion(resultados: ResultadoImportacion[]): string {
  const nuevos = resultados.filter((r) => r.estado === "importado").length;
  const duplicados = resultados.filter((r) => r.estado === "duplicado").length;
  const errores = resultados.flatMap((r) => (r.estado === "error" ? [r.error.mensaje] : []));
  const partes: string[] = [];
  if (nuevos) partes.push(`${nuevos} ${nuevos === 1 ? "importado" : "importados"}`);
  if (duplicados) partes.push(`${duplicados} ${duplicados === 1 ? "duplicado" : "duplicados"}`);
  if (errores.length) {
    partes.push(
      `${errores.length} con error: ${errores[0]}${errores.length > 1 ? " (y otros)" : ""}`,
    );
  }
  return partes.join(" · ");
}

export const useBiblioteca = create<EstadoBiblioteca>()((set, get) => ({
  entradas: [],
  cargando: true,
  error: null,
  avisos: [],

  cargar: async () => {
    try {
      const entradas = await api.listar();
      set({ entradas, cargando: false, error: null });
    } catch (e) {
      set({ cargando: false, error: api.normalizarError(e) });
    }
  },

  importar: async (rutas) => {
    let resultados: ResultadoImportacion[];
    try {
      resultados = await api.importar(rutas);
    } catch (e) {
      get().avisar(api.normalizarError(e).mensaje);
      return [];
    }
    // Las nuevas van delante; una repetida no se vuelve a añadir.
    const nuevas = resultados.flatMap((r) => (r.estado === "importado" ? [r.entrada] : []));
    if (nuevas.length) {
      set((s) => {
        const ids = new Set(nuevas.map((n) => n.id));
        return { entradas: [...nuevas, ...s.entradas.filter((x) => !ids.has(x.id))] };
      });
    }
    const resumen = resumenImportacion(resultados);
    if (resumen) get().avisar(resumen);
    return resultados;
  },

  renombrar: async (id, nombre) => {
    const actualizada = await api.renombrar(id, nombre);
    set((s) => ({ entradas: s.entradas.map((x) => (x.id === id ? actualizada : x)) }));
    return actualizada;
  },

  eliminar: async (id) => {
    await api.eliminar(id);
    set((s) => ({ entradas: s.entradas.filter((x) => x.id !== id) }));
  },

  avisar: (mensaje) => {
    const aviso: Aviso = {
      id: siguienteAviso++,
      mensaje,
      caduca: Date.now() + DURACION_AVISO_MS,
    };
    set((s) => ({ avisos: [...s.avisos, aviso] }));
  },

  descartarAviso: (id) => set((s) => ({ avisos: s.avisos.filter((a) => a.id !== id) })),
}));
