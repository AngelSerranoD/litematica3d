// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Contadores de peticiones en vuelo, compartidos por los trozos del almacén de edición.

import { normalizarError } from "../../api/llamar";

/**
 * Cada contador sube cuando lo pedido antes deja de valer: una respuesta solo
 * se usa si el número que se guardó al pedirla sigue siendo el último.
 */
export const peticiones = {
  /** Sube con cada lista abierta: las respuestas de antes ya no valen. */
  lista: 0,
  /** Sube con cada historial pedido o schematic nuevo. */
  historial: 0,
  /** Sube con cada familia seleccionada o cerrada. */
  familia: 0,
  /** Sube con cada familia destino elegida. */
  plan: 0,
};

/** Las respuestas de familia y plan en vuelo ya no valen. */
export function olvidarFamilia() {
  peticiones.familia++;
  peticiones.plan++;
}

export const mensajeDe = (e: unknown) => normalizarError(e).mensaje;
