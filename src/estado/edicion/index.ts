// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";
import { alCambiarTexturas } from "../texturas";
import { useVisor } from "../visor";
import { olvidarColaCeldas, trozoCeldas } from "./celdas";
import { trozoFamilia } from "./familia";
import { trozoHistorial } from "./historial";
import { trozoModo } from "./modo";
import { trozoParecidos } from "./parecidos";
import { trozoPendientes } from "./pendientes";
import { peticiones } from "./peticiones";
import type { EstadoEdicion } from "./tipos";

/**
 * Edición de bloques en el visor (docs/edicion.md, tarea 10).
 *
 * El visor tiene dos modos: **Visualizar** (lo de siempre) y **Modificar**.
 * En Modificar, pinchar un bloque abre la lista de los más parecidos; lo que
 * se elige queda **pendiente** (cambios sueltos por posición y reemplazos de
 * un estado por otro en todo el schematic) hasta pulsar **Aplicar**, que lo
 * escribe en la copia de la biblioteca y deja lo anterior en el historial.
 *
 * El almacén se arma con trozos, uno por archivo de esta carpeta: `modo`
 * (schematic abierto, modo, panel y confirmaciones), `parecidos` (bloque
 * pinchado y su lista), `pendientes`, `familia`, `historial` (aplicar,
 * restaurar, exportar) y `celdas` (iconos y colores de los block states).
 *
 * Como en `visor.ts`, ningún selector debe construir arrays u objetos nuevos:
 * las listas solo cambian de referencia cuando cambia su contenido.
 */

export type {
  CambioPendiente,
  Confirmacion,
  ListaParecidos,
  ModoVisor,
  MotivoDescarte,
  PanelEdicion,
  ReemplazoPendiente,
  SeleccionFamilia,
} from "./tipos";
export { bloquesAfectados } from "../edicionPura";
export { celdaIcono } from "./celdas";

export const useEdicion = create<EstadoEdicion>()((...a) => ({
  ...trozoModo(...a),
  ...trozoParecidos(...a),
  ...trozoPendientes(...a),
  ...trozoFamilia(...a),
  ...trozoHistorial(...a),
  ...trozoCeldas(...a),
}));

// En Modificar, cada bloque que se pincha en el visor abre su lista de
// parecidos; al quitar la selección (clic en el vacío, Esc) se cierra.
const desuscribir = useVisor.subscribe((s, antes) => {
  if (s.seleccion === antes.seleccion) return;
  const e = useEdicion.getState();
  if (e.modo !== "modificar") return;
  const sel = s.seleccion;
  if (!sel) {
    if (e.lista?.objetivo.tipo === "bloque" || e.pinchado) {
      peticiones.lista++;
      useEdicion.setState({
        pinchado: null,
        lista: e.lista?.objetivo.tipo === "bloque" ? null : e.lista,
      });
    }
    return;
  }
  // `bloque` llega después (respuesta de `bloque_en`): hasta entonces nada.
  if (sel.bloque && sel.bloque !== antes.seleccion?.bloque) e.pinchar(sel.celda, sel.bloque);
});
// Con otras texturas las celdas de antes no valen: se vuelven a pedir.
const desuscribirTexturas = alCambiarTexturas(() => {
  olvidarColaCeldas();
  useEdicion.setState({ celdas: {}, versionCeldas: null });
});
// Con la recarga en caliente del desarrollo, el módulo viejo deja de escuchar.
import.meta.hot?.dispose(() => {
  desuscribir();
  desuscribirTexturas();
});

/** Número de pendientes, cambios más reemplazos (selector atómico). */
export const numPendientes = (s: EstadoEdicion) => s.cambios.length + s.reemplazos.length;
