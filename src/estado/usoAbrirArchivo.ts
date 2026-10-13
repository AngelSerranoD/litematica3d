// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useBiblioteca } from "./biblioteca";
import { useNavegacion } from "./navegacion";

/** Importa (o localiza, si ya estaba) el archivo y abre su vista. */
async function abrirArchivo(ruta: string): Promise<void> {
  const [resultado] = await useBiblioteca.getState().importar([ruta]);
  if (resultado && resultado.estado !== "error") {
    useNavegacion.getState().abrir(resultado.entrada.id);
  }
}

/**
 * Abrir un schematic con doble clic en el Explorador: el que trajo el
 * arranque (comando `archivo_pendiente`) y los que llegan después con la app
 * ya abierta (evento `abrir-archivo`). Se escucha primero y se recoge después
 * para no perder ninguno.
 */
export function usoAbrirArchivo(): void {
  useEffect(() => {
    if (!isTauri()) return;
    let activo = true;
    let quitar: (() => void) | undefined;

    listen<string>("abrir-archivo", (e) => void abrirArchivo(e.payload))
      .then(async (fn) => {
        if (!activo) return fn();
        quitar = fn;
        const pendiente = await invoke<string | null>("archivo_pendiente");
        if (activo && pendiente) await abrirArchivo(pendiente);
      })
      .catch(() => {});

    return () => {
      activo = false;
      quitar?.();
    };
  }, []);
}
