// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Comandos de la pantalla de materiales: lista de comprobación, nombres de
// los ítems en español y guardado del CSV. El cálculo en sí es `materiales`
// (src/api/biblioteca.ts).

import { invoke } from "@tauri-apps/api/core";
import { normalizarError } from "./biblioteca";

async function llamar<T>(orden: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(orden, args);
  } catch (e) {
    throw normalizarError(e);
  }
}

/** IDs de ítem (`minecraft:…`) marcados como reunidos en este schematic. */
export const comprobacion = (id: string) => llamar<string[]>("comprobacion", { id });

/** Marca o desmarca un ítem como reunido. */
export const marcar = (id: string, item: string, reunido: boolean) =>
  llamar<null>("marcar", { id, item, reunido });

/**
 * Nombres en español de los ítems (`minecraft:stone` → «Piedra»), sacados
 * del jar de Minecraft. Vacío si no hay Minecraft instalado.
 */
export const nombresItems = () => llamar<Record<string, string>>("nombres_items");

/** Escribe texto en un archivo elegido por el usuario (solo `.csv` o `.txt`). */
export const guardarTexto = (ruta: string, contenido: string) =>
  llamar<null>("guardar_texto", { ruta, contenido });
