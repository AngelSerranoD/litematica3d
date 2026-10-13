// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { invoke, type InvokeArgs, type InvokeOptions } from "@tauri-apps/api/core";
import type { ErrorApp } from "./biblioteca";

/** Convierte lo que rechace `invoke` en un ErrorApp (tipo «interno» si no lo es). */
export function normalizarError(e: unknown): ErrorApp {
  if (
    typeof e === "object" &&
    e !== null &&
    "tipo" in e &&
    "mensaje" in e &&
    typeof e.tipo === "string" &&
    typeof e.mensaje === "string"
  ) {
    return e as ErrorApp;
  }
  const mensaje =
    typeof e === "string" ? e : e instanceof Error ? e.message : "Error inesperado.";
  return { tipo: "interno", mensaje };
}

/** `invoke` del núcleo con los errores ya normalizados a ErrorApp. */
export async function llamar<T>(
  orden: string,
  args?: InvokeArgs,
  opciones?: InvokeOptions,
): Promise<T> {
  try {
    return await invoke<T>(orden, args, opciones);
  } catch (e) {
    throw normalizarError(e);
  }
}
