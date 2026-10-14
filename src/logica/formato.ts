// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import type { Dimensiones, Formato } from "../api/biblioteca";

/**
 * «9 × 31 × 9»: ancho × alto × largo. Con `compacta`, sin espacios («9×31×9»),
 * para celdas estrechas.
 */
export function formatoDimensiones({ ancho, alto, largo }: Dimensiones, compacta = false): string {
  const sep = compacta ? "×" : " × ";
  return [ancho, alto, largo].join(sep);
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

/**
 * Número en texto con los mismos grupos que `Cifra`, separados por un espacio
 * fino irrompible: «2 932», «12 345».
 */
export const formatoNumero = (n: number): string => gruposDeMiles(n).join(" ");

/** Distancia OKLab con coma decimal y tres cifras: «0,012». */
export const formatoDistancia = (d: number) => d.toFixed(3).replace(".", ",");

const NOMBRES_FORMATO: Record<Formato, { corto: string; largo: string }> = {
  litematic: { corto: "Litematica", largo: "Litematica (.litematic)" },
  sponge: { corto: "Sponge", largo: "Sponge (.schem)" },
  mcedit: { corto: "MCEdit", largo: "MCEdit (.schematic)" },
  estructura: { corto: "Estructura", largo: "Estructura (.nbt)" },
};

/** Nombre del formato sin la extensión: «Litematica». */
export const formatoCorto = (f: Formato): string => NOMBRES_FORMATO[f].corto;

/** Nombre del formato con su extensión: «Litematica (.litematic)». */
export const nombreFormato = (f: Formato): string => NOMBRES_FORMATO[f].largo;

const dosCifras = (n: number) => String(n).padStart(2, "0");

/** «12/10/2026». */
export function fechaCorta(ms: number): string {
  const d = new Date(ms);
  return `${dosCifras(d.getDate())}/${dosCifras(d.getMonth() + 1)}/${d.getFullYear()}`;
}

const fechaLarga = new Intl.DateTimeFormat("es-ES", { dateStyle: "long", timeStyle: "short" });

/** «12 de octubre de 2026, 14:03». */
export const fechaCompleta = (ms: number): string => fechaLarga.format(ms);
