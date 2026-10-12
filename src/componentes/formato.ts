// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import type { Dimensiones, Formato } from "../api/biblioteca";

/** «32×18×40»: ancho × alto × largo, sin espacios (para celdas estrechas). */
export function dimensionesCompactas({ ancho, alto, largo }: Dimensiones): string {
  return `${ancho}×${alto}×${largo}`;
}

/** «32 por 18 por 40», para lectores de pantalla. */
export function dimensionesHabladas({ ancho, alto, largo }: Dimensiones): string {
  return `${ancho} por ${alto} por ${largo}`;
}

/**
 * Grupos de tres cifras: 1234567 → ["1", "234", "567"]. Se agrupan también
 * los números de cuatro cifras, para que las columnas queden alineadas.
 */
export function gruposDeMiles(n: number): string[] {
  const signo = n < 0 ? "−" : "";
  const cifras = String(Math.trunc(Math.abs(n)));
  const grupos: string[] = [];
  for (let fin = cifras.length; fin > 0; fin -= 3) {
    grupos.unshift(cifras.slice(Math.max(0, fin - 3), fin));
  }
  grupos[0] = signo + grupos[0];
  return grupos;
}

const NOMBRE_CORTO: Record<Formato, string> = {
  litematic: "Litematica",
  sponge: "Sponge",
  mcedit: "MCEdit",
  estructura: "Estructura",
};

/** Nombre del formato sin la extensión: «Litematica». */
export const formatoCorto = (f: Formato): string => NOMBRE_CORTO[f];

const dosCifras = (n: number) => String(n).padStart(2, "0");

/** «12/10/2026». */
export function fechaCorta(ms: number): string {
  const d = new Date(ms);
  return `${dosCifras(d.getDate())}/${dosCifras(d.getMonth() + 1)}/${d.getFullYear()}`;
}

const fechaLarga = new Intl.DateTimeFormat("es-ES", { dateStyle: "long", timeStyle: "short" });

/** «12 de octubre de 2026, 14:03». */
export const fechaCompleta = (ms: number): string => fechaLarga.format(ms);
