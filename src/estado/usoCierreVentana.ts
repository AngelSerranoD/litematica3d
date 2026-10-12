// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useBiblioteca } from "./biblioteca";

/**
 * Al cerrar la ventana, ejecuta los borrados que seguían en cuenta atrás
 * (si no, un cierre rápido dejaría el schematic sin borrar aunque el usuario
 * ya lo viera desaparecer). El cierre espera a que terminen.
 */
export function usoCierreVentana(): void {
  useEffect(() => {
    if (!isTauri()) return;
    let activo = true;
    let soltar: (() => void) | undefined;

    getCurrentWindow()
      .onCloseRequested(async () => {
        await useBiblioteca.getState().vaciarPendientes();
      })
      .then((fn) => {
        if (activo) soltar = fn;
        else fn();
      })
      .catch(() => {});

    return () => {
      activo = false;
      soltar?.();
    };
  }, []);
}
