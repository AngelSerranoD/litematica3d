// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import type { TargetAndTransition, Transition, Variants } from "motion/react";
import { muelle } from "../estilos/movimiento";

/**
 * Cómo entra en escena una lámina (rejilla) o una fila (lista). Se calcula
 * una vez por montaje y no cambia después, para no romper el memo.
 */
export interface InfoEntrada {
  /** Retraso de la entrada escalonada, en segundos. */
  retraso: number;
  /** Recién importado: cae en su sitio con un muelle corto. */
  caer: boolean;
  /** El schematic del que se vuelve: ya está en su sitio (llega volando). */
  fija: boolean;
}

/** Acciones comunes de lámina y fila. El objeto es estable entre renders. */
export interface AccionesElemento {
  abrir: (id: string) => void;
  /** El elemento ha recibido el foco: pasa a ser el de la tabulación. */
  activar: (id: string) => void;
  empezarRenombrado: (id: string) => void;
  terminarRenombrado: (id: string, nombre: string | null, devolverFoco: boolean) => void;
  eliminar: (id: string) => void;
}

/** Salida al filtrar o eliminar: se desvanece con el muelle de pulsación (se lee al usarla, no al importar). */
export const salidaElemento = (): TargetAndTransition => ({
  opacity: 0,
  transition: muelle("pulsacion"),
});

/** Layout (reordenar, filtrar): firme, sin rebote. */
export const transicionElemento = (): Transition => ({ layout: muelle("vista") });

/**
 * Hereda «oculta» → «visible» de la vista. No define «saliendo»: cuando
 * sale la biblioteca entera se desvanece el contenedor, no cada lámina.
 */
export const varianteElemento: Variants = {
  oculta: (a: InfoEntrada) =>
    a.fija ? { opacity: 1, y: 0 } : a.caer ? { opacity: 0, y: -40 } : { opacity: 0, y: 6 },
  visible: (a: InfoEntrada) => ({
    opacity: 1,
    y: 0,
    transition: a.caer
      ? { ...muelle("caida"), opacity: muelle("pulsacion") }
      : { ...muelle("vista"), delay: a.retraso },
  }),
};
