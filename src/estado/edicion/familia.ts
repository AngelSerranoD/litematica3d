// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Trozo del almacén de edición: selección por familia y su plan de reemplazos.

import {
  familiaDe,
  familiasParecidas,
  miembrosFamilia,
  planFamilia,
} from "../../api/parecidos";
import { mensajeDe, olvidarFamilia, peticiones } from "./peticiones";
import type { EstadoEdicion, SeleccionFamilia, Trozo } from "./tipos";

type TrozoFamilia = Pick<
  EstadoEdicion,
  "familia" | "fijarFamilia" | "seleccionarFamilia" | "elegirFamiliaDestino" | "cerrarFamilia"
>;

/** Familias destino que se piden. */
const FAMILIAS_DESTINO = 12;

export const trozoFamilia: Trozo<TrozoFamilia> = (set, get) => {
  /** Cambia la selección por familia si sigue siendo la misma `familia`. */
  const fijarSiSigue = (familia: string, cambio: Partial<SeleccionFamilia>) =>
    set((s) =>
      s.familia && s.familia.familia === familia ? { familia: { ...s.familia, ...cambio } } : {},
    );

  return {
    familia: null,

    fijarFamilia: (familia) => {
      olvidarFamilia();
      set({ familia });
    },

    seleccionarFamilia: (estado) => {
      const id = get().id;
      if (!id) return;
      olvidarFamilia();
      const n = peticiones.familia;
      // Una lista de reemplazo abierta era de la familia anterior.
      if (get().lista?.objetivo.tipo === "reemplazo") {
        peticiones.lista++;
        set({ lista: null });
      }
      const base: SeleccionFamilia = {
        familia: "",
        origen: estado,
        estado: "cargando",
        miembros: [],
        destinos: null,
        destino: null,
        pidiendoPlan: null,
        mensaje: null,
      };
      set({ familia: base, panel: "cambiar" });
      const fallo = (mensaje: string) => {
        if (n === peticiones.familia)
          set((s) => (s.familia ? { familia: { ...s.familia, estado: "error", mensaje } } : {}));
      };
      (async () => {
        const fam = await familiaDe(estado);
        if (n !== peticiones.familia) return;
        if (!fam.sugerible) {
          fallo(
            "Este bloque no se puede cambiar en grupo: es aire, un fluido, un bloque técnico o " +
              "un bloque con datos (cofres, hornos, carteles…), y cambiarlo por familia podría " +
              "perder su contenido. Si hace falta, cámbialo suelto con su lista de parecidos.",
          );
          return;
        }
        set((s) => (s.familia ? { familia: { ...s.familia, familia: fam.familia } } : {}));
        const [miembros, destinos] = await Promise.all([
          miembrosFamilia(id, fam.familia),
          familiasParecidas(fam.familia, FAMILIAS_DESTINO),
        ]);
        if (n !== peticiones.familia) return;
        // Si ya había un plan de esta familia pendiente, se recuerda su destino.
        const destino =
          get().reemplazos.find((r) => r.familia === fam.familia && r.familiaDestino)
            ?.familiaDestino ?? null;
        set((s) =>
          s.familia
            ? {
                familia: {
                  ...s.familia,
                  estado: "lista",
                  miembros,
                  destinos,
                  destino,
                  mensaje: destinos.length
                    ? null
                    : "No hay otra familia de material con las mismas formas por la que cambiarla.",
                },
              }
            : {},
        );
        get().pedirCeldas(miembros.map((m) => m.estado));
      })().catch((e) => fallo(mensajeDe(e)));
    },

    elegirFamiliaDestino: (destino) => {
      const { id, familia: f } = get();
      if (!id || !f || f.estado !== "lista") return;
      const origen = f.familia;
      // La que ya estaba elegida: se quita su plan.
      if (f.destino === destino && f.pidiendoPlan === null) {
        peticiones.plan++;
        get().quitarReemplazosDeFamilia(origen);
        fijarSiSigue(origen, { destino: null, mensaje: null });
        return;
      }
      const n = ++peticiones.plan;
      fijarSiSigue(origen, { pidiendoPlan: destino, mensaje: null });
      planFamilia(id, origen, destino)
        .then((plan) => {
          if (n !== peticiones.plan || get().familia?.familia !== origen) return;
          const cantidades = new Map(f.miembros.map((m) => [m.estado, m.cantidad]));
          get().quitarReemplazosDeFamilia(origen);
          get().agregarReemplazos(
            plan.map((r) => ({
              ...r,
              cantidad: cantidades.get(r.estadoOrigen) ?? null,
              familia: origen,
              familiaDestino: destino,
            })),
          );
          fijarSiSigue(origen, {
            destino: plan.length ? destino : null,
            pidiendoPlan: null,
            mensaje: plan.length
              ? null
              : "Con esa familia no cambiaría ningún bloque: no queda nada pendiente.",
          });
          get().pedirCeldas(plan.map((r) => r.estadoDestino));
        })
        .catch((e) => {
          if (n !== peticiones.plan) return;
          fijarSiSigue(origen, {
            pidiendoPlan: null,
            mensaje: `No se pudo preparar el cambio: ${mensajeDe(e)}`,
          });
        });
    },

    cerrarFamilia: () => {
      if (!get().familia) return;
      olvidarFamilia();
      const { lista, pinchado } = get();
      set({ familia: null });
      // Se vuelve a la lista del bloque pinchado (una de reemplazo era de la familia).
      if (lista?.objetivo.tipo === "bloque") return;
      if (pinchado) get().abrirLista({ tipo: "bloque", pinchado });
      else if (lista) {
        peticiones.lista++;
        set({ lista: null });
      }
    },
  };
};
