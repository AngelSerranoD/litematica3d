// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useMemo } from "react";
import { claveVisual, type Malla } from "../../api/visor";
import { celdaIcono, useEdicion } from "../../estado/edicion";
import type { CambioPrevia } from "./previaEdicion";
import type { DestinoPrevia, PreviaMateriales } from "./sombreado";

/**
 * Lo que el visor tiene que dibujar de la edición: la previsualización de
 * los reemplazos y el resaltado de la familia (tabla de materiales), y los
 * cambios sueltos (capa encima). Pide las celdas del atlas (o, sin atlas,
 * los colores planos) de los destinos que falten; mientras no llegan, ese
 * pendiente no se previsualiza.
 */
export function usoPreviaEdicion(malla: Malla, conAtlas: boolean) {
  const reemplazos = useEdicion((s) => s.reemplazos);
  const cambios = useEdicion((s) => s.cambios);
  const familia = useEdicion((s) => s.familia);
  const celdas = useEdicion((s) => s.celdas);
  const versionCeldas = useEdicion((s) => s.versionCeldas);
  const colores = useEdicion((s) => s.colores);
  // Las celdas solo valen si son del atlas con el que se dibuja la malla.
  const version = malla.atlas?.version ?? null;
  const texturas = conAtlas && version !== null && version === versionCeldas;

  useEffect(() => {
    const destinos = [
      ...reemplazos.map((r) => r.estadoDestino),
      ...cambios.map((c) => c.nuevoEstado),
    ];
    if (!destinos.length) return;
    const e = useEdicion.getState();
    if (conAtlas) e.pedirCeldas(destinos);
    else e.pedirColores(destinos);
  }, [reemplazos, cambios, conAtlas]);

  const destinoDe = useMemo(
    () =>
      (estado: string): DestinoPrevia | null => {
        if (conAtlas) {
          const fila = texturas ? celdas[estado] : undefined;
          if (fila === undefined) return null;
          return { caras: fila, color: celdaIcono(fila)?.color ?? null };
        }
        const color = colores[estado];
        return color === undefined ? null : { caras: null, color };
      },
    [conAtlas, texturas, celdas, colores],
  );

  const previa = useMemo<PreviaMateriales | null>(() => {
    const porOrigen = new Map<string, DestinoPrevia>();
    for (const r of reemplazos) {
      const d = destinoDe(r.estadoDestino);
      if (d) porOrigen.set(claveVisual(r.estadoOrigen), d);
    }
    // El contorno acento marca la familia mientras se elige destino; con un destino
    // elegido, lo que se ve es ya la previsualización.
    const resaltados = new Set<string>();
    if (familia?.estado === "lista" && familia.destino === null && familia.pidiendoPlan === null)
      for (const m of familia.miembros) resaltados.add(claveVisual(m.estado));
    if (!porOrigen.size && !resaltados.size) return null;
    return { reemplazos: porOrigen, resaltados };
  }, [reemplazos, familia, destinoDe]);

  const listos = useMemo(
    () =>
      cambios.flatMap((c): CambioPrevia[] => {
        const d = destinoDe(c.nuevoEstado);
        return d
          ? [
              {
                celda: c.celda,
                estadoVisual: claveVisual(c.estadoAnterior),
                estado: c.nuevoEstado,
                caras: d.caras,
                color: d.color,
              },
            ]
          : [];
      }),
    [cambios, destinoDe],
  );
  // `celdas` cambia cada vez que llegan iconos de la lista de parecidos: la
  // capa solo se rehace si cambia algo de lo que dibuja.
  const firma = listos
    .map((c) => `${c.celda.join(",")}|${c.estadoVisual}|${c.estado}|${c.caras ? "t" : "c"}`)
    .join(";");
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const cambiosPrevia = useMemo(() => listos, [firma]);

  return { previa, cambios: cambiosPrevia };
}
