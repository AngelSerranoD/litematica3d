// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { domMax } from "motion/react";

/**
 * Paquete de funciones de Motion que carga LazyMotion de forma diferida.
 * Hace falta domMax (y no domAnimation) porque la app usa animaciones de
 * layout: layoutId (elemento compartido) y layout.
 */
export default domMax;
