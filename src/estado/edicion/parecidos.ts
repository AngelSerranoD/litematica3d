// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Trozo del almacén de edición: bloque pinchado y su lista de parecidos.

import { destinosConservando, parecidos as pedirParecidos } from "../../api/parecidos";
import { useVisor } from "../visor";
import { mensajeDe, olvidarFamilia, peticiones } from "./peticiones";
import type { EstadoEdicion, Pinchado, Trozo } from "./tipos";

type TrozoParecidos = Pick<
  EstadoEdicion,
  "pinchado" | "lista" | "pinchar" | "abrirLista" | "cerrarLista" | "elegirParecido"
>;

/** Bloques parecidos que se piden por bloque. */
const PARECIDOS_POR_BLOQUE = 16;

export const trozoParecidos: Trozo<TrozoParecidos> = (set, get) => ({
  pinchado: null,
  lista: null,

  pinchar: (celda, bloque) => {
    const pinchado: Pinchado = { celda, bloque };
    // Pinchar otro bloque cierra la familia abierta (sus reemplazos siguen
    // pendientes); Mayús+clic vuelve a seleccionar la del bloque nuevo.
    if (get().familia) olvidarFamilia();
    set({ pinchado, panel: "cambiar", familia: null });
    get().abrirLista({ tipo: "bloque", pinchado });
  },

  abrirLista: (objetivo) => {
    const n = ++peticiones.lista;
    const origen =
      objetivo.tipo === "bloque" ? objetivo.pinchado.bloque.estado : objetivo.estadoOrigen;
    if (objetivo.tipo === "bloque" && objetivo.pinchado.bloque.region === null) {
      set({
        lista: {
          objetivo,
          origen,
          estado: "error",
          parecidos: [],
          mensaje: "Este bloque no está dentro de ninguna región: no se puede cambiar.",
        },
      });
      return;
    }
    set({
      lista: {
        objetivo,
        origen,
        estado: "cargando",
        parecidos: [],
        mensaje: null,
      },
    });
    get().pedirCeldas([origen]);
    // El primer uso construye el catálogo (~0,25 s con jar): se ve «cargando».
    pedirParecidos(origen, PARECIDOS_POR_BLOQUE)
      .then((parecidos) => {
        if (n !== peticiones.lista) return;
        set({
          lista: {
            objetivo,
            origen,
            estado: "lista",
            parecidos,
            mensaje: null,
          },
        });
        get().pedirCeldas(parecidos.map((p) => p.estado));
      })
      .catch((e) => {
        if (n !== peticiones.lista) return;
        set({
          lista: {
            objetivo,
            origen,
            estado: "error",
            parecidos: [],
            mensaje: mensajeDe(e),
          },
        });
      });
  },

  cerrarLista: () => {
    const lista = get().lista;
    peticiones.lista++;
    if (!lista) return;
    if (lista.objetivo.tipo === "bloque") {
      set({ lista: null, pinchado: null });
      useVisor.getState().limpiarSeleccion();
    } else {
      set({ lista: null });
    }
  },

  elegirParecido: (p) => {
    const lista = get().lista;
    if (!lista) return;
    if (lista.objetivo.tipo === "reemplazo") {
      const { estadoOrigen, hermanos = [] } = lista.objetivo;
      get().fijarDestinoReemplazo(estadoOrigen, p.estado);
      // Cada hermano conserva sus propias propiedades (facing, half…): las
      // resuelve Rust con la misma regla que las sugerencias.
      if (hermanos.length > 0) {
        destinosConservando(p.estado, hermanos)
          .then((rs) => {
            for (const r of rs) get().fijarDestinoReemplazo(r.estadoOrigen, r.estadoDestino);
          })
          .catch((e) =>
            set((s) => (s.lista ? { lista: { ...s.lista, mensaje: mensajeDe(e) } } : {})),
          );
      }
      return;
    }
    const { celda, bloque } = lista.objetivo.pinchado;
    if (bloque.region === null) return;
    const [x, y, z] = bloque.posicion;
    get().agregarCambio({
      region: bloque.region,
      x,
      y,
      z,
      nuevoEstado: p.estado,
      celda,
      estadoAnterior: bloque.estado,
    });
  },
});
