// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import type { Transition } from "motion/react";

/**
 * Muelles para Motion.
 *
 * Los números NO se repiten aquí: viven en src/estilos/tokens.css como
 * variables (--muelle-<nombre>-rigidez, -amortiguacion, -masa) y este módulo
 * los lee del :root la primera vez que se piden. Así la documentación de los
 * tokens y lo que anima Motion son la misma fuente.
 *
 * Se leen en el momento de pintar (no al importar el módulo) porque en
 * desarrollo Vite inyecta el CSS al importar main.tsx; a esas alturas ya está.
 */
type NombreMuelle = "vista" | "compartido" | "pulsacion" | "caida";

const cache = new Map<NombreMuelle, Transition>();

function leerNumero(estilos: CSSStyleDeclaration, variable: string): number {
  const valor = Number.parseFloat(estilos.getPropertyValue(variable));
  if (Number.isNaN(valor)) {
    throw new Error(`Falta el token de movimiento ${variable} en tokens.css`);
  }
  return valor;
}

/** Devuelve la transición de muelle definida en tokens.css para `nombre`. */
export function muelle(nombre: NombreMuelle): Transition {
  const guardado = cache.get(nombre);
  if (guardado) return guardado;
  const estilos = getComputedStyle(document.documentElement);
  const transicion: Transition = {
    type: "spring",
    stiffness: leerNumero(estilos, `--muelle-${nombre}-rigidez`),
    damping: leerNumero(estilos, `--muelle-${nombre}-amortiguacion`),
    mass: leerNumero(estilos, `--muelle-${nombre}-masa`),
  };
  cache.set(nombre, transicion);
  return transicion;
}
