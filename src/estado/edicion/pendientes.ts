// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Trozo del almacén de edición: cambios sueltos y reemplazos pendientes de aplicar.

import { olvidarFamilia } from "./peticiones";
import type { EstadoEdicion, ReemplazoPendiente, SeleccionFamilia, Trozo } from "./tipos";

type TrozoPendientes = Pick<
  EstadoEdicion,
  | "cambios"
  | "reemplazos"
  | "agregarCambio"
  | "quitarCambio"
  | "agregarReemplazos"
  | "fijarDestinoReemplazo"
  | "quitarReemplazo"
  | "quitarReemplazosDeFamilia"
  | "descartarTodo"
>;

const claveCambio = (region: string, x: number, y: number, z: number) => `${region}|${x},${y},${z}`;

export const trozoPendientes: Trozo<TrozoPendientes> = (set) => ({
  cambios: [],
  reemplazos: [],

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

  descartarTodo: () => {
    olvidarFamilia();
    set({ cambios: [], reemplazos: [], familia: null, error: null });
  },
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
