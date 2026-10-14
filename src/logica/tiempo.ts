// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

/** Espera `ms` milisegundos (con `0` cede el hilo a la interfaz). */
export const dormir = (ms: number) =>
  new Promise<void>((r) => window.setTimeout(r, ms));
