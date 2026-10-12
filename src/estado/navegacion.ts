// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
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

export const useNavegacion = create<EstadoNavegacion>()((set, get) => ({
  ruta: { vista: "biblioteca" },
  direccion: 1,
  ultimoId: null,
  abrir: (id) => set({ ruta: { vista: "schematic", id }, direccion: 1, ultimoId: id }),
  volver: () => {
    if (get().ruta.vista === "biblioteca") return;
    set({ ruta: { vista: "biblioteca" }, direccion: -1 });
  },
}));
