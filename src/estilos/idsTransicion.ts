// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

/** layoutId del elemento compartido miniatura (biblioteca) ↔ visor (schematic). */
export const idVisor = (id: string) => `visor-${id}`;
/** layoutId del dibujo dentro del visor, para que no se deforme en el vuelo. */
export const idBloque = (id: string) => `bloque-${id}`;
