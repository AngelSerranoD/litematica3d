// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useMemo } from "react";
import { create } from "zustand";
import * as api from "../api/biblioteca";
import { nombreFormato } from "../logica/formato";
import type { Entrada, ErrorApp, ResultadoImportacion } from "../api/biblioteca";
import { elegirArchivos } from "../api/dialogo";
import { normalizarError } from "../api/llamar";

/** Tiempo que dura un aviso en pantalla. */
const DURACION_AVISO_MS = 5000;

/** Tiempo que se puede deshacer un borrado antes de mandarlo a la Papelera. */
export const ESPERA_BORRADO_MS = 5000;

/** Botón de un aviso (p. ej. «Deshacer»): al pulsarlo se ejecuta y el aviso se cierra. */
interface AccionAviso {
  etiqueta: string;
  ejecutar: () => void;
}

export interface Aviso {
  id: number;
  mensaje: string;
  /** Marca de tiempo (ms) a partir de la cual el aviso ya no se enseña. */
  caduca: number;
  accion?: AccionAviso;
}

interface OpcionesAviso {
  accion?: AccionAviso;
  /** Cuánto dura en pantalla (por defecto `DURACION_AVISO_MS`). */
  duracionMs?: number;
}

export type CampoOrden = "fecha" | "nombre" | "tamano";
type DireccionOrden = "asc" | "desc";
type ModoVista = "rejilla" | "lista";

export interface Orden {
  campo: CampoOrden;
  direccion: DireccionOrden;
}

/** Una entrada ocultada que aún se puede recuperar. */
export interface PendienteDeBorrar {
  entrada: Entrada;
  /** Posición original en `entradas`, para devolverla a su sitio. */
  indice: number;
  /** Marca de tiempo (ms) en que se ejecuta el borrado. */
  vence: number;
}

const CLAVE_MODO_VISTA = "litematica3d.modoVista";

function leerModoVista(): ModoVista {
  try {
    const v = localStorage.getItem(CLAVE_MODO_VISTA);
    if (v === "rejilla" || v === "lista") return v;
  } catch {
    // Sin acceso a localStorage: se usa el valor por defecto.
  }
  return "rejilla";
}

function guardarModoVista(modo: ModoVista) {
  try {
    localStorage.setItem(CLAVE_MODO_VISTA, modo);
  } catch {
    // No es grave: solo se pierde la preferencia.
  }
}

/** Minúsculas y sin tildes, para buscar sin distinguir mayúsculas ni acentos. */
export function normalizarTexto(texto: string): string {
  return texto.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
}

/** Filtra por nombre, autor y formato, y ordena. No modifica `entradas`. */
function calcularVisibles(
  entradas: Entrada[],
  busqueda: string,
  orden: Orden,
): Entrada[] {
  const consulta = normalizarTexto(busqueda.trim());
  const filtradas = consulta
    ? entradas.filter((e) =>
        normalizarTexto(
          `${e.nombre} ${e.autor ?? ""} ${e.formato} ${nombreFormato(e.formato)}`,
        ).includes(consulta),
      )
    : entradas.slice();
  const signo = orden.direccion === "asc" ? 1 : -1;
  const comparar = (a: Entrada, b: Entrada): number => {
    switch (orden.campo) {
      case "nombre":
        return a.nombre.localeCompare(b.nombre, "es", { sensitivity: "base", numeric: true });
      case "tamano":
        return a.totalBloques - b.totalBloques;
      default:
        return a.fechaImportacion - b.fechaImportacion;
    }
  };
  return filtradas.sort((a, b) => signo * comparar(a, b));
}

interface EstadoBiblioteca {
  entradas: Entrada[];
  /** Verdadero hasta que termina la primera carga. */
  cargando: boolean;
  error: ErrorApp | null;
  avisos: Aviso[];
  busqueda: string;
  orden: Orden;
  modoVista: ModoVista;
  /** Ids recién importados en la última importación (para animar su «caída»). */
  ultimaImportada: string[];
  /** Borrados que se pueden deshacer, por id. */
  pendientesDeBorrar: Record<string, PendienteDeBorrar>;
  cargar: () => Promise<void>;
  importar: (rutas: string[]) => Promise<ResultadoImportacion[]>;
  /** Abre el diálogo del sistema y importa lo elegido. */
  elegirEImportar: () => Promise<ResultadoImportacion[]>;
  /** Optimista; si falla se revierte, se avisa y devuelve `null`. */
  renombrar: (id: string, nombre: string) => Promise<Entrada | null>;
  /** Oculta ya; a los 5 s va a la Papelera salvo que se deshaga. */
  eliminar: (id: string) => void;
  deshacerEliminar: (id: string) => void;
  /** Ejecuta ya los borrados pendientes (al cerrar la ventana). */
  vaciarPendientes: () => Promise<void>;
  establecerBusqueda: (texto: string) => void;
  establecerOrden: (orden: Orden) => void;
  /** Ordena por ese campo; si ya era el activo, invierte la dirección. */
  alternarOrden: (campo: CampoOrden) => void;
  establecerModoVista: (modo: ModoVista) => void;
  avisar: (mensaje: string, opciones?: OpcionesAviso) => void;
  /** Pone al día una entrada que cambió fuera (edición): la cola de miniaturas la revisa. */
  actualizarEntrada: (entrada: Entrada) => void;
  descartarAviso: (id: number) => void;
}

/** Temporizadores de los borrados diferidos (fuera del estado: no son serializables). */
const temporizadores = new Map<string, ReturnType<typeof setTimeout>>();

let siguienteAviso = 1;

/** «3 importados · 1 duplicado · 1 con error: <mensaje>» */
function resumenImportacion(resultados: ResultadoImportacion[]): string {
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
  busqueda: "",
  orden: { campo: "fecha", direccion: "desc" },
  modoVista: leerModoVista(),
  ultimaImportada: [],
  pendientesDeBorrar: {},

  cargar: async () => {
    try {
      const entradas = await api.listar();
      set({ entradas, cargando: false, error: null });
    } catch (e) {
      set({ cargando: false, error: normalizarError(e) });
    }
  },

  importar: async (rutas) => {
    let resultados: ResultadoImportacion[];
    try {
      resultados = await api.importar(rutas);
    } catch (e) {
      get().avisar(normalizarError(e).mensaje);
      return [];
    }
    // Las nuevas van delante; una repetida no se vuelve a añadir.
    const nuevas = resultados.flatMap((r) => (r.estado === "importado" ? [r.entrada] : []));
    if (nuevas.length) {
      set((s) => {
        const ids = new Set(nuevas.map((n) => n.id));
        return {
          entradas: [...nuevas, ...s.entradas.filter((x) => !ids.has(x.id))],
          ultimaImportada: nuevas.map((n) => n.id),
        };
      });
    }
    const resumen = resumenImportacion(resultados);
    if (resumen) get().avisar(resumen);
    return resultados;
  },

  elegirEImportar: async () => {
    let rutas: string[];
    try {
      rutas = await elegirArchivos();
    } catch (e) {
      get().avisar(`No se pudo abrir el selector de archivos: ${normalizarError(e).mensaje}`);
      return [];
    }
    if (!rutas.length) return [];
    return get().importar(rutas);
  },

  renombrar: async (id, nombre) => {
    const previa = get().entradas.find((x) => x.id === id);
    if (!previa) return null;
    set((s) => ({ entradas: s.entradas.map((x) => (x.id === id ? { ...x, nombre } : x)) }));
    try {
      const actualizada = await api.renombrar(id, nombre);
      set((s) => ({ entradas: s.entradas.map((x) => (x.id === id ? actualizada : x)) }));
      return actualizada;
    } catch (e) {
      // Se revierte solo el nombre, por si la entrada cambió mientras tanto.
      set((s) => ({
        entradas: s.entradas.map((x) => (x.id === id ? { ...x, nombre: previa.nombre } : x)),
      }));
      get().avisar(`No se pudo renombrar: ${normalizarError(e).mensaje}`);
      return null;
    }
  },

  eliminar: (id) => {
    const { entradas, pendientesDeBorrar } = get();
    const indice = entradas.findIndex((x) => x.id === id);
    if (indice < 0 || pendientesDeBorrar[id]) return;
    const entrada = entradas[indice];
    set((s) => ({
      entradas: s.entradas.filter((x) => x.id !== id),
      pendientesDeBorrar: {
        ...s.pendientesDeBorrar,
        [id]: { entrada, indice, vence: Date.now() + ESPERA_BORRADO_MS },
      },
    }));
    temporizadores.set(
      id,
      setTimeout(() => void ejecutarBorrado(id), ESPERA_BORRADO_MS),
    );
  },

  deshacerEliminar: (id) => {
    const pendiente = get().pendientesDeBorrar[id];
    if (!pendiente) return;
    cancelarTemporizador(id);
    restaurar(pendiente);
  },

  vaciarPendientes: async () => {
    const ids = Object.keys(get().pendientesDeBorrar);
    await Promise.all(ids.map((id) => ejecutarBorrado(id)));
  },

  establecerBusqueda: (texto) => set({ busqueda: texto }),

  establecerOrden: (orden) => set({ orden }),

  alternarOrden: (campo) =>
    set((s) => ({
      orden:
        s.orden.campo === campo
          ? { campo, direccion: s.orden.direccion === "asc" ? "desc" : "asc" }
          : // Fecha y tamaño arrancan de más a menos; el nombre, de la A a la Z.
            { campo, direccion: campo === "nombre" ? "asc" : "desc" },
    })),

  establecerModoVista: (modo) => {
    guardarModoVista(modo);
    set({ modoVista: modo });
  },

  avisar: (mensaje, opciones) => {
    const aviso: Aviso = {
      id: siguienteAviso++,
      mensaje,
      caduca: Date.now() + (opciones?.duracionMs ?? DURACION_AVISO_MS),
      accion: opciones?.accion,
    };
    set((s) => ({ avisos: [...s.avisos, aviso] }));
  },

  actualizarEntrada: (entrada) =>
    set((s) => ({ entradas: s.entradas.map((x) => (x.id === entrada.id ? entrada : x)) })),

  descartarAviso: (id) => set((s) => ({ avisos: s.avisos.filter((a) => a.id !== id) })),
}));

function cancelarTemporizador(id: string) {
  const t = temporizadores.get(id);
  if (t !== undefined) clearTimeout(t);
  temporizadores.delete(id);
}

/** Devuelve una entrada ocultada a su posición original (o al final si la lista es más corta). */
function restaurar(pendiente: PendienteDeBorrar) {
  const { entrada, indice } = pendiente;
  useBiblioteca.setState((s) => {
    const resto = { ...s.pendientesDeBorrar };
    delete resto[entrada.id];
    const entradas = s.entradas.slice();
    entradas.splice(Math.min(indice, entradas.length), 0, entrada);
    return { entradas, pendientesDeBorrar: resto };
  });
}

/** Manda la entrada a la Papelera; si falla, la restaura y avisa. */
async function ejecutarBorrado(id: string) {
  const pendiente = useBiblioteca.getState().pendientesDeBorrar[id];
  if (!pendiente) return;
  cancelarTemporizador(id);
  try {
    await api.eliminar(id);
    useBiblioteca.setState((s) => {
      const resto = { ...s.pendientesDeBorrar };
      delete resto[id];
      return { pendientesDeBorrar: resto };
    });
  } catch (e) {
    restaurar(pendiente);
    useBiblioteca
      .getState()
      .avisar(
        `No se pudo eliminar «${pendiente.entrada.nombre}»: ${normalizarError(e).mensaje}`,
      );
  }
}

/** Entradas filtradas y ordenadas; estable entre renders mientras no cambie el estado. */
export function useEntradasVisibles(): Entrada[] {
  const entradas = useBiblioteca((s) => s.entradas);
  const busqueda = useBiblioteca((s) => s.busqueda);
  const orden = useBiblioteca((s) => s.orden);
  return useMemo(() => calcularVisibles(entradas, busqueda, orden), [entradas, busqueda, orden]);
}

