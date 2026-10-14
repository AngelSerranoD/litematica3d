// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { open } from "@tauri-apps/plugin-dialog";

/**
 * Extensiones que admite el núcleo (sin punto). `.mcstructure` no está
 * soportado. Espejo de `EXTENSIONES` en `src-tauri/src/argumentos.rs` y de
 * `fileAssociations` en `tauri.conf.json`: si se cambia una, cambian las tres
 * (lo comprueba `src-tauri/src/espejos.rs`).
 */
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
