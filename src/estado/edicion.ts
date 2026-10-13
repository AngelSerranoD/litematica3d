// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";
import { celdasAtlas, coloresBloques, MAX_CELDAS_ATLAS, type CeldaAtlas } from "../api/atlas";
import { normalizarError } from "../api/biblioteca";
import { elegirDestinoLitematic } from "../api/dialogo";
import {
  aplicarCambios,
  exportarSchematic,
  historial,
  restaurarVersion,
  type Cambio,
  type Edicion,
  type Reemplazo,
  type Version,
} from "../api/edicion";
import {
  familiaDe,
  familiasParecidas,
  miembrosFamilia,
  parecidos as pedirParecidos,
  planFamilia,
  type EstadoPresente,
  type FamiliaParecida,
  type Parecido,
  type Rgb,
} from "../api/parecidos";
import type { BloqueSeleccionado, Vec3 } from "../api/visor";
import { useBiblioteca } from "./biblioteca";
import { alCambiarTexturas } from "./texturas";
import { useVisor } from "./visor";

/**
 * Edición de bloques en el visor (docs/edicion.md, tarea 10).
 *
 * El visor tiene dos modos: **Visualizar** (lo de siempre) y **Modificar**.
 * En Modificar, pinchar un bloque abre la lista de los más parecidos; lo que
 * se elige queda **pendiente** (cambios sueltos por posición y reemplazos de
 * un estado por otro en todo el schematic) hasta pulsar **Aplicar**, que lo
 * escribe en la copia de la biblioteca y deja lo anterior en el historial.
 *
 * Como en `visor.ts`, ningún selector debe construir arrays u objetos nuevos:
 * las listas solo cambian de referencia cuando cambia su contenido.
 */

export type ModoVisor = "visualizar" | "modificar";

/** Un cambio suelto pendiente: lo que pide `aplicarCambios` y algo más para enseñarlo. */
export interface CambioPendiente extends Cambio {
  /** `region|x,y,z`: una sola entrada por posición (gana la última elección). */
  clave: string;
  /** Celda en coordenadas del modelo (resaltado y previsualización en el visor). */
  celda: Vec3;
  /** Block state que hay ahora en esa posición. */
  estadoAnterior: string;
}

/** Un reemplazo pendiente (todas las apariciones de un estado exacto). */
export interface ReemplazoPendiente extends Reemplazo {
  /** Posiciones que ocupa el origen en el schematic (`miembrosFamilia`), o `null` si no se sabe. */
  cantidad: number | null;
  /** Familia de la que salió (selección por familia), o `null` si se añadió suelto. */
  familia: string | null;
  /** Familia destino del plan del que salió (para recordarla al volver a la familia). */
  familiaDestino?: string;
}

/** Bloque pinchado en Modificar. */
export interface Pinchado {
  /** Celda en coordenadas del modelo. */
  celda: Vec3;
  bloque: BloqueSeleccionado;
}

/** A qué se aplica lo que se elija en la lista de parecidos. */
export type ObjetivoLista =
  /** El bloque pinchado: elegir crea un `CambioPendiente` en su posición. */
  | { tipo: "bloque"; pinchado: Pinchado }
  /** Una fila de la tabla origen → destino de una familia: elegir cambia su destino. */
  | { tipo: "reemplazo"; estadoOrigen: string };

export interface ListaParecidos {
  objetivo: ObjetivoLista;
  /** Block state del que se buscan parecidos. */
  origen: string;
  estado: "cargando" | "lista" | "error";
  parecidos: Parecido[];
  /** Por qué no hay lista (error o bloque que no se puede cambiar). */
  mensaje: string | null;
}

/**
 * Selección por familia («Seleccionar familia» o Mayús+clic): familia del
 * bloque pinchado, sus estados en el schematic y las familias destino.
 */
export interface SeleccionFamilia {
  /** Familia de material; `""` mientras se pregunta (`familiaDe`). */
  familia: string;
  /** Block state desde el que se seleccionó (el bloque pinchado). */
  origen: string;
  estado: "cargando" | "lista" | "error";
  /** Block states de la familia presentes en el schematic (`miembrosFamilia`). */
  miembros: EstadoPresente[];
  /** Familias destino (`familiasParecidas`), o `null` mientras se piden. */
  destinos: FamiliaParecida[] | null;
  /** Familia destino elegida (su plan está en `reemplazos`, con `familia` = la origen). */
  destino: string | null;
  /** Familia destino cuyo plan se está pidiendo (`planFamilia`). */
  pidiendoPlan: string | null;
  mensaje: string | null;
}

/** Pregunta abierta en el diálogo de confirmación de la edición. */
export type Confirmacion =
  | { tipo: "aplicar" }
  | {
      tipo: "descartar";
      /** Qué se iba a hacer: cambia el texto de la pregunta. */
      motivo: MotivoDescarte;
      /** Lo que se hace tras descartar. */
      continuar: () => void;
    };

export type MotivoDescarte = "salirModo" | "salirSchematic" | "cerrarVentana" | "restaurar" | "todo";

/** Qué enseña la parte de arriba del panel de edición. */
export type PanelEdicion = "cambiar" | "historial";

/** Operación larga en curso (desactiva los botones que chocarían). */
export type Ocupado = "aplicar" | "restaurar" | "exportar" | null;

interface EstadoEdicion {
  /** Schematic abierto (`null` en la biblioteca). */
  id: string | null;
  modo: ModoVisor;
  panel: PanelEdicion;
  pinchado: Pinchado | null;
  lista: ListaParecidos | null;
  cambios: CambioPendiente[];
  reemplazos: ReemplazoPendiente[];
  familia: SeleccionFamilia | null;
  ocupado: Ocupado;
  /** Último error de aplicar/restaurar/exportar, para enseñarlo en el panel. */
  error: string | null;
  /** Historial del schematic, o `null` si no se ha pedido aún. */
  versiones: Version[] | null;
  cargandoHistorial: boolean;
  errorHistorial: string | null;
  confirmacion: Confirmacion | null;
  /**
   * Sube tras cada aplicar o restaurar del schematic abierto: el visor vuelve
   * a pedir la malla y la lista de materiales se recalcula.
   */
  revision: number;
  /** Celdas del atlas por block state (7 caras, `CARAS_ATLAS`); `[]` = sin textura. */
  celdas: Record<string, (CeldaAtlas | null)[]>;
  /** Versión del atlas con que se resolvieron `celdas`. */
  versionCeldas: string | null;
  /** Color plano por block state (previsualización sin atlas); `null` = estado no válido. */
  colores: Record<string, Rgb | null>;

  /** Al abrir un schematic: si es otro, todo vuelve a empezar (en Visualizar). */
  iniciar: (id: string | null) => void;
  /** Cambia de modo; salir de Modificar con pendientes pregunta antes. */
  fijarModo: (modo: ModoVisor) => void;
  alternarModo: () => void;
  verPanel: (panel: PanelEdicion) => void;

  /** Bloque pinchado en Modificar (lo llama la suscripción a `useVisor`). */
  pinchar: (celda: Vec3, bloque: BloqueSeleccionado) => void;
  /** Abre la lista de parecidos para el bloque pinchado o para una fila de reemplazo. */
  abrirLista: (objetivo: ObjetivoLista) => void;
  /** Cierra la lista; si era la del bloque pinchado, también la selección del visor. */
  cerrarLista: () => void;
  /** Elige un parecido de la lista abierta (lo deja pendiente). */
  elegirParecido: (parecido: Parecido) => void;

  /** Añade (o sustituye, por posición) un cambio suelto. Mismo estado = se quita. */
  agregarCambio: (c: Omit<CambioPendiente, "clave">) => void;
  quitarCambio: (clave: string) => void;
  /**
   * Añade reemplazos (p. ej. el plan de una familia). Un origen que ya
   * estaba se sustituye; origen = destino se quita.
   */
  agregarReemplazos: (r: ReemplazoPendiente[]) => void;
  /** Cambia el destino de un reemplazo de la tabla origen → destino. */
  fijarDestinoReemplazo: (estadoOrigen: string, estadoDestino: string) => void;
  quitarReemplazo: (estadoOrigen: string) => void;
  /** Quita los reemplazos que vinieron de una familia (al cambiar de destino o cerrarla). */
  quitarReemplazosDeFamilia: (familia: string) => void;
  fijarFamilia: (familia: SeleccionFamilia | null) => void;
  /**
   * Selecciona la familia de un block state: la resalta en el visor y pide
   * sus estados en el schematic y las familias destino.
   */
  seleccionarFamilia: (estado: string) => void;
  /**
   * Elige la familia destino: pide su plan y lo deja pendiente (sustituye el
   * de otro destino). Elegir la que ya estaba la quita.
   */
  elegirFamiliaDestino: (familia: string) => void;
  /** Cierra la selección por familia (sus reemplazos siguen pendientes). */
  cerrarFamilia: () => void;
  /** Descarta todos los pendientes sin preguntar. */
  descartarTodo: () => void;

  /**
   * Si hay pendientes, abre la pregunta de descartarlos y devuelve `false`
   * (`continuar` se llama si se descartan). Sin pendientes, `true`.
   */
  pedirDescartar: (motivo: MotivoDescarte, continuar: () => void) => boolean;
  /** Abre la confirmación de Aplicar (con el número de bloques afectados). */
  pedirAplicar: () => void;
  /** Acepta la pregunta abierta. */
  confirmar: () => void;
  cancelarConfirmacion: () => void;

  /** Escribe los pendientes en la copia de la biblioteca. */
  aplicar: () => Promise<void>;
  /** Vuelve a una versión del historial (deshacer = la que devolvió aplicar). */
  restaurar: (version: number, id?: string) => Promise<void>;
  cargarHistorial: () => Promise<void>;
  /** Diálogo de guardado y exportación como .litematic de la copia actual. */
  exportar: (nombre: string) => Promise<void>;

  /** Pide las celdas del atlas de los estados que falten (iconos y previsualización). */
  pedirCeldas: (estados: string[]) => void;
  /** Pide el color plano de los estados que falten (previsualización sin atlas). */
  pedirColores: (estados: string[]) => void;
}

/** Bloques parecidos que se piden por bloque. */
export const PARECIDOS_POR_BLOQUE = 16;
/** Cuánto se ve el aviso con Deshacer tras aplicar o restaurar. */
const DURACION_AVISO_EDICION_MS = 10000;

const claveCambio = (region: string, x: number, y: number, z: number) => `${region}|${x},${y},${z}`;

/** Sube con cada lista abierta: las respuestas de antes ya no valen. */
let peticionLista = 0;
/** Sube con cada historial pedido o schematic nuevo. */
let peticionHistorial = 0;
/** Sube con cada familia seleccionada o cerrada. */
let peticionFamilia = 0;
/** Sube con cada familia destino elegida. */
let peticionPlan = 0;
/** Familias destino que se piden. */
const FAMILIAS_DESTINO = 12;
/** Colores planos en vuelo. */
const pidiendoColores = new Set<string>();
/** Estados cuyas celdas están en vuelo. */
const pidiendoCeldas = new Set<string>();
/** Estados por pedir en la próxima tanda (se juntan las peticiones de un mismo pintado). */
const colaCeldas = new Set<string>();
let tandaProgramada = false;

const mensajeDe = (e: unknown) => normalizarError(e).mensaje;

/** Bloques que tocarían los pendientes (los reemplazos sin cantidad no suman). */
export function bloquesAfectados(cambios: CambioPendiente[], reemplazos: ReemplazoPendiente[]) {
  let total = cambios.length;
  let exacto = true;
  for (const r of reemplazos) {
    if (r.cantidad === null) exacto = false;
    else total += r.cantidad;
  }
  return { total, exacto };
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

export const useEdicion = create<EstadoEdicion>()((set, get) => {
  /** Lo que hay que hacer tras aplicar o restaurar con éxito. */
  const despues = (ed: Edicion) => {
    useBiblioteca.getState().actualizarEntrada(ed.entrada);
    if (ed.entrada.id !== get().id) return; // era de otro schematic (aviso tardío)
    peticionLista++;
    olvidarFamilia();
    // `version: null` = no cambió nada en disco: no hace falta remallar.
    const cambio = ed.version !== null;
    set((s) => ({
      revision: cambio ? s.revision + 1 : s.revision,
      pinchado: null,
      lista: null,
      cambios: [],
      reemplazos: [],
      familia: null,
      error: null,
    }));
    // La celda seleccionada puede tener ya otro bloque.
    useVisor.getState().limpiarSeleccion();
    if (cambio && get().versiones !== null) void get().cargarHistorial();
  };

  const hayPendientes = () => get().cambios.length > 0 || get().reemplazos.length > 0;

  /** Las respuestas de familia y plan en vuelo ya no valen. */
  function olvidarFamilia() {
    peticionFamilia++;
    peticionPlan++;
  }

  /** Cambia la selección por familia si sigue siendo la misma `familia`. */
  const fijarSiSigue = (familia: string, cambio: Partial<SeleccionFamilia>) =>
    set((s) =>
      s.familia && s.familia.familia === familia ? { familia: { ...s.familia, ...cambio } } : {},
    );

  return {
    id: null,
    modo: "visualizar",
    panel: "cambiar",
    pinchado: null,
    lista: null,
    cambios: [],
    reemplazos: [],
    familia: null,
    ocupado: null,
    error: null,
    versiones: null,
    cargandoHistorial: false,
    errorHistorial: null,
    confirmacion: null,
    revision: 0,
    celdas: {},
    versionCeldas: null,
    colores: {},

    iniciar: (id) => {
      if (id === get().id) return;
      peticionLista++;
      peticionHistorial++;
      olvidarFamilia();
      set({
        id,
        modo: "visualizar",
        panel: "cambiar",
        pinchado: null,
        lista: null,
        cambios: [],
        reemplazos: [],
        familia: null,
        ocupado: null,
        error: null,
        versiones: null,
        cargandoHistorial: false,
        errorHistorial: null,
        confirmacion: null,
      });
    },

    fijarModo: (modo) => {
      const s = get();
      if (modo === s.modo) return;
      if (modo === "modificar") {
        set({ modo, panel: "cambiar", error: null });
        // Lo que ya estuviera seleccionado pasa a ser el bloque pinchado.
        const sel = useVisor.getState().seleccion;
        if (sel?.bloque) get().pinchar(sel.celda, sel.bloque);
        return;
      }
      const salir = () => {
        peticionLista++;
        olvidarFamilia();
        set({
          modo: "visualizar",
          pinchado: null,
          lista: null,
          familia: null,
          confirmacion: null,
        });
      };
      if (get().pedirDescartar("salirModo", salir)) salir();
    },

    alternarModo: () => get().fijarModo(get().modo === "visualizar" ? "modificar" : "visualizar"),

    // El historial lo pide su sección al montarse.
    verPanel: (panel) => set({ panel }),

    pinchar: (celda, bloque) => {
      const pinchado: Pinchado = { celda, bloque };
      // Pinchar otro bloque cierra la familia abierta (sus reemplazos siguen
      // pendientes); Mayús+clic vuelve a seleccionar la del bloque nuevo.
      if (get().familia) olvidarFamilia();
      set({ pinchado, panel: "cambiar", familia: null });
      get().abrirLista({ tipo: "bloque", pinchado });
    },

    abrirLista: (objetivo) => {
      const n = ++peticionLista;
      const origen =
        objetivo.tipo === "bloque" ? objetivo.pinchado.bloque.estado : objetivo.estadoOrigen;
      if (objetivo.tipo === "bloque" && objetivo.pinchado.bloque.region === null) {
        set({
          lista: {
            objetivo,
            origen,
            estado: "error",
            parecidos: [],
            mensaje: "Este bloque no está dentro de ninguna región: no se puede cambiar.",
          },
        });
        return;
      }
      set({
        lista: {
          objetivo,
          origen,
          estado: "cargando",
          parecidos: [],
          mensaje: null,
        },
      });
      get().pedirCeldas([origen]);
      // El primer uso construye el catálogo (~0,25 s con jar): se ve «cargando».
      pedirParecidos(origen, PARECIDOS_POR_BLOQUE)
        .then((parecidos) => {
          if (n !== peticionLista) return;
          set({
            lista: {
              objetivo,
              origen,
              estado: "lista",
              parecidos,
              mensaje: null,
            },
          });
          get().pedirCeldas(parecidos.map((p) => p.estado));
        })
        .catch((e) => {
          if (n !== peticionLista) return;
          set({
            lista: {
              objetivo,
              origen,
              estado: "error",
              parecidos: [],
              mensaje: mensajeDe(e),
            },
          });
        });
    },

    cerrarLista: () => {
      const lista = get().lista;
      peticionLista++;
      if (!lista) return;
      if (lista.objetivo.tipo === "bloque") {
        set({ lista: null, pinchado: null });
        useVisor.getState().limpiarSeleccion();
      } else {
        set({ lista: null });
      }
    },

    elegirParecido: (p) => {
      const lista = get().lista;
      if (!lista) return;
      if (lista.objetivo.tipo === "reemplazo") {
        get().fijarDestinoReemplazo(lista.objetivo.estadoOrigen, p.estado);
        return;
      }
      const { celda, bloque } = lista.objetivo.pinchado;
      if (bloque.region === null) return;
      const [x, y, z] = bloque.posicion;
      get().agregarCambio({
        region: bloque.region,
        x,
        y,
        z,
        nuevoEstado: p.estado,
        celda,
        estadoAnterior: bloque.estado,
      });
    },

    agregarCambio: (c) => {
      const clave = claveCambio(c.region, c.x, c.y, c.z);
      set((s) => {
        const resto = s.cambios.filter((x) => x.clave !== clave);
        return {
          cambios: c.nuevoEstado === c.estadoAnterior ? resto : [...resto, { ...c, clave }],
          error: null,
        };
      });
    },

    quitarCambio: (clave) => set((s) => ({ cambios: s.cambios.filter((x) => x.clave !== clave) })),

    agregarReemplazos: (nuevos) => {
      if (!nuevos.length) return;
      set((s) => {
        const origenes = new Set(nuevos.map((r) => r.estadoOrigen));
        const resto = s.reemplazos.filter((r) => !origenes.has(r.estadoOrigen));
        return {
          reemplazos: [...resto, ...nuevos.filter((r) => r.estadoOrigen !== r.estadoDestino)],
          error: null,
        };
      });
    },

    fijarDestinoReemplazo: (estadoOrigen, estadoDestino) =>
      set((s) => {
        const actual = s.reemplazos.find((r) => r.estadoOrigen === estadoOrigen);
        const resto = s.reemplazos.filter((r) => r.estadoOrigen !== estadoOrigen);
        if (estadoOrigen === estadoDestino) return { reemplazos: resto };
        const nuevo: ReemplazoPendiente = actual
          ? { ...actual, estadoDestino }
          : nuevoReemplazo(s.familia, estadoOrigen, estadoDestino);
        // Mismo sitio en la lista: la tabla no salta al cambiar un destino.
        return {
          reemplazos: actual
            ? s.reemplazos.map((r) => (r.estadoOrigen === estadoOrigen ? nuevo : r))
            : [...resto, nuevo],
        };
      }),

    quitarReemplazo: (estadoOrigen) =>
      set((s) => ({
        reemplazos: s.reemplazos.filter((r) => r.estadoOrigen !== estadoOrigen),
      })),

    quitarReemplazosDeFamilia: (familia) =>
      set((s) => ({
        reemplazos: s.reemplazos.filter((r) => r.familia !== familia),
      })),

    fijarFamilia: (familia) => {
      olvidarFamilia();
      set({ familia });
    },

    seleccionarFamilia: (estado) => {
      const id = get().id;
      if (!id) return;
      olvidarFamilia();
      const n = peticionFamilia;
      // Una lista de reemplazo abierta era de la familia anterior.
      if (get().lista?.objetivo.tipo === "reemplazo") {
        peticionLista++;
        set({ lista: null });
      }
      const base: SeleccionFamilia = {
        familia: "",
        origen: estado,
        estado: "cargando",
        miembros: [],
        destinos: null,
        destino: null,
        pidiendoPlan: null,
        mensaje: null,
      };
      set({ familia: base, panel: "cambiar" });
      const fallo = (mensaje: string) => {
        if (n === peticionFamilia)
          set((s) => (s.familia ? { familia: { ...s.familia, estado: "error", mensaje } } : {}));
      };
      (async () => {
        const fam = await familiaDe(estado);
        if (n !== peticionFamilia) return;
        if (!fam.sugerible) {
          fallo(
            "Este bloque no se puede cambiar en grupo: es aire, un fluido, un bloque técnico o " +
              "un bloque con datos (cofres, hornos, carteles…), y cambiarlo por familia podría " +
              "perder su contenido. Si hace falta, cámbialo suelto con su lista de parecidos.",
          );
          return;
        }
        set((s) => (s.familia ? { familia: { ...s.familia, familia: fam.familia } } : {}));
        const [miembros, destinos] = await Promise.all([
          miembrosFamilia(id, fam.familia),
          familiasParecidas(fam.familia, FAMILIAS_DESTINO),
        ]);
        if (n !== peticionFamilia) return;
        // Si ya había un plan de esta familia pendiente, se recuerda su destino.
        const destino =
          get().reemplazos.find((r) => r.familia === fam.familia && r.familiaDestino)
            ?.familiaDestino ?? null;
        set((s) =>
          s.familia
            ? {
                familia: {
                  ...s.familia,
                  estado: "lista",
                  miembros,
                  destinos,
                  destino,
                  mensaje: destinos.length
                    ? null
                    : "No hay otra familia de material con las mismas formas por la que cambiarla.",
                },
              }
            : {},
        );
        get().pedirCeldas(miembros.map((m) => m.estado));
      })().catch((e) => fallo(mensajeDe(e)));
    },

    elegirFamiliaDestino: (destino) => {
      const { id, familia: f } = get();
      if (!id || !f || f.estado !== "lista") return;
      const origen = f.familia;
      // La que ya estaba elegida: se quita su plan.
      if (f.destino === destino && f.pidiendoPlan === null) {
        peticionPlan++;
        get().quitarReemplazosDeFamilia(origen);
        fijarSiSigue(origen, { destino: null, mensaje: null });
        return;
      }
      const n = ++peticionPlan;
      fijarSiSigue(origen, { pidiendoPlan: destino, mensaje: null });
      planFamilia(id, origen, destino)
        .then((plan) => {
          if (n !== peticionPlan || get().familia?.familia !== origen) return;
          const cantidades = new Map(f.miembros.map((m) => [m.estado, m.cantidad]));
          get().quitarReemplazosDeFamilia(origen);
          get().agregarReemplazos(
            plan.map((r) => ({
              ...r,
              cantidad: cantidades.get(r.estadoOrigen) ?? null,
              familia: origen,
              familiaDestino: destino,
            })),
          );
          fijarSiSigue(origen, {
            destino: plan.length ? destino : null,
            pidiendoPlan: null,
            mensaje: plan.length
              ? null
              : "Con esa familia no cambiaría ningún bloque: no queda nada pendiente.",
          });
          get().pedirCeldas(plan.map((r) => r.estadoDestino));
        })
        .catch((e) => {
          if (n !== peticionPlan) return;
          fijarSiSigue(origen, {
            pidiendoPlan: null,
            mensaje: `No se pudo preparar el cambio: ${mensajeDe(e)}`,
          });
        });
    },

    cerrarFamilia: () => {
      if (!get().familia) return;
      olvidarFamilia();
      const { lista, pinchado } = get();
      set({ familia: null });
      // Se vuelve a la lista del bloque pinchado (una de reemplazo era de la familia).
      if (lista?.objetivo.tipo === "bloque") return;
      if (pinchado) get().abrirLista({ tipo: "bloque", pinchado });
      else if (lista) {
        peticionLista++;
        set({ lista: null });
      }
    },

    descartarTodo: () => {
      olvidarFamilia();
      set({ cambios: [], reemplazos: [], familia: null, error: null });
    },

    pedirDescartar: (motivo, continuar) => {
      if (!hayPendientes()) return true;
      set({ confirmacion: { tipo: "descartar", motivo, continuar } });
      return false;
    },

    pedirAplicar: () => {
      if (!hayPendientes() || get().ocupado) return;
      set({ confirmacion: { tipo: "aplicar" } });
    },

    confirmar: () => {
      const c = get().confirmacion;
      set({ confirmacion: null });
      if (!c) return;
      if (c.tipo === "aplicar") {
        void get().aplicar();
      } else {
        get().descartarTodo();
        c.continuar();
      }
    },

    cancelarConfirmacion: () => set({ confirmacion: null }),

    aplicar: async () => {
      const { id, cambios, reemplazos, ocupado } = get();
      if (!id || ocupado || (!cambios.length && !reemplazos.length)) return;
      set({ ocupado: "aplicar", error: null });
      try {
        const ed = await aplicarCambios(
          id,
          cambios.map(({ region, x, y, z, nuevoEstado }) => ({
            region,
            x,
            y,
            z,
            nuevoEstado,
          })),
          reemplazos.map(({ estadoOrigen, estadoDestino }) => ({
            estadoOrigen,
            estadoDestino,
          })),
        );
        despues(ed);
        const avisar = useBiblioteca.getState().avisar;
        const version = ed.version;
        if (version === null) {
          avisar("No había nada que cambiar: el schematic ya tenía esos bloques.");
        } else {
          avisar(ed.cambios === 1 ? "Cambiado 1 bloque." : `Cambiados ${ed.cambios} bloques.`, {
            accion: {
              etiqueta: "Deshacer",
              ejecutar: () => void get().restaurar(version, id),
            },
            duracionMs: DURACION_AVISO_EDICION_MS,
          });
        }
      } catch (e) {
        if (get().id === id) set({ error: `No se pudo aplicar: ${mensajeDe(e)}` });
        else useBiblioteca.getState().avisar(`No se pudo aplicar: ${mensajeDe(e)}`);
      } finally {
        if (get().id === id) set({ ocupado: null });
      }
    },

    restaurar: async (version, idPedido) => {
      const id = idPedido ?? get().id;
      if (!id) return;
      const actual = get().id === id;
      if (actual && get().ocupado) return;
      // Los pendientes son de la versión de ahora: restaurar los invalida.
      if (actual && !get().pedirDescartar("restaurar", () => void get().restaurar(version, id)))
        return;
      if (actual) set({ ocupado: "restaurar", error: null });
      try {
        const ed = await restaurarVersion(id, version);
        despues(ed);
        const rehacer = ed.version;
        useBiblioteca.getState().avisar(
          "Versión restaurada.",
          rehacer === null
            ? undefined
            : {
                accion: {
                  etiqueta: "Rehacer",
                  ejecutar: () => void get().restaurar(rehacer, id),
                },
                duracionMs: DURACION_AVISO_EDICION_MS,
              },
        );
      } catch (e) {
        const mensaje = `No se pudo restaurar: ${mensajeDe(e)}`;
        if (get().id === id) set({ error: mensaje });
        else useBiblioteca.getState().avisar(mensaje);
      } finally {
        if (get().id === id && get().ocupado === "restaurar") set({ ocupado: null });
      }
    },

    cargarHistorial: async () => {
      const id = get().id;
      if (!id) return;
      const n = ++peticionHistorial;
      set({ cargandoHistorial: true, errorHistorial: null });
      try {
        const versiones = await historial(id);
        if (n === peticionHistorial) set({ versiones, cargandoHistorial: false });
      } catch (e) {
        if (n === peticionHistorial)
          set({ cargandoHistorial: false, errorHistorial: mensajeDe(e) });
      }
    },

    exportar: async (nombre) => {
      const id = get().id;
      if (!id || get().ocupado) return;
      set({ ocupado: "exportar", error: null });
      try {
        const ruta = await elegirDestinoLitematic(nombre);
        if (!ruta) return;
        await exportarSchematic(id, ruta);
        useBiblioteca.getState().avisar(`Exportado a ${ruta}`);
      } catch (e) {
        const mensaje = `No se pudo exportar: ${mensajeDe(e)}`;
        if (get().id === id) set({ error: mensaje });
        else useBiblioteca.getState().avisar(mensaje);
      } finally {
        if (get().id === id && get().ocupado === "exportar") set({ ocupado: null });
      }
    },

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
  };
});

// En Modificar, cada bloque que se pincha en el visor abre su lista de
// parecidos; al quitar la selección (clic en el vacío, Esc) se cierra.
useVisor.subscribe((s, antes) => {
  if (s.seleccion === antes.seleccion) return;
  const e = useEdicion.getState();
  if (e.modo !== "modificar") return;
  const sel = s.seleccion;
  if (!sel) {
    if (e.lista?.objetivo.tipo === "bloque" || e.pinchado) {
      peticionLista++;
      useEdicion.setState({
        pinchado: null,
        lista: e.lista?.objetivo.tipo === "bloque" ? null : e.lista,
      });
    }
    return;
  }
  // `bloque` llega después (respuesta de `bloque_en`): hasta entonces nada.
  if (sel.bloque && sel.bloque !== antes.seleccion?.bloque) e.pinchar(sel.celda, sel.bloque);
});

// Con otras texturas las celdas de antes no valen: se vuelven a pedir.
alCambiarTexturas(() => {
  colaCeldas.clear();
  useEdicion.setState({ celdas: {}, versionCeldas: null });
});

/**
 * Reemplazo nuevo de la tabla origen → destino: si el origen es de la familia
 * seleccionada, entra en ella (con su cantidad); si no, suelto.
 */
function nuevoReemplazo(
  familia: SeleccionFamilia | null,
  estadoOrigen: string,
  estadoDestino: string,
): ReemplazoPendiente {
  const miembro = familia?.miembros.find((m) => m.estado === estadoOrigen);
  return miembro && familia
    ? { estadoOrigen, estadoDestino, cantidad: miembro.cantidad, familia: familia.familia }
    : { estadoOrigen, estadoDestino, cantidad: null, familia: null };
}

/** ¿Hay cambios o reemplazos sin aplicar? (selector atómico) */
export const hayPendientes = (s: EstadoEdicion) => s.cambios.length > 0 || s.reemplazos.length > 0;

/** Número de pendientes, cambios más reemplazos (selector atómico). */
export const numPendientes = (s: EstadoEdicion) => s.cambios.length + s.reemplazos.length;
