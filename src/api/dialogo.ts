// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { open } from "@tauri-apps/plugin-dialog";

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
