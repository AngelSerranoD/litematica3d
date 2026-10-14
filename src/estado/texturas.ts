// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";
import { invalidarAtlas } from "../api/atlas";
import { useMateriales } from "./materiales";

/**
 * «Texturas cambiadas»: la versión de Minecraft que se usa de verdad ha
 * cambiado (otra versión o otra carpeta `.minecraft` en los ajustes).
 *
 * `generacion` sube en cada cambio. Quien pinte con las texturas depende de
 * ella: el visor vuelve a pedir la malla y el atlas, la pantalla de
 * materiales vuelve a pedir los nombres de los ítems. Fuera de React, con
 * `alCambiarTexturas` (p. ej. para regenerar miniaturas).
 */
interface EstadoTexturas {
  generacion: number;
}

export const useTexturas = create<EstadoTexturas>()(() => ({ generacion: 0 }));

/**
 * Avisa de que las texturas han cambiado: tira el atlas en memoria y los
 * nombres de los ítems, y sube la generación. La llaman los ajustes cuando
 * `guardar_ajustes` devuelve `true`.
 */
export function texturasCambiadas(): void {
  invalidarAtlas();
  useMateriales.getState().olvidarNombres();
  useTexturas.setState((s) => ({ generacion: s.generacion + 1 }));
}

/**
 * Llama a `oyente` cada vez que cambian las texturas (después de invalidar
 * lo de memoria). Devuelve la función para dejar de escuchar.
 */
export function alCambiarTexturas(oyente: (generacion: number) => void): () => void {
  return useTexturas.subscribe((s, antes) => {
    if (s.generacion !== antes.generacion) oyente(s.generacion);
  });
}
