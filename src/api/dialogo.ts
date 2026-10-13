// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { open, save } from "@tauri-apps/plugin-dialog";

/** Extensiones que admite el núcleo (sin punto). `.mcstructure` no está soportado. */
export const EXTENSIONES_ADMITIDAS = ["litematic", "schem", "schematic", "nbt"];

/** Abre el selector de archivos del sistema (multiselección). Devuelve `[]` si se cancela. */
export async function elegirArchivos(): Promise<string[]> {
  const elegidos = await open({
    multiple: true,
    directory: false,
    title: "Importar schematics",
    filters: [{ name: "Schematics de Minecraft", extensions: EXTENSIONES_ADMITIDAS }],
  });
  if (!elegidos) return [];
  return Array.isArray(elegidos) ? elegidos : [elegidos];
}

/**
 * Pide dónde guardar la lista de materiales en CSV. Devuelve `null` si se
 * cancela. Se asegura de que la ruta acabe en `.csv` (`guardar_texto` solo
 * acepta `.csv` y `.txt`).
 */
export async function elegirDestinoCsv(nombrePropuesto: string): Promise<string | null> {
  const ruta = await save({
    title: "Guardar lista de materiales",
    defaultPath: nombrePropuesto,
    filters: [{ name: "CSV (Excel, separado por punto y coma)", extensions: ["csv"] }],
  });
  if (!ruta) return null;
  return /\.csv$/i.test(ruta) ? ruta : `${ruta}.csv`;
}

/** Pide la carpeta `.minecraft`. Devuelve `null` si se cancela. */
export async function elegirCarpetaMinecraft(desde?: string): Promise<string | null> {
  const ruta = await open({
    directory: true,
    multiple: false,
    title: "Carpeta de Minecraft (.minecraft)",
    defaultPath: desde,
  });
  return typeof ruta === "string" ? ruta : null;
}

/**
 * Pide dónde exportar un schematic. Devuelve `null` si se cancela. Se asegura
 * de que la ruta acabe en `.litematic` (`exportar_schematic` solo acepta esa
 * extensión).
 */
export async function elegirDestinoLitematic(nombrePropuesto: string): Promise<string | null> {
  const ruta = await save({
    title: "Exportar schematic",
    defaultPath: /\.litematic$/i.test(nombrePropuesto)
      ? nombrePropuesto
      : `${nombrePropuesto}.litematic`,
    filters: [{ name: "Litematica (.litematic)", extensions: ["litematic"] }],
  });
  if (!ruta) return null;
  return /\.litematic$/i.test(ruta) ? ruta : `${ruta}.litematic`;
}
