// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Trozo del almacén de edición: schematic abierto, modo, panel y confirmaciones.

import { useVisor } from "../visor";
import { olvidarFamilia, peticiones } from "./peticiones";
import type { EstadoEdicion, Trozo } from "./tipos";

type TrozoModo = Pick<
  EstadoEdicion,
  | "id"
  | "modo"
  | "panel"
  | "confirmacion"
  | "iniciar"
  | "fijarModo"
  | "alternarModo"
  | "verPanel"
  | "pedirDescartar"
  | "pedirAplicar"
  | "confirmar"
  | "cancelarConfirmacion"
>;

export const trozoModo: Trozo<TrozoModo> = (set, get) => {
  const hayPendientes = () => get().cambios.length > 0 || get().reemplazos.length > 0;

  return {
    id: null,
    modo: "visualizar",
    panel: "cambiar",
    confirmacion: null,

    iniciar: (id) => {
      if (id === get().id) return;
      peticiones.lista++;
      peticiones.historial++;
      olvidarFamilia();
      set({
        id,
        modo: "visualizar",
        panel: "cambiar",
        pinchado: null,
        lista: null,
        cambios: [],
        reemplazos: [],
        familia: null,
        ocupado: null,
        error: null,
        versiones: null,
        cargandoHistorial: false,
        errorHistorial: null,
        confirmacion: null,
      });
    },

    fijarModo: (modo) => {
      const s = get();
      if (modo === s.modo) return;
      if (modo === "modificar") {
        set({ modo, panel: "cambiar", error: null });
        // Lo que ya estuviera seleccionado pasa a ser el bloque pinchado.
        const sel = useVisor.getState().seleccion;
        if (sel?.bloque) get().pinchar(sel.celda, sel.bloque);
        return;
      }
      const salir = () => {
        peticiones.lista++;
        olvidarFamilia();
        set({
          modo: "visualizar",
          pinchado: null,
          lista: null,
          familia: null,
          confirmacion: null,
        });
      };
      if (get().pedirDescartar("salirModo", salir)) salir();
    },

    alternarModo: () => get().fijarModo(get().modo === "visualizar" ? "modificar" : "visualizar"),

    // El historial lo pide su sección al montarse.
    verPanel: (panel) => set({ panel }),

    pedirDescartar: (motivo, continuar) => {
      if (!hayPendientes()) return true;
      set({ confirmacion: { tipo: "descartar", motivo, continuar } });
      return false;
    },

    pedirAplicar: () => {
      if (!hayPendientes() || get().ocupado) return;
      set({ confirmacion: { tipo: "aplicar" } });
    },

    confirmar: () => {
      const c = get().confirmacion;
      set({ confirmacion: null });
      if (!c) return;
      if (c.tipo === "aplicar") {
        void get().aplicar();
      } else {
        get().descartarTodo();
        c.continuar();
      }
    },

    cancelarConfirmacion: () => set({ confirmacion: null }),
  };
};
