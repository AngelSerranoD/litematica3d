// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";

/** Vista activa. Sin react-router: la app tiene dos pantallas y no hay URL. */
export type Ruta = { vista: "biblioteca" } | { vista: "schematic"; id: string };

interface EstadoNavegacion {
  ruta: Ruta;
  /** 1 al entrar en un schematic, -1 al volver: orienta la transición. */
  direccion: 1 | -1;
  /** Último schematic abierto, para devolverle el foco al volver. */
  ultimoId: string | null;
  abrir: (id: string) => void;
  volver: () => void;
}

/**
 * Guarda de salida del schematic abierto: recibe cómo seguir y devuelve
 * `true` si se puede salir ya. Si devuelve `false`, se queda con `continuar`
 * para llamarlo más tarde (p. ej. tras preguntar si se descartan los cambios
 * pendientes de la edición).
 */
type GuardaSalida = (continuar: () => void) => boolean;

let guarda: GuardaSalida | null = null;

/** Pone la guarda de salida; devuelve la función que la quita. */
export function fijarGuardaSalida(g: GuardaSalida): () => void {
  guarda = g;
  return () => {
    if (guarda === g) guarda = null;
  };
}

/** Sale ya o, si la guarda lo para, cuando ella llame a `continuar`. */
function salir(continuar: () => void) {
  if (guarda && !guarda(continuar)) return;
  continuar();
}

export const useNavegacion = create<EstadoNavegacion>()((set, get) => ({
  ruta: { vista: "biblioteca" },
  direccion: 1,
  ultimoId: null,
  abrir: (id) => {
    const ruta = get().ruta;
    const seguir = () => set({ ruta: { vista: "schematic", id }, direccion: 1, ultimoId: id });
    if (ruta.vista === "schematic" && ruta.id !== id) salir(seguir);
    else seguir();
  },
  volver: () => {
    if (get().ruta.vista === "biblioteca") return;
    salir(() => {
      if (get().ruta.vista !== "biblioteca") set({ ruta: { vista: "biblioteca" }, direccion: -1 });
    });
  },
}));
