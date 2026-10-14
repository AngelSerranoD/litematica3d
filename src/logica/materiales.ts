// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Cuentas de la pantalla de materiales sin React: nombres, filtro y orden,
// conversión de capas y exportación (texto y CSV).

import { type Material } from "../api/biblioteca";
import { formatoNumero } from "./formato";
import { normalizarTexto } from "../estado/biblioteca";

/** ID con el que el núcleo agrupa los bloques antiguos sin traducción. */
const ID_DESCONOCIDO = "litematica3d:desconocido";

/** «minecraft:oak_planks» → «Oak Planks» (cuando no hay nombres del juego). */
function humanizarId(id: string): string {
  const sinEspacio = id.includes(":") ? id.slice(id.indexOf(":") + 1) : id;
  const texto = sinEspacio.replace(/[_/]+/g, " ").trim();
  // Title Case, como los nombres del juego.
  return texto ? texto.replace(/(^|\s)(\S)/g, (_m, sp: string, c: string) => sp + c.toUpperCase()) : id;
}

/** Nombre en inglés si el juego lo trae; si no, el ID humanizado. */
export function nombreItem(id: string, nombres: Record<string, string> | null): string {
  if (id === ID_DESCONOCIDO) return "Bloques desconocidos";
  return nombres?.[id] ?? humanizarId(id);
}

/**
 * Capas del visor (coordenadas del modelo, inclusivas) → `opciones.capas` de
 * `materiales`. El núcleo usa las mismas coordenadas que `malla` y suma él el
 * origen Y, así que van tal cual. ÚNICO sitio donde se traduce el rango.
 */
export function capasAMateriales(desde: number, hasta: number): [number, number] {
  return [desde, hasta];
}

/** Y del schematic de una capa del modelo (para rotular, como el deslizador). */
export const yDeCapa = (capa: number, origenY: number): number => origenY + capa;

export interface Fila extends Material {
  nombre: string;
  /** Nombre e ID normalizados, para buscar. */
  clave: string;
}

export type CampoOrdenMateriales = "cantidad" | "nombre";

export interface OrdenMateriales {
  campo: CampoOrdenMateriales;
  ascendente: boolean;
}

export function aFilas(materiales: Material[], nombres: Record<string, string> | null): Fila[] {
  return materiales.map((m) => {
    const nombre = nombreItem(m.item, nombres);
    return { ...m, nombre, clave: normalizarTexto(`${nombre} ${m.item}`) };
  });
}

const cotejo = new Intl.Collator("es", { sensitivity: "base", numeric: true });

/** Filtra por texto (nombre o ID) y por reunidos, y ordena. No modifica `filas`. */
export function filtrarYOrdenar(
  filas: Fila[],
  busqueda: string,
  orden: OrdenMateriales,
  ocultarReunidos: boolean,
  reunidos: ReadonlySet<string>,
): Fila[] {
  const terminos = normalizarTexto(busqueda).split(/\s+/).filter(Boolean);
  const visibles = filas.filter(
    (f) =>
      (!ocultarReunidos || !reunidos.has(f.item)) && terminos.every((t) => f.clave.includes(t)),
  );
  const signo = orden.ascendente ? 1 : -1;
  return visibles.sort((a, b) => {
    const porNombre = cotejo.compare(a.nombre, b.nombre);
    if (orden.campo === "nombre") return signo * porNombre || a.item.localeCompare(b.item);
    return signo * (a.cantidad - b.cantidad) || porNombre;
  });
}

/** «19 stacks + 18», «1 stack», «5» (ítems que no se apilan van sueltos). */
function desglose(f: Material): string {
  const partes: string[] = [];
  if (f.stacks > 0)
    partes.push(`${formatoNumero(f.stacks)} ${f.stacks === 1 ? "stack" : "stacks"}`);
  if (f.sueltos > 0 || partes.length === 0) partes.push(formatoNumero(f.sueltos));
  let texto = partes.join(" + ");
  if (f.shulkers > 0)
    texto += ` · ${formatoNumero(f.shulkers)} shulker${f.shulkers === 1 ? "" : "s"}`;
  return texto;
}

interface Resumen {
  titulo: string;
  /** «Región: todas · Capas Y 0–31». */
  filtro: string;
  totalItems: number;
  totalStacks: number;
  totalShulkers: number;
}

/** Lista legible para pegar en un chat: una línea por ítem, con casilla. */
export function textoLista(filas: Fila[], reunidos: ReadonlySet<string>, r: Resumen): string {
  const ancho = Math.max(0, ...filas.map((f) => formatoNumero(f.cantidad).length));
  const lineas = filas.map(
    (f) =>
      `[${reunidos.has(f.item) ? "x" : " "}] ${formatoNumero(f.cantidad).padStart(ancho)}  ${f.nombre}  (${desglose(f)})`,
  );
  return [
    `Materiales: ${r.titulo}`,
    r.filtro,
    "",
    ...lineas,
    "",
    `Total: ${formatoNumero(r.totalItems)} bloques · ${formatoNumero(r.totalStacks)} stacks · ${formatoNumero(r.totalShulkers)} shulkers`,
  ].join("\n");
}

/**
 * Campo CSV: entre comillas si lleva separador, comillas o saltos de línea.
 * Los textos que empiezan por = + - @ tabulador o retorno llevan un apóstrofo
 * delante: los ids vienen del schematic (archivo ajeno) y Excel los ejecutaría
 * como fórmulas.
 */
function campoCsv(valor: string | number): string {
  let texto = String(valor);
  if (typeof valor === "string" && /^[=+\-@\t\r]/.test(texto)) texto = `'${texto}`;
  return /[;"\r\n]/.test(texto) ? `"${texto.replace(/"/g, '""')}"` : texto;
}

/**
 * CSV para Excel en español: separador «;», fin de línea CRLF y BOM UTF-8
 * (sin él, Excel abre el archivo como ANSI y rompe las tildes).
 */
export function csvLista(filas: Fila[], reunidos: ReadonlySet<string>): string {
  const cabecera = ["nombre", "id", "cantidad", "stacks", "sueltos", "shulkers", "reunido"];
  const lineas = filas.map((f) =>
    [
      f.nombre,
      f.item,
      f.cantidad,
      f.stacks,
      f.sueltos,
      f.shulkers,
      reunidos.has(f.item) ? "sí" : "no",
    ]
      .map(campoCsv)
      .join(";"),
  );
  return `﻿${[cabecera.join(";"), ...lineas].join("\r\n")}\r\n`;
}

/** Nombre de archivo propuesto: sin caracteres que Windows no admite. */
export function nombreArchivo(titulo: string): string {
  const limpio = titulo
    // Los caracteres de control también están prohibidos en nombres de Windows.
    // oxlint-disable-next-line no-control-regex
    .replace(/[<>:"/\\|?*\u0000-\u001f]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return `${limpio || "schematic"} - materiales.csv`;
}
