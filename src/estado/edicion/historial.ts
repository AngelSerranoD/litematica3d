// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Trozo del almacén de edición: aplicar, restaurar, historial de versiones y exportar.

import {
  aplicarCambios,
  exportarSchematic,
  historial,
  restaurarVersion,
  type Edicion,
} from "../../api/edicion";
import { useBiblioteca } from "../biblioteca";
import { inversaDe } from "../edicionPura";
import { useVisor } from "../visor";
import { mensajeDe, olvidarFamilia, peticiones } from "./peticiones";
import type { EstadoEdicion, Trozo } from "./tipos";

type TrozoHistorial = Pick<
  EstadoEdicion,
  | "ocupado"
  | "error"
  | "versiones"
  | "cargandoHistorial"
  | "errorHistorial"
  | "revision"
  | "aplicar"
  | "restaurar"
  | "cargarHistorial"
  | "exportar"
>;

/** Cuánto se ve el aviso con Deshacer tras aplicar o restaurar. */
const DURACION_AVISO_EDICION_MS = 10000;

export const trozoHistorial: Trozo<TrozoHistorial> = (set, get) => {
  /** Lo que hay que hacer tras aplicar o restaurar con éxito. */
  const despues = (ed: Edicion) => {
    useBiblioteca.getState().actualizarEntrada(ed.entrada);
    if (ed.entrada.id !== get().id) return; // era de otro schematic (aviso tardío)
    peticiones.lista++;
    olvidarFamilia();
    // `version: null` = no cambió nada en disco: no hace falta remallar.
    const cambio = ed.version !== null;
    set((s) => ({
      revision: cambio ? s.revision + 1 : s.revision,
      pinchado: null,
      lista: null,
      cambios: [],
      reemplazos: [],
      familia: null,
      error: null,
    }));
    // La celda seleccionada puede tener ya otro bloque.
    useVisor.getState().limpiarSeleccion();
    if (cambio && get().versiones !== null) void get().cargarHistorial();
  };

  return {
    ocupado: null,
    error: null,
    versiones: null,
    cargandoHistorial: false,
    errorHistorial: null,
    revision: 0,

    aplicar: async () => {
      const { id, cambios, reemplazos, ocupado } = get();
      if (!id || ocupado || (!cambios.length && !reemplazos.length)) return;
      set({ ocupado: "aplicar", error: null });
      try {
        const ed = await aplicarCambios(
          id,
          cambios.map(({ region, x, y, z, nuevoEstado }) => ({
            region,
            x,
            y,
            z,
            nuevoEstado,
          })),
          reemplazos.map(({ estadoOrigen, estadoDestino }) => ({
            estadoOrigen,
            estadoDestino,
          })),
        );
        despues(ed);
        const avisar = useBiblioteca.getState().avisar;
        const version = ed.version;
        if (version === null) {
          avisar("No había nada que cambiar: el schematic ya tenía esos bloques.");
        } else {
          avisar(ed.cambios === 1 ? "Cambiado 1 bloque." : `Cambiados ${ed.cambios} bloques.`, {
            accion: {
              etiqueta: "Deshacer",
              ejecutar: () => void get().restaurar(version, id),
            },
            duracionMs: DURACION_AVISO_EDICION_MS,
          });
        }
      } catch (e) {
        if (get().id === id) set({ error: `No se pudo aplicar: ${mensajeDe(e)}` });
        else useBiblioteca.getState().avisar(`No se pudo aplicar: ${mensajeDe(e)}`);
      } finally {
        if (get().id === id) set({ ocupado: null });
      }
    },

    restaurar: async (version, idPedido, hecha = "Deshacer") => {
      const id = idPedido ?? get().id;
      if (!id) return;
      const actual = get().id === id;
      if (actual && get().ocupado) return;
      // Los pendientes son de la versión de ahora: restaurar los invalida.
      if (actual && !get().pedirDescartar("restaurar", () => void get().restaurar(version, id, hecha)))
        return;
      if (actual) set({ ocupado: "restaurar", error: null });
      try {
        const ed = await restaurarVersion(id, version);
        despues(ed);
        const rehacer = ed.version;
        useBiblioteca.getState().avisar(
          "Versión restaurada.",
          rehacer === null
            ? undefined
            : {
                accion: {
                  etiqueta: inversaDe(hecha),
                  ejecutar: () => void get().restaurar(rehacer, id, inversaDe(hecha)),
                },
                duracionMs: DURACION_AVISO_EDICION_MS,
              },
        );
      } catch (e) {
        const mensaje = `No se pudo restaurar: ${mensajeDe(e)}`;
        if (get().id === id) set({ error: mensaje });
        else useBiblioteca.getState().avisar(mensaje);
      } finally {
        if (get().id === id && get().ocupado === "restaurar") set({ ocupado: null });
      }
    },

    cargarHistorial: async () => {
      const id = get().id;
      if (!id) return;
      const n = ++peticiones.historial;
      set({ cargandoHistorial: true, errorHistorial: null });
      try {
        const versiones = await historial(id);
        if (n === peticiones.historial) set({ versiones, cargandoHistorial: false });
      } catch (e) {
        if (n === peticiones.historial)
          set({ cargandoHistorial: false, errorHistorial: mensajeDe(e) });
      }
    },

    exportar: async (nombre) => {
      const id = get().id;
      if (!id || get().ocupado) return;
      set({ ocupado: "exportar", error: null });
      try {
        const ruta = await exportarSchematic(id, nombre);
        if (!ruta) return;
        useBiblioteca.getState().avisar(`Exportado a ${ruta}`);
      } catch (e) {
        const mensaje = `No se pudo exportar: ${mensajeDe(e)}`;
        if (get().id === id) set({ error: mensaje });
        else useBiblioteca.getState().avisar(mensaje);
      } finally {
        if (get().id === id && get().ocupado === "exportar") set({ ocupado: null });
      }
    },
  };
};
