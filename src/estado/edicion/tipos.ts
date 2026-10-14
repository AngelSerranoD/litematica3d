// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Tipos del almacén de edición: su estado entero y los trozos que lo forman.

import type { StateCreator } from "zustand";
import type { CeldaAtlas } from "../../api/atlas";
import type { Cambio, Reemplazo, Version } from "../../api/edicion";
import type { EstadoPresente, FamiliaParecida, Parecido, Rgb } from "../../api/parecidos";
import type { BloqueSeleccionado, Vec3 } from "../../api/visor";

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
type ObjetivoLista =
  /** El bloque pinchado: elegir crea un `CambioPendiente` en su posición. */
  | { tipo: "bloque"; pinchado: Pinchado }
  /** Una fila de la tabla origen → destino de una familia: elegir cambia su destino. */
  | {
      tipo: "reemplazo";
      /** Estado representante de la fila (el que se busca en la lista). */
      estadoOrigen: string;
      /** Otros estados del mismo bloque (otro `facing`…) que cambian con él. */
      hermanos?: string[];
    };

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
type Ocupado = "aplicar" | "restaurar" | "exportar" | null;

export interface EstadoEdicion {
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
  restaurar: (
    version: number,
    id?: string,
    /** Acción que se está haciendo (Deshacer/Rehacer): el aviso ofrece la contraria. */
    hecha?: "Deshacer" | "Rehacer",
  ) => Promise<void>;
  cargarHistorial: () => Promise<void>;
  /** Diálogo de guardado y exportación como .litematic de la copia actual. */
  exportar: (nombre: string) => Promise<void>;

  /** Pide las celdas del atlas de los estados que falten (iconos y previsualización). */
  pedirCeldas: (estados: string[]) => void;
  /** Pide el color plano de los estados que falten (previsualización sin atlas). */
  pedirColores: (estados: string[]) => void;
}

/**
 * Un trozo del almacén: una parte de `EstadoEdicion` (`T`) creada con el
 * `set`/`get` del almacén entero, para que cada trozo pueda leer y llamar a
 * los demás.
 */
export type Trozo<T extends Partial<EstadoEdicion>> = StateCreator<EstadoEdicion, [], [], T>;
