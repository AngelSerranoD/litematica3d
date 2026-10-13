// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import type { Variants } from "motion/react";
import { muelle } from "../estilos/movimiento";

/**
 * Transición entre vistas.
 *
 * El contenedor de la vista NO se desvanece al entrar: si lo hiciera, el
 * elemento compartido (layoutId) aparecería transparente a mitad de vuelo.
 * Entran por separado las piezas (cabecera, ficha, láminas); la vista que
 * sale sí se desvanece entera. `custom` es la dirección: 1 al entrar en un
 * schematic, -1 al volver a la biblioteca.
 */
export const varianteVista: Variants = {
  oculta: { opacity: 1 },
  // opacity: 1 explícito: si se vuelve a una vista que aún estaba saliendo,
  // AnimatePresence la reutiliza y tiene que recuperar la opacidad.
  visible: { opacity: 1, transition: { staggerChildren: 0.035 } },
  // Funciones y no objetos: muelle() lee tokens.css al pintar, no al importar.
  saliendo: () => ({ opacity: 0, transition: muelle("vista") }),
};

export const variantePieza: Variants = {
  oculta: (direccion: 1 | -1) => ({ opacity: 0, x: 12 * direccion }),
  visible: () => ({ opacity: 1, x: 0, transition: muelle("vista") }),
};
