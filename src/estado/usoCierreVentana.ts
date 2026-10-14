// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useBiblioteca } from "./biblioteca";
import { useEdicion } from "./edicion";

/**
 * Al cerrar la ventana:
 * - con cambios de edición sin aplicar, pregunta antes si se descartan (como
 *   la guarda de salida del schematic); si se descartan, la ventana se cierra;
 * - ejecuta los borrados que seguían en cuenta atrás (si no, un cierre rápido
 *   dejaría el schematic sin borrar aunque el usuario ya lo viera
 *   desaparecer). El cierre espera a que terminen.
 */
export function usoCierreVentana(): void {
  useEffect(() => {
    if (!isTauri()) return;
    let activo = true;
    let soltar: (() => void) | undefined;
    const ventana = getCurrentWindow();

    ventana
      .onCloseRequested(async (evento) => {
        const cerrar = async () => {
          await useBiblioteca.getState().vaciarPendientes();
          await ventana.destroy();
        };
        // Sin pendientes devuelve true y el cierre sigue su curso.
        if (!useEdicion.getState().pedirDescartar("cerrarVentana", () => void cerrar())) {
          evento.preventDefault();
          return;
        }
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
